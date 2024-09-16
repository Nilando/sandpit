use super::trace::Trace;
use super::tracer::Tracer;
use super::trace_job::TraceJob;
use crate::config::GcConfig;
use crossbeam_channel::{Receiver, Sender};
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc, Mutex, MutexGuard, RwLock, RwLockReadGuard,
};
use std::time::Instant;
use crate::allocator::{Allocator, Allocate, GenerationalArena};

pub struct TracerController {
    sender: Sender<Vec<TraceJob>>,
    receiver: Receiver<Vec<TraceJob>>,

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
    alloc_lock: Mutex<()>,

    // config vars
    pub num_tracers: usize,
    pub trace_share_min: usize,
    pub trace_chunk_size: usize,
    pub trace_share_ratio: f32,
    pub trace_wait_time: u64,
    pub mutator_share_min: usize,
}

impl TracerController {
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
            alloc_lock: Mutex::new(()),

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

    pub fn get_alloc_lock(&self) -> MutexGuard<()> {
        self.alloc_lock.lock().unwrap()
    }

    pub fn is_alloc_lock(&self) -> bool {
        self.alloc_lock.try_lock().is_ok()
    }

    pub fn has_work(&self) -> bool {
        !self.receiver.is_empty()
    }

    pub fn incr_recv(&self) {
        self.work_received.fetch_add(1, Ordering::SeqCst);
    }

    pub fn send_work(&self, work: Vec<TraceJob>) {
        self.work_sent.fetch_add(1, Ordering::SeqCst);
        self.sender.send(work).unwrap();
    }

    pub fn recv_work(&self) -> Option<Vec<TraceJob>> {
        self.start_waiting();

        let duration = std::time::Duration::from_millis(self.trace_wait_time);
        let deadline = Instant::now().checked_add(duration).unwrap();

        loop {
            match self.receiver.recv_deadline(deadline) {
                Ok(work) => {
                    self.stop_waiting();
                    self.incr_recv();
                    return Some(work);
                }
                Err(_) => {
                    if self.is_trace_completed() {
                        self.stop_waiting();
                        return None;
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

        false
    }

    pub fn wait_for_trace_completion(&self) {
        drop(self.trace_lock.write().unwrap());

        debug_assert_eq!(self.sent(), self.received());
        debug_assert_eq!(self.sender.len(), 0);
        debug_assert_eq!(self.tracers_waiting(), 0);
        debug_assert!(self.is_trace_completed());
        debug_assert!(self.mutators_stopped());

        self.clean_up();
    }

    pub fn is_tracing(&self) -> bool {
        self.trace_lock.try_write().is_err()
    }

    pub fn trace<T: Trace>(self: Arc<Self>, root: &T, old_object_count: Arc<AtomicUsize>, mark: <<Allocator as Allocate>::Arena as GenerationalArena>::Mark) {
        self.clone().trace_root(root, old_object_count.clone(), mark);
        self.clone().spawn_tracers(old_object_count, mark);
    }

    fn clean_up(&self) {
        self.yield_flag.store(false, Ordering::SeqCst);
        self.trace_end_flag.store(false, Ordering::SeqCst);
        self.work_received.store(0, Ordering::SeqCst);
        self.work_sent.store(0, Ordering::SeqCst);
        self.tracers_waiting.store(0, Ordering::SeqCst);
    }

    fn trace_root<T: Trace>(self: Arc<Self>, root: &T, old_object_count: Arc<AtomicUsize>, mark: <<Allocator as Allocate>::Arena as GenerationalArena>::Mark) {
        let mut tracer = Tracer::new(self.clone(), mark);
        root.trace(&mut tracer);
        tracer.flush_work();
        old_object_count.fetch_add(tracer.get_mark_count(), Ordering::SeqCst);
    }

    fn spawn_tracers(self: Arc<Self>, old_object_count: Arc<AtomicUsize>, mark: <<Allocator as Allocate>::Arena as GenerationalArena>::Mark) {
        // create a channel to be used to wait until all tracers have started
        let (sender, receiver) = crossbeam_channel::unbounded::<()>();

        for i in 0..self.num_tracers {
            let controller = self.clone();
            let sender = sender.clone();
            let thread_name: String = format!("GC_TRACER_{i}");
            let thread = std::thread::Builder::new().name(thread_name);

            let object_count = old_object_count.clone();
            let _ = thread.spawn(move || {
                let _lock = controller.trace_lock.read().unwrap();
                let mut tracer = Tracer::new(controller.clone(), mark);

                sender.send(()).unwrap();

                tracer.trace_loop();
                object_count.clone().fetch_add(tracer.get_mark_count(), Ordering::SeqCst);
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
