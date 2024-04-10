use super::allocate::{GenerationalArena};
use super::collector::Collect;
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};

use std::thread;
use std::time;

pub trait Monitor {
    fn new<C: Collect>(collector: Arc<C>) -> Self;
    fn start(&self);
    fn stop(&self);
}

pub struct MonitorController {
    worker: Arc<MonitorWorker>,
}

struct MonitorWorker {
    collector: Arc<dyn Collect>,
    metrics: Mutex<MonitorMetrics>,
    monitor_flag: AtomicBool,
}

struct MonitorMetrics {
    prev_block_count: usize,
    debt: f64
}

impl MonitorMetrics {
    fn new() -> Self {
        Self {
            prev_block_count: 0,
            debt: 0.0
        }
    }
}

impl MonitorWorker {
    fn new(collector: Arc<dyn Collect>) -> Self {
        let monitor_flag = AtomicBool::new(false);

        Self {
            collector,
            monitor_flag,
            metrics: Mutex::new(MonitorMetrics::new()),
        }
    }
}

unsafe impl Send for MonitorWorker {}
unsafe impl Sync for MonitorWorker {}

const DEBT_CEILING: f64 = 10.0;
const DEBT_INTEREST_RATE: f64 = 1.5;

impl Monitor for MonitorController {
    fn new<C: Collect>(collector: Arc<C>) -> Self {
        let worker = Arc::new(MonitorWorker::new(collector));

        Self { worker }
    }

    fn stop(&self) {
        self.worker.monitor_flag.store(false, Ordering::Relaxed);
    }

    fn start(&self) {
        let flag = self.worker.monitor_flag.compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed);
        if flag.is_err() { return }

        let worker = self.worker.clone();

        thread::spawn(move || worker.monitor());
    }
}

impl MonitorWorker {
    fn monitor(&self) {
        loop {
            self.sleep();

            if !self.monitor_flag.load(Ordering::Relaxed) { break; }

            let mut metrics = self.metrics.lock().unwrap();
            metrics.debt *= DEBT_INTEREST_RATE;

            if metrics.prev_block_count < self.collector.arena_size() {
                let new_debt = self.collector.arena_size() - metrics.prev_block_count;

                metrics.debt += new_debt as f64;
            }

            if metrics.debt >= DEBT_CEILING {
                self.collector.collect();
                metrics.debt = 0.0;
            }
        }
    }

    fn sleep(&self) {
        let millis = time::Duration::from_millis(500);

        thread::sleep(millis);
    }
}
