use super::trace::Trace;
use super::trace_job::TraceJob;
use super::tracer::Tracer;
use crate::config::GcConfig;
use crate::header::GcMark;
use crossbeam_channel::{Receiver, Sender};
use std::sync::{
    atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering},
    Arc, Mutex, MutexGuard, RwLock, RwLockReadGuard,
};
use std::thread::JoinHandle;
use std::time::Instant;

pub struct TracerController {
    sender: Sender<Vec<TraceJob>>,
    receiver: Receiver<Vec<TraceJob>>,

    yield_flag: AtomicBool,
    trace_end_flag: AtomicBool,
    // TODO: store in GcArray instead of vec?
    // this will be tricky since then tracing will require
    // access to a mutator, or at least the arena in some way
    // or should this be some kind of channel?
    tracers_waiting: AtomicUsize,
    work_sent: AtomicUsize,
    work_received: AtomicUsize,
    current_mark: AtomicU8,

    // This lock is set by time slicer to elongate the period in which mutators yield
    time_slice_lock: Mutex<()>,

    // mutators hold a ReadGuard of this lock preventing
    // the tracers from declaring the trace complete until
    // all mutators are stopped.
    yield_lock: RwLock<()>,

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
            tracers_waiting: AtomicUsize::new(0),
            work_sent: AtomicUsize::new(0),
            work_received: AtomicUsize::new(0),
            time_slice_lock: Mutex::new(()),
            current_mark: AtomicU8::new(GcMark::Red.into()),

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

    pub fn get_time_slice_lock(&self) -> MutexGuard<()> {
        self.time_slice_lock.lock().unwrap()
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
            // The tracers are out of work, raise this flag to stop the mutators.
            self.raise_yield_flag();

            if self.mutators_stopped() {
                // Let the other tracers know they should stop by raising this flag
                self.trace_end_flag.store(true, Ordering::SeqCst);
                return true;
            }
        }

        false
    }

    pub fn trace<T: Trace, F: FnOnce() -> ()>(
        self: Arc<Self>,
        root: &T,
        old_object_count: Arc<AtomicUsize>,
        trace_callback: F,
    ) {
        self.clone().trace_root(root, old_object_count.clone());
        let join_handles = self.clone().spawn_tracers(old_object_count);

        trace_callback();

        for jh in join_handles.into_iter() {
            jh.join().unwrap_or_else(|_| {
                println!("GC Tracer Panic");
                std::process::abort();
            });
        }

        self.clean_up();
    }

    pub fn rotate_mark(&self) -> GcMark {
        let new_mark = self.get_current_mark().rotate();

        self.current_mark.store(new_mark.into(), Ordering::SeqCst);

        new_mark
    }

    pub fn get_current_mark(&self) -> GcMark {
        self.current_mark.load(Ordering::SeqCst).into()
    }

    pub fn prev_mark(&self) -> GcMark {
        self.get_current_mark().prev()
    }

    fn clean_up(&self) {
        debug_assert_eq!(self.sent(), self.received());
        debug_assert_eq!(self.sender.len(), 0);
        debug_assert_eq!(self.tracers_waiting(), 0);
        debug_assert!(self.is_trace_completed());
        debug_assert!(self.mutators_stopped());

        self.yield_flag.store(false, Ordering::SeqCst);
        self.trace_end_flag.store(false, Ordering::SeqCst);
        self.work_received.store(0, Ordering::SeqCst);
        self.work_sent.store(0, Ordering::SeqCst);
        self.tracers_waiting.store(0, Ordering::SeqCst);
    }

    fn trace_root<T: Trace>(self: Arc<Self>, root: &T, old_object_count: Arc<AtomicUsize>) {
        let mut tracer = self.new_tracer(0);
        root.trace(&mut tracer);
        tracer.flush_work();
        old_object_count.fetch_add(tracer.get_mark_count(), Ordering::SeqCst);
    }

    fn new_tracer(self: Arc<Self>, id: usize) -> Tracer {
        let mark = self.get_current_mark();

        Tracer::new(self.clone(), mark, id)
    }

    fn spawn_tracers(self: Arc<Self>, old_object_count: Arc<AtomicUsize>) -> Vec<JoinHandle<()>> {
        let mut join_handles = vec![];

        for i in 0..self.num_tracers {
            let thread_name: String = format!("GC_TRACER_{i}");
            let thread = std::thread::Builder::new().name(thread_name);
            let object_count = old_object_count.clone();
            let controller = self.clone();
            let jh = thread
                .spawn(move || {
                    let mut tracer = controller.clone().new_tracer(i);
                    let marked_objects = tracer.trace_loop();

                    object_count.fetch_add(marked_objects, Ordering::SeqCst);
                })
                .unwrap_or_else(|_| {
                    println!("Failed to start GC Thread");
                    std::process::abort();
                });

            join_handles.push(jh);
        }

        join_handles
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
