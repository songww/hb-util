use clap::StructOpt;

use crate::options::Options;

pub struct FontText<Consumer, FontOptions, TextOptions> {
    text_opts: TextOptions,
    font_opts: FontOptions,
    consumer: Consumer,
}

impl<Consumer, FontOptions, TextOptions> FontText<Consumer, FontOptions, TextOptions> {
    pub fn new() -> Self {
        Options::try_parse().unwrap();
        todo!()
    }

    pub fn main(&self) {
        unsafe {
            // self.consumer.init()
            while let Some(true) = self.consumer.consume_line() {
                //
            }
            self.consumer.finish();
        }
    }
}
