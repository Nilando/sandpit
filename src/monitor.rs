use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use alloc::sync::Arc;
use std::env::var;
use std::sync::Mutex;
use std::thread;
use std::time;

use super::collector::Collect;
use super::config::Config;

// The monitor is responsible for automatically triggering garbage collections.
// It determines when to make a major and or minor collection by considering,
// the current and past size of the arena, as well as the number of "old objects"
// which are objects that have been traced.
pub struct Monitor<T: Collect + 'static> {
    collector: Arc<T>,

    flag: AtomicBool,
    monitor_lock: Mutex<()>,

    prev_arena_size: AtomicU64,
    max_old_objects: AtomicU64,

    // config vars
    max_old_growth_rate: f32,
    arena_size_ratio_trigger: f32,
    wait_duration: u64,
}

unsafe impl<T: Collect + 'static> Send for Monitor<T> {}
unsafe impl<T: Collect + 'static> Sync for Monitor<T> {}

impl<T: Collect + 'static> Monitor<T> {
    pub fn new(collector: Arc<T>, config: &Config) -> Self {
        let prev_arena_size = collector.get_arena_size();

        Self {
            collector,
            flag: AtomicBool::new(false),
            monitor_lock: Mutex::new(()),
            prev_arena_size: AtomicU64::new(prev_arena_size),
            // TODO make this a config var
            max_old_objects: AtomicU64::new(0),
            max_old_growth_rate: config.monitor_max_old_growth_rate,
            arena_size_ratio_trigger: config.monitor_arena_size_ratio_trigger,
            wait_duration: config.monitor_wait_time,
        }
    }

    pub fn stop(&self) {
        self.flag.store(false, Ordering::Relaxed);
        let _lock = self.monitor_lock.lock().unwrap();
    }

    pub fn get_max_old_objects(&self) -> u64 {
        self.max_old_objects.load(Ordering::Relaxed)
    }

    pub fn get_prev_arena_size(&self) -> u64 {
        self.prev_arena_size.load(Ordering::Relaxed)
    }

    pub fn start(self: Arc<Self>) {
        if self
            .flag
            .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
            .is_err()
        {
            return;
        }

        std::thread::spawn(move || self.monitor());
    }

    fn monitor(&self) {
        let _lock = self.monitor_lock.lock().unwrap();

        loop {
            self.sleep();

            if self.should_stop_monitoring() {
                break;
            }

            self.test_triggers();
        }
    }

    fn debug(&self) {
        let current_old = self.collector.get_old_objects_count() as u64;
        let max_old = self.max_old_objects.load(Ordering::Relaxed);
        let size = self.collector.get_arena_size();
        let prev_size = self.prev_arena_size.load(Ordering::Relaxed);
        println!("GC_DEBUG: max_old: {}, current_old: {}, prev_size: {} kb, size: {} kb", max_old, current_old, (prev_size/1024), (size/1024));
    }

    fn test_triggers(&self) {
        if self.minor_trigger() {
            if var("GC_DEBUG").is_ok() {
                self.debug();
            }
            // monitor collection {old max} {found old objects} {arena size}
            self.collector.minor_collect();

            if var("GC_DEBUG").is_ok() {
                self.debug();
            }

            if self.major_trigger() {
                if var("GC_DEBUG").is_ok() {
                    self.debug();
                }
                self.collector.major_collect();
                if var("GC_DEBUG").is_ok() {
                    self.debug();
                }

                self.update_old_max();
            }

            self.prev_arena_size
                .store(self.collector.get_arena_size(), Ordering::Relaxed);
        }
    }

    fn major_trigger(&self) -> bool {
        let old_objects = self.collector.get_old_objects_count() as u64;

        old_objects > self.max_old_objects.load(Ordering::Relaxed)
    }

    fn minor_trigger(&self) -> bool {
        let arena_size = self.collector.get_arena_size();
        let prev_arena_size = self.prev_arena_size.load(Ordering::Relaxed);

        arena_size as f32 > (prev_arena_size as f32 * self.arena_size_ratio_trigger)
    }

    fn update_old_max(&self) {
        let old_objects = self.collector.get_old_objects_count();

        self.max_old_objects.store(
            (old_objects as f32 * self.max_old_growth_rate).floor() as u64,
            Ordering::Relaxed,
        );
    }

    fn should_stop_monitoring(&self) -> bool {
        !self.flag.load(Ordering::Relaxed)
    }

    fn sleep(&self) {
        let duration = time::Duration::from_millis(self.wait_duration);

        thread::sleep(duration);
    }
}
