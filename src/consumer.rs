pub trait Consumer {
    type Opts;
    unsafe fn with_options(options: &Self::Opts) -> Self;
    unsafe fn consume_line(&self, options: &Self::Opts) -> anyhow::Result<bool>;
    unsafe fn new_line(&self) {}
    unsafe fn finish(&self, options: &Self::Opts);
}
