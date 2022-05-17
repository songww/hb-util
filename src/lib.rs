#![feature(inherent_associated_types)]

pub mod application;
pub mod consumer;
pub mod font_text;
pub mod helper_cairo;
pub mod options;
pub mod output;
pub mod shape_consumer;
pub mod view_cairo;

use harfbuzz_sys as ffi;

#[inline(always)]
pub fn hb_direction_is_vertical(dir: ffi::hb_direction_t) -> bool {
    (dir as ::std::os::raw::c_uint) & !1 == 6
}

#[inline(always)]
pub fn hb_direction_is_backward(dir: ffi::hb_direction_t) -> bool {
    (dir as ::std::os::raw::c_uint) & !2 == 5
}

pub use helper_cairo::{
    render_color_glyph, render_glyph, HbFont, ScaledFontExt, HB_CAIRO_FONT_KEY,
};
