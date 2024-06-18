use super::marker::Marker;
use super::trace::Trace;
use super::trace_job::TraceJob;
use super::tracer::TraceWorker;
use crate::config::GcConfig;
use crossbeam_channel::{Receiver, Sender};
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc, Mutex, MutexGuard, RwLock, RwLockReadGuard,
};
use std::time::Instant;

pub struct TracerController<M: Marker> {
    sender: Sender<Vec<TraceJob<M>>>,
    receiver: Receiver<Vec<TraceJob<M>>>,

    yield_flag: AtomicBool,
    yield_lock: RwLock<()>,
    trace_end_flag: AtomicBool,
    trace_lock: RwLock<()>,
    // TODO: store in GcArray instead of vec?
    // this will be tricky since then tracing will require
    // access to a mutator, or at least the arena in some way
    // or should this be some kind of channel?
    tracers_waiting: AtomicUsize,
    work_sent: AtomicUsize,
    work_received: AtomicUsize,
    write_barrier_lock: Mutex<()>,

    // config vars
    pub num_tracers: usize,
    pub trace_share_min: usize,
    pub trace_chunk_size: usize,
    pub trace_share_ratio: f32,
    pub trace_wait_time: u64,
    pub mutator_share_min: usize,
}

impl<M: Marker> TracerController<M> {
    pub fn new(config: &GcConfig) -> Self {
        let (sender, receiver) = crossbeam_channel::unbounded();

        Self {
            sender,
            receiver,

            yield_flag: AtomicBool::new(false),
            yield_lock: RwLock::new(()),
            trace_end_flag: AtomicBool::new(false),
            trace_lock: RwLock::new(()),
            tracers_waiting: AtomicUsize::new(0),
            work_sent: AtomicUsize::new(0),
            work_received: AtomicUsize::new(0),
            write_barrier_lock: Mutex::new(()),

            num_tracers: config.tracer_threads,
            trace_share_min: config.trace_share_min,
            trace_chunk_size: config.trace_chunk_size,
            trace_share_ratio: config.trace_share_ratio,
            trace_wait_time: config.trace_wait_time,
            mutator_share_min: config.mutator_share_min,
        }
    }

    pub fn yield_flag(&self) -> bool {
        self.yield_flag.load(Ordering::SeqCst)
    }

    pub fn raise_yield_flag(&self) {
        self.yield_flag.store(true, Ordering::SeqCst);
    }

    pub fn yield_lock(&self) -> RwLockReadGuard<()> {
        self.yield_lock.read().unwrap()
    }

    pub fn get_write_barrier_lock(&self) -> MutexGuard<()> {
        self.write_barrier_lock.lock().unwrap()
    }

    pub fn is_write_barrier_locked(&self) -> bool {
        self.write_barrier_lock.try_lock().is_ok()
    }

    pub fn has_work(&self) -> bool {
        !self.receiver.is_empty()
    }

    pub fn incr_recv(&self) {
        self.work_received.fetch_add(1, Ordering::SeqCst);
    }

    pub fn send_work(&self, work: Vec<TraceJob<M>>) {
        self.work_sent.fetch_add(1, Ordering::SeqCst);
        self.sender.send(work).unwrap();
    }

    pub fn recv_work(&self) -> Vec<TraceJob<M>> {
        self.start_waiting();

        let duration = std::time::Duration::from_millis(self.trace_wait_time);
        let deadline = Instant::now().checked_add(duration).unwrap();

        loop {
            match self.receiver.recv_deadline(deadline) {
                Ok(work) => {
                    self.stop_waiting();
                    self.incr_recv();
                    return work;
                }
                Err(_) => {
                    if self.is_trace_completed() {
                        self.stop_waiting();
                        return vec![];
                    }
                }
            }
        }
    }

    pub fn is_trace_completed(&self) -> bool {
        if self.trace_end_flag.load(Ordering::SeqCst) {
            return true;
        }

        if self.tracers_waiting() == self.num_tracers && self.sent() == self.received() {
            if self.mutators_stopped() {
                // Let the other tracers no they should stop by raising this flag
                self.trace_end_flag.store(true, Ordering::SeqCst);
                return true;
            } else {
                // The tracers are out of work but the mutators are still running
                // Raise the yield flag to request the mutators to stop, so tracing
                // can complete.
                self.yield_flag.store(true, Ordering::SeqCst);
            }
        }

        return false;
    }

    pub fn wait_for_trace_completion(&self) {
        drop(self.trace_lock.write().unwrap());

        debug_assert_eq!(self.sent(), self.received());
        debug_assert_eq!(self.sender.len(), 0);
        debug_assert_eq!(self.tracers_waiting(), 0);
        debug_assert_eq!(self.is_trace_completed(), true);
        debug_assert_eq!(self.mutators_stopped(), true);

        self.clean_up();
    }

    pub fn is_tracing(&self) -> bool {
        self.trace_lock.try_write().is_err()
    }

    pub fn trace<T: Trace>(self: Arc<Self>, root: &T, marker: Arc<M>) {
        self.clone().trace_root(root, marker.clone());
        self.clone().spawn_tracers(marker.clone());
    }

    fn clean_up(&self) {
        self.yield_flag.store(false, Ordering::SeqCst);
        self.trace_end_flag.store(false, Ordering::SeqCst);
        self.work_received.store(0, Ordering::SeqCst);
        self.work_sent.store(0, Ordering::SeqCst);
        self.tracers_waiting.store(0, Ordering::SeqCst);
    }

    fn trace_root<T: Trace>(self: Arc<Self>, root: &T, marker: Arc<M>) {
        let mut tracer = TraceWorker::new(self.clone(), marker.clone());
        root.trace(&mut tracer);
        tracer.flush_work();
    }

    fn spawn_tracers(self: Arc<Self>, marker: Arc<M>) {
        // create a channel to be used to wait until all tracers have started
        let (sender, receiver) = crossbeam_channel::unbounded::<()>();

        for _ in 0..self.num_tracers {
            let controller = self.clone();
            let sender = sender.clone();
            let marker = marker.clone();

            std::thread::spawn(move || {
                let _lock = controller.trace_lock.read().unwrap();
                let mut tracer = TraceWorker::new(controller.clone(), marker);

                sender.send(()).unwrap();

                tracer.trace_loop();
            });
        }

        // wait for tracers to start
        for _ in 0..self.num_tracers {
            receiver.recv().unwrap();
        }
    }

    fn tracers_waiting(&self) -> usize {
        self.tracers_waiting.load(Ordering::SeqCst)
    }

    fn start_waiting(&self) -> usize {
        self.tracers_waiting.fetch_add(1, Ordering::SeqCst)
    }

    fn stop_waiting(&self) {
        self.tracers_waiting.fetch_sub(1, Ordering::SeqCst);
    }

    fn sent(&self) -> usize {
        self.work_sent.load(Ordering::SeqCst)
    }

    fn received(&self) -> usize {
        self.work_received.load(Ordering::SeqCst)
    }

    fn mutators_stopped(&self) -> bool {
        self.yield_lock.try_write().is_ok()
    }
}
