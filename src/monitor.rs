use super::collector::Collect;
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc,
};

use std::thread;
use std::time;

const MAX_OLD_GROWTH_RATE: f64 = 10.0;
const ARENA_SIZE_RATIO_TRIGGER: f64 = 2.0;

pub struct Monitor<T: Collect + 'static> {
    collector: Arc<T>,
    flag: AtomicBool,
    prev_arena_size: AtomicUsize,
    max_old_objects: AtomicUsize,
}

unsafe impl<T: Collect + 'static> Send for Monitor<T> {}
unsafe impl<T: Collect + 'static> Sync for Monitor<T> {}

impl<T: Collect + 'static> Monitor<T> {
    pub fn new(collector: Arc<T>) -> Self {
        let prev_arena_size = collector.get_arena_size();
        Self {
            collector,
            flag: AtomicBool::new(false),
            prev_arena_size: AtomicUsize::new(prev_arena_size),
            max_old_objects: AtomicUsize::new(0),
        }
    }

    pub fn stop(&self) {
        self.flag.store(false, Ordering::SeqCst);
    }

    pub fn get_max_old_objects(&self) -> usize {
        self.max_old_objects.load(Ordering::SeqCst)
    }

    pub fn get_prev_arena_size(&self) -> usize {
        self.prev_arena_size.load(Ordering::SeqCst)
    }

    pub fn start(self: Arc<Self>) {
        if self
            .flag
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return;
        }

        std::thread::spawn(move || self.monitor());
    }

    fn monitor(&self) {
        loop {
            self.sleep();

            if self.should_stop_monitoring() {
                break;
            }

            if self.minor_trigger() {
                self.collector.minor_collect();

                if self.major_trigger() {
                    self.collector.major_collect();

                    self.update_old_max();
                }

                self.prev_arena_size
                    .store(self.collector.get_arena_size(), Ordering::SeqCst);
            }
        }
    }

    fn major_trigger(&self) -> bool {
        let old_objects = self.collector.get_old_objects_count();

        old_objects > self.max_old_objects.load(Ordering::SeqCst)
    }

    fn minor_trigger(&self) -> bool {
        let arena_size = self.collector.get_arena_size();
        let prev_arena_size = self.prev_arena_size.load(Ordering::SeqCst);

        arena_size as f64 >= (prev_arena_size as f64 * ARENA_SIZE_RATIO_TRIGGER)
    }

    fn update_old_max(&self) {
        let old_objects = self.collector.get_old_objects_count();

        self.max_old_objects.store(
            (old_objects as f64 * MAX_OLD_GROWTH_RATE).floor() as usize,
            Ordering::SeqCst,
        );
    }

    fn should_stop_monitoring(&self) -> bool {
        !self.flag.load(Ordering::SeqCst)
    }

    fn sleep(&self) {
        let ten_millis = time::Duration::from_millis(10);

        thread::sleep(ten_millis);
    }
}