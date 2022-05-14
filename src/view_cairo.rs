use std::mem::MaybeUninit;

use harfbuzz_sys as ffi;

use crate::helper_cairo::{
    create_cairo_context, create_scaled_font, HelperCairoLine, ScaledFontExt,
};
use crate::options::{FontExtents, Options};
use crate::output::Output;

const SUBPIXEL_BITS: i32 = 6;

pub struct ViewCairo {
    scale_bits: i32,
    direction: ffi::hb_direction_t,
    lines: Vec<HelperCairoLine>,
}

impl Output for ViewCairo {
    type Opts = Options;
    fn create(_buffer: *mut ffi::hb_buffer_t, _opts: &Options) -> ViewCairo {
        ViewCairo {
            scale_bits: SUBPIXEL_BITS,
            direction: ffi::HB_DIRECTION_INVALID,
            lines: Vec::new(),
        }
    }

    fn new_line(&self) {}
    unsafe fn consume_text(
        &mut self,
        _buffer: *mut ffi::hb_buffer_t,
        _text: &str,
        _utf8_clusters: bool,
    ) {
    }
    unsafe fn consume_glyphs(
        &mut self,
        buffer: *mut ffi::hb_buffer_t,
        text: &str,
        utf8_clusters: bool,
    ) {
        self.direction = ffi::hb_buffer_get_direction(buffer);
        let l = HelperCairoLine::from_buffer(buffer, text, self.scale_bits, utf8_clusters);
        self.lines.push(l);
    }
    unsafe fn finish(&mut self, _buffer: *mut ffi::hb_buffer_t, opts: &Options) {
        self.render(opts);
    }
}

impl ViewCairo {
    unsafe fn render(&self, opts: &Options) -> anyhow::Result<()> {
        let is_vertical = crate::hb_direction_is_vertical(self.direction);
        let vert = if is_vertical { 1. } else { 0. };
        let horiz = if is_vertical { 0. } else { 1. };

        let font_size = opts.font_opts.font_size.unwrap_or_default();

        let x_sign = if font_size.x < 0. { -1. } else { 1. };
        let y_sign = if font_size.y < 0. { -1. } else { 1. };

        let font = opts.font_opts.font();

        let font_extents = if let Some(extents) = opts.view.font_extents {
            extents
        } else {
            let mut hb_extents = MaybeUninit::zeroed();
            ffi::hb_font_get_extents_for_direction(font, self.direction, hb_extents.as_mut_ptr());
            let hb_extents = hb_extents.assume_init();
            FontExtents {
                ascent: libm::scalbn(hb_extents.ascender as _, self.scale_bits),
                descent: -libm::scalbn(hb_extents.descender as _, self.scale_bits),
                line_gap: libm::scalbn(hb_extents.line_gap as _, self.scale_bits),
            }
        };

        let ascent = y_sign * font_extents.ascent;
        let descent = y_sign * font_extents.descent;
        let line_gap = y_sign * font_extents.line_gap + opts.view.line_space;
        let leading = ascent + descent + line_gap;

        let mut w = 0.;
        let mut h = 0.;
        let v = self.lines.len() as f64 * leading - (font_extents.line_gap + opts.view.line_space);
        if is_vertical {
            w = v;
            h = 0.;
        } else {
            h = v;
            w = 0.;
        }

        for line in self.lines.iter() {
            let (x_advance, y_advance) = line.advance();
            if is_vertical {
                h = h.max(y_sign * y_advance);
            } else {
                w = w.max(x_sign * x_advance);
            }
        }

        let scaled_font = create_scaled_font(&opts.font_opts)?;

        let content = if scaled_font.has_color() {
            cairo::Content::Color
        } else {
            cairo::Content::Alpha
        };

        let margin = opts.view.margin.unwrap_or_default();

        let cr = create_cairo_context(
            w + margin.l + margin.r,
            h + margin.t + margin.b,
            &opts.view,
            &opts.output,
            content,
        )?;

        cr.set_scaled_font(&scaled_font);

        cr.translate(margin.l, margin.t);

        if is_vertical {
            cr.translate(w - ascent, y_sign.clamp(0., h));
        } else {
            cr.translate(
                x_sign.clamp(0., w),
                if y_sign < 0. { descent } else { ascent },
            );
        }

        // Draw
        cr.translate(vert * leading, -horiz * leading);
        for l in self.lines.iter() {
            cr.translate(-vert * leading, horiz * leading);
            if opts.view.annotate {
                todo!()
            }

            if false && cr.target().type_() == cairo::SurfaceType::Image {
                // cairo_show_glyphs dosen't supported subpixel positioning
                cr.glyph_path(&l.glyphs);
                cr.fill();
            } else if !l.text_clusters.is_empty() {
                cr.show_text_glyphs(&l.utf8, &l.glyphs, &l.text_clusters, l.cluster_flags);
            } else {
                cr.show_glyphs(&l.glyphs);
            }
        }
        Ok(())
    }
}
