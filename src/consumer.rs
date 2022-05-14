pub trait Consumer {
    // type Opts: clap::Parser;
    type Opts: clap::Parser;
    fn with_options(options: &Self::Opts) -> Self;
    unsafe fn consume_line(&mut self, options: &Self::Opts) -> anyhow::Result<bool>;
    unsafe fn finish(&mut self, options: &Self::Opts);
}
