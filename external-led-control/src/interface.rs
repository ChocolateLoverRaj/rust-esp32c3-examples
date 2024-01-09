pub trait Interface {
    fn notify_change(&mut self);
    fn stop(self);
}
