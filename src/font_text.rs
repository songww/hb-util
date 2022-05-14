use clap::Parser as ClapApp;

use crate::consumer::Consumer;
use crate::options::{FontOpts, TextOpts};

pub struct FontText<Cons: Consumer> {
    cons: Cons,
    opts: Cons::Opts,
}

impl<Cons: Consumer> FontText<Cons> {
    pub fn new() -> Self {
        let mut opts = Cons::Opts::parse();
        opts.load_font();
        opts.read();
        let cons = Cons::with_options(&opts);
        Self { opts, cons }
    }

    pub fn run(&mut self) {
        unsafe {
            while let Ok(true) = self.cons.consume_line(&self.opts) {
                //
            }
            self.cons.finish(&self.opts);
        }
    }
}
