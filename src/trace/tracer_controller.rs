use super::marker::Marker;
use std::time::Instant;
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
        self.yield_lock.try_write().is_ok()
    }

    pub fn has_work(&self) -> bool {
        !self.receiver.is_empty()
    }

    pub fn send_work(&self, work: Vec<TraceJob<M>>) {
        self.work_sent.fetch_add(1, Ordering::SeqCst);
        self.sender.send(work).unwrap();
    }

    pub fn recv_work(&self) -> Option<Vec<TraceJob<M>>> {
        let duration = std::time::Duration::from_millis(10);
        let deadline = Instant::now().checked_add(duration).unwrap();

        match self.receiver.recv_deadline(deadline) {
            Ok(work) => Some(work),
            Err(_) => None,
        }
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

    pub fn is_trace_completed(&self) -> bool {
        if self.trace_end_flag.load(Ordering::SeqCst) {
            return true;
        }

        if  self.tracers_waiting() == self.num_tracers() &&
            self.sent() == self.received() {
            if self.mutators_stopped() {
                self.trace_end_flag.store(true, Ordering::SeqCst);
                return true;
            } else {
                self.yield_flag.store(true, Ordering::SeqCst);
            }
        }

        return false
    }

    pub fn trace<T: Trace>(self: Arc<Self>, root: &T, marker: Arc<M>) {
        self.clone().spawn_tracers(root, marker.clone());
        self.wait_for_tracers();
        self.clean_up();
    }

    fn wait_for_tracers(&self) {
        println!("sent: {}", self.sent());
        println!("recv: {}", self.received());
        println!("waiting: {}", self.tracers_waiting());
        let _tracer_lock = self.tracer_lock.write().unwrap();
        println!("done");

        debug_assert_eq!(self.sent(), self.received());
        debug_assert_eq!(self.sender.len(), 0);
        debug_assert_eq!(self.tracers_waiting(), 0);
        debug_assert_eq!(self.is_trace_completed(), true);
        debug_assert_eq!(self.mutators_stopped(), true);
    }

    fn clean_up(&self) {
        self.yield_flag.store(false, Ordering::SeqCst);
        self.trace_end_flag.store(false, Ordering::SeqCst);
        self.work_received.store(0, Ordering::SeqCst);
        self.work_sent.store(0, Ordering::SeqCst);
        self.tracers_waiting.store(0, Ordering::SeqCst);
    }

    fn spawn_tracers<T: Trace>(self: Arc<Self>, root: &T, marker: Arc<M>) {
        let (sender, receiver) = crossbeam_channel::unbounded::<()>();

        for i in 0..NUM_TRACER_THREADS {
            let mut tracer = TraceWorker::new(
                self.clone(),
                marker.clone(),
            );

            if i == 0 {
                root.trace(&mut tracer);
            }

            let binding = self.clone();
            let sender = sender.clone();
            std::thread::spawn(move|| {
                let _lock = binding.tracer_lock();

                sender.send(());

                tracer.trace_loop();
            });
        }

        for _ in 0..NUM_TRACER_THREADS {
            receiver.recv().unwrap();
        }
    }
}
