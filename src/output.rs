use harfbuzz_sys as ffi;

pub trait Output {
    type Opts: clap::Parser;
    fn create(buffer: *mut ffi::hb_buffer_t, opts: &Self::Opts) -> Self;
    fn new_line(&self);
    unsafe fn consume_text(
        &mut self,
        buffer: *mut ffi::hb_buffer_t,
        text: &str,
        utf8_clusters: bool,
    );
    unsafe fn consume_glyphs(
        &mut self,
        buffer: *mut ffi::hb_buffer_t,
        text: &str,
        utf8_clusters: bool,
    );
    unsafe fn finish(&mut self, buffer: *mut ffi::hb_buffer_t, opts: &Self::Opts);
}
