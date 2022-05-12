pub trait Consumer: Default {
    type Opts;
    unsafe fn init<O>(&mut self, options: &O);
    unsafe fn consume_line<O>(&self, options: &O) -> anyhow::Result<bool>;
    unsafe fn finish(&self);
    unsafe fn new_line(&self) {}
}
