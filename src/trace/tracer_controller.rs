use super::marker::Marker;
use super::trace::Trace;
use super::trace_job::TraceJob;
use super::tracer::TraceWorker;
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc, RwLock, RwLockReadGuard,
};
use crossbeam_channel::{Sender, Receiver};

const NUM_TRACER_THREADS: usize = 3;

unsafe impl<M: Marker> Send for TracerController<M> {}
unsafe impl<M: Marker> Sync for TracerController<M> {}

pub struct TracerController<M: Marker> {
    yield_flag: AtomicBool,
    yield_lock: RwLock<()>,
    mutators_stopped_flag: AtomicBool,
    tracer_lock: RwLock<()>,
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
    num_tracers: usize
}

impl<M: Marker> TracerController<M> {
    pub fn new() -> Self {
        let (sender, receiver) = crossbeam_channel::unbounded();

        Self {
            yield_flag: AtomicBool::new(false),
            yield_lock: RwLock::new(()),
            mutators_stopped_flag: AtomicBool::new(false),
            trace_end_flag: AtomicBool::new(false),
            tracer_lock: RwLock::new(()),
            tracers_waiting: AtomicUsize::new(0),
            work_sent: AtomicUsize::new(0),
            work_received: AtomicUsize::new(0),
            num_tracers: NUM_TRACER_THREADS,
            sender,
            receiver
        }
    }

    pub fn yield_flag(&self) -> bool {
        self.yield_flag.load(Ordering::SeqCst)
    }

    pub fn yield_lock(&self) -> RwLockReadGuard<()> {
        self.yield_lock.read().unwrap()
    }

    pub fn tracer_lock(&self) -> RwLockReadGuard<()> {
        self.tracer_lock.read().unwrap()
    }

    pub fn mutators_stopped(&self) -> bool {
        self.mutators_stopped_flag.load(Ordering::SeqCst)
    }

    pub fn get_sender(&self) -> Sender<Vec<TraceJob<M>>> {
        self.sender.clone()
    }

    pub fn num_tracers(&self) -> usize {
        self.num_tracers
    }

    pub fn incr_send(&self) {
        self.work_sent.fetch_add(1, Ordering::SeqCst);
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

    pub fn is_trace_completed(&self) -> bool {
        if self.trace_end_flag.load(Ordering::SeqCst) {
            return true;
        }

        if  self.tracers_waiting() == self.num_tracers() &&
            self.sent() == self.received() &&
            self.mutators_stopped() {
                self.trace_end_flag.store(true, Ordering::SeqCst);
            return true;
        } else {
            return false;
        }
    }

    pub fn trace<T: Trace>(self: Arc<Self>, root: &T, marker: Arc<M>) {
        self.clone().spawn_tracers(root, marker.clone());
        self.wait_for_tracers_to_start();
        self.monitor_trace();
    }

    fn wait_for_tracers_to_start(&self) {
        let time = std::time::Duration::from_millis(1000);

        std::thread::sleep(time);
    }

    fn wait_for_mutators(&self) {
        self.yield_flag.store(true, Ordering::SeqCst);
        let _mutator_lock = self.yield_lock.write().unwrap();
        self.mutators_stopped_flag.store(true, Ordering::SeqCst);
    }

    fn wait_for_tracers_to_finish(&self) {
        let _tracer_lock = self.tracer_lock.write().unwrap();

        debug_assert_eq!(self.sent(), self.received());
        debug_assert_eq!(self.sender.len(), 0);
        debug_assert_eq!(self.tracers_waiting(), 0);
        debug_assert_eq!(self.yield_flag(), true);
        debug_assert_eq!(self.is_trace_completed(), true);
        debug_assert_eq!(self.mutators_stopped(), true);
    }

    fn monitor_trace(self: Arc<Self>) {
        self.wait_for_mutators();
        self.wait_for_tracers_to_finish();
        self.clean_up();
    }

    fn clean_up(&self) {
        self.yield_flag.store(false, Ordering::SeqCst);
        self.mutators_stopped_flag.store(false, Ordering::SeqCst);
        self.trace_end_flag.store(false, Ordering::SeqCst);
        self.work_received.store(0, Ordering::SeqCst);
        self.work_sent.store(0, Ordering::SeqCst);
        self.tracers_waiting.store(0, Ordering::SeqCst);
    }

    fn spawn_tracers<T: Trace>(self: Arc<Self>, root: &T, marker: Arc<M>) {
        for i in 0..NUM_TRACER_THREADS {
            let mut tracer = TraceWorker::new(
                self.clone(),
                marker.clone(),
                self.sender.clone(),
                self.receiver.clone(),
            );

            if i == 0 {
                root.trace(&mut tracer);
            }

            let binding = self.clone();
            std::thread::spawn(move|| {
                let _lock = binding.tracer_lock();
                tracer.trace_loop();
            });
        }
    }
}
