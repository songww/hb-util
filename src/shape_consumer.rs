use harfbuzz_sys as ffi;

use crate::consumer::Consumer;
use crate::options::{FontOpts, ShapeOpts, TextOpts};
use crate::output::Output;

pub struct ShapeConsumer<Out> {
    buffer: *mut ffi::hb_buffer_t,
    out: Out,
}

impl<T> Drop for ShapeConsumer<T> {
    fn drop(&mut self) {
        unsafe {
            ffi::hb_buffer_destroy(self.buffer);
        }
    }
}

impl<Out: Output> Consumer for ShapeConsumer<Out>
where
    Out::Opts: FontOpts + ShapeOpts + TextOpts,
{
    type Opts = Out::Opts;

    fn with_options(opts: &<Out as Output>::Opts) -> ShapeConsumer<Out> {
        let buffer = unsafe { ffi::hb_buffer_create() };
        let out = Out::create(buffer, opts);
        Self { buffer, out }
    }

    unsafe fn consume_line(&mut self, opts: &Out::Opts) -> anyhow::Result<bool> {
        let text = opts
            .readline()
            .ok_or_else(|| anyhow::anyhow!("no more line"))?;

        self.out.new_line();

        for n in 0..opts.num_iterations() {
            opts.populate_buffer(self.buffer, &text, opts.text_before(), opts.text_after());

            if n == 1 {
                self.out
                    .consume_text(self.buffer, &text, opts.utf8_clusters());
            }

            if let Err(err) = opts.shape(opts.font().as_ptr(), self.buffer) {
                eprintln!("{}", err);
                if ffi::hb_buffer_get_content_type(self.buffer)
                    == ffi::HB_BUFFER_CONTENT_TYPE_GLYPHS
                {
                    break;
                }
                return Ok(true);
            }
        }
        self.out
            .consume_glyphs(self.buffer, &text, opts.utf8_clusters());
        Ok(true)
    }

    unsafe fn finish(&mut self, opts: &Out::Opts) {
        self.out.finish(self.buffer, opts);
        ffi::hb_buffer_destroy(self.buffer);
        self.buffer = std::ptr::null_mut();
    }
}
