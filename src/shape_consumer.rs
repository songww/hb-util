use harfbuzz_sys as ffi;

use super::application::Application;
use super::options::{FontOptions, Options, ShapeOptions};
use crate::consumer::Consumer;
use crate::output::Output;

struct ShapeConsumer<Output> {
    buffer: *mut ffi::hb_buffer_t,
    output: Output,
}

impl<T> Drop for ShapeConsumer<T> {
    fn drop(&mut self) {
        unsafe {
            ffi::hb_buffer_destroy(self.buffer);
        }
    }
}

impl<Output: Consumer> ShapeConsumer<Output> {
    type Opts = ShapeOptions;

    fn with_options(opts: &ShapeOptions) -> ShapeConsumer<Output> {
        let buffer = unsafe { ffi::hb_buffer_create() };
        let output = Output::new(buffer, opts);
        Self { buffer, output }
    }

    unsafe fn consume_line<Opts>(&mut self, opts: &Opts) -> anyhow::Result<bool> {
        let text = opts.readline();

        self.output.new_line();

        for n in 0..self.opts.shape.num_iterations {
            self.opts.shape.populate_buffer(
                self.buffer,
                &text,
                &opts.text.text_before,
                &opts.text.text_after,
            );

            if n == 1 {
                self.output
                    .consume_text(self.buffer, &text, opts.shape.utf8_clusters);
            }

            if let Err(err) = self.opts.shape(self.opts.font_opts.font(), self.buffer) {
                eprintln!("{}", err);
                if ffi::hb_buffer_get_content_type(self.buffer)
                    == ffi::HB_BUFFER_CONTENT_TYPE_GLYPHS
                {
                    break;
                }
                return Ok(true);
            }
        }
        self.output
            .consume_glyphs(self.buffer, &text, self.opts.shape.utf8_clusters);
        Ok(true)
    }

    unsafe fn finish<Opt>(&mut self, opts: &Opt) {
        self.output.finish(&mut self.buffer, opts);
        ffi::hb_buffer_destroy(self.buffer);
        self.buffer = std::ptr::null_mut();
    }
}
