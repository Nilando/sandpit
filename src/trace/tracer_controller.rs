use super::marker::Marker;
use crate::config::GcConfig;
use super::trace::Trace;
use super::trace_job::TraceJob;
use super::tracer::TraceWorker;
use crossbeam_channel::{Receiver, Sender};
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc, RwLock, RwLockReadGuard,
};
use std::time::Instant;

pub struct TracerController<M: Marker> {
    yield_flag: AtomicBool,
    yield_lock: RwLock<()>,
    trace_end_flag: AtomicBool,
    // TODO: store in GcArray instead of vec?
    // this will be tricky since then tracing will require
    // access to a mutator, or at least the arena in some way
    // or should this be some kind of channel?
    sender: Sender<Vec<TraceJob<M>>>,
    receiver: Receiver<Vec<TraceJob<M>>>,
    tracers_waiting: AtomicUsize,
    work_sent: AtomicUsize,
    work_received: AtomicUsize,

    // config vars
    pub num_tracers: usize,
    pub trace_share_min: usize,
    pub trace_chunk_size: usize,
    pub trace_share_ratio: f32,
    pub trace_wait_time: u64,
    pub mutator_share_min: usize,
    // max_headroom = ((prev_arena_size * arena_size_ratio_trigger) * 0.5) - current_size
    // available_headroom = max_headroom - current_size
    // C = collector_time
    // M = mutator_time
    // H = max_headroom / available_headroom
    // timeslice_size = 2
    // min_collector_time = 0.6
    // M = (timeslice_size - 0.6) * H 
    // C = timeslice_size - M
    // C = how long the mutators are paused
    // TODO: The tracer controller needs:
    // prev_arena_size
    // arena_size_ratio_trigger
    // and current arena size
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
            tracers_waiting: AtomicUsize::new(0),
            work_sent: AtomicUsize::new(0),
            work_received: AtomicUsize::new(0),

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

    pub fn yield_lock(&self) -> RwLockReadGuard<()> {
        self.yield_lock.read().unwrap()
    }

    pub fn mutators_stopped(&self) -> bool {
        self.yield_lock.try_write().is_ok()
    }

    pub fn has_work(&self) -> bool {
        !self.receiver.is_empty()
    }

    pub fn num_tracers(&self) -> usize {
        self.num_tracers
    }

    pub fn incr_recv(&self) {
        self.work_received.fetch_add(1, Ordering::SeqCst);
    }

    pub fn sent(&self) -> usize {
        self.work_sent.load(Ordering::SeqCst)
    }

    pub fn received(&self) -> usize {
        self.work_received.load(Ordering::SeqCst)
    }

    pub fn tracers_waiting(&self) -> usize {
        self.tracers_waiting.load(Ordering::SeqCst)
    }

    pub fn start_waiting(&self) -> usize {
        self.tracers_waiting.fetch_add(1, Ordering::SeqCst)
    }

    pub fn stop_waiting(&self) {
        self.tracers_waiting.fetch_sub(1, Ordering::SeqCst);
    }

    pub fn send_work(&self, work: Vec<TraceJob<M>>) {
        self.work_sent.fetch_add(1, Ordering::SeqCst);
        self.sender.send(work).unwrap();
    }

    pub fn recv_work(&self) -> Option<Vec<TraceJob<M>>> {
        let duration = std::time::Duration::from_millis(self.trace_wait_time);
        let deadline = Instant::now().checked_add(duration).unwrap();

        match self.receiver.recv_deadline(deadline) {
            Ok(work) => Some(work),
            Err(_) => None,
        }
    }

    pub fn is_trace_completed(&self) -> bool {
        if self.trace_end_flag.load(Ordering::SeqCst) {
            return true;
        }

        if self.tracers_waiting() == self.num_tracers() && self.sent() == self.received() {
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

    pub fn trace<T: Trace>(self: Arc<Self>, root: &T, marker: Arc<M>) {
        self.clone().trace_root(root, marker.clone());
        self.clone().spawn_tracers(marker.clone());
        self.clean_up();
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
                let mut tracer = TraceWorker::new(controller.clone(), marker);

                tracer.trace_loop();

                sender.send(()).unwrap();
            });
        }

        /*
        let now_in_nanos = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_nanos();
        let timeslice_size = 2_000_000;
        let remainder = now_in_nanos % timeslice_size;
        */

        // wait M time
        // is trace finished?
        // grab write_barrier
        // trace for C time
        // is trace finished?
        // drop write_barrier

        // wait for tracers to finish
        for _ in 0..self.num_tracers {
            receiver.recv().unwrap();
        }

        debug_assert_eq!(self.sent(), self.received());
        debug_assert_eq!(self.sender.len(), 0);
        debug_assert_eq!(self.tracers_waiting(), 0);
        debug_assert_eq!(self.is_trace_completed(), true);
        debug_assert_eq!(self.mutators_stopped(), true);
    }
}
