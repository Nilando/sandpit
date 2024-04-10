use super::gc_ptr::GcPtr;
use super::monitor::Monitor;
use super::collector::GcController;
use std::sync::Arc;

pub struct Gc<C: GcController, M: Monitor> {
    controller: Arc<C>,
    monitor: Arc<M>,
    // config: Config
}

unsafe impl<C: GcController + Send, M: Monitor> Send for Gc<C, M> {}
unsafe impl<C: GcController + Sync, M: Monitor> Sync for Gc<C, M> {}

impl<C: GcController, M: Monitor> Drop for Gc<C, M> {
    fn drop(&mut self) {
        self.stop_monitor()
    }
}

impl<C: GcController, M: Monitor> Gc<C, M> {
    pub fn build(callback: fn(&mut C::Mutator) -> GcPtr<C::Root>) -> Self {
        let controller = Arc::new(C::build(callback));
        let monitor = Arc::new(M::new(controller.clone()));

        Self { controller, monitor }
    }

    pub fn mutate(&mut self, callback: fn(GcPtr<C::Root>, &mut C::Mutator)) {
        self.controller.mutate(callback);
    }

    pub fn collect(&mut self) {
        self.controller.collect();
    }

    pub fn eden_collect(&mut self) {
        self.controller.eden_collect();
    }

    pub fn start_monitor(&mut self) {
        self.monitor.start();
    }

    pub fn stop_monitor(&mut self) {
        self.monitor.stop();
    }
}
