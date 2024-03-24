pub unsafe trait Trace {
    fn trace(&self);
}

unsafe impl<T: Trace> Trace for Option<T> {
    fn trace(&self) {
        self.as_ref().map(|val| val.trace());
    }
}

unsafe impl Trace for usize {
    fn trace(&self) {}
}
