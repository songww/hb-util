use std::{mem::MaybeUninit, rc::Rc};

use harfbuzz_sys as ffi;

use crate::options::{FontOptions, OutputAndFormatOptions, OutputFormat, ViewOptions};

pub struct HbFont(*mut ffi::hb_font_t);
impl Drop for HbFont {
    fn drop(&mut self) {
        unsafe { ffi::hb_font_destroy(self.0) }
    }
}
impl HbFont {
    pub fn as_ptr(&self) -> *mut ffi::hb_font_t {
        self.0
    }
    pub unsafe fn from_raw(raw: *mut ffi::hb_font_t) -> Self {
        Self(raw)
    }
}

pub struct HelperCairoLine {
    pub glyphs: Vec<cairo::Glyph>,
    pub utf8: String,
    pub text_clusters: Vec<cairo::TextCluster>,
    pub cluster_flags: cairo::TextClusterFlags,
}

impl HelperCairoLine {
    pub fn advance(&self) -> (f64, f64) {
        let glyph = self.glyphs.last().unwrap();
        (glyph.x(), glyph.y())
    }

    pub unsafe fn from_buffer(
        buffer: *mut ffi::hb_buffer_t,
        text: &str,
        scale_bits: i32,
        utf8_clusters: bool,
    ) -> Self {
        let num_glyphs = ffi::hb_buffer_get_length(buffer);
        let hb_glyph = ffi::hb_buffer_get_glyph_infos(buffer, std::ptr::null_mut());
        let hb_position = ffi::hb_buffer_get_glyph_positions(buffer, std::ptr::null_mut());

        let mut glyphs: Vec<cairo::ffi::cairo_glyph_t> =
            Vec::with_capacity(num_glyphs as usize + 1);

        let mut num_clusters = if num_glyphs > 0 { 1 } else { 0 };
        for i in 1..(num_clusters as isize) {
            if (*hb_glyph.offset(i)).cluster != (*hb_glyph.offset(i - 1)).cluster {
                num_clusters += 1;
            }
        }
        let mut clusters: Vec<cairo::ffi::cairo_text_cluster_t> =
            Vec::with_capacity(num_clusters as usize);
        clusters.resize_with(num_clusters as usize, || cairo::ffi::cairo_text_cluster_t {
            num_bytes: 0,
            num_glyphs: 0,
        });

        let mut x = 0.;
        let mut y = 0.;
        for i in 0..num_glyphs {
            let glyph = cairo::ffi::cairo_glyph_t {
                index: (*hb_glyph.offset(i as _)).codepoint as _,
                x: libm::scalbn((*hb_position).x_offset as f64 + x, scale_bits),
                y: libm::scalbn(-(*hb_position).y_offset as f64 + y, scale_bits),
            };
            x += (*hb_position).x_advance as f64;
            y += -(*hb_position).y_advance as f64;
            glyphs.push(glyph.into());
        }
        glyphs.push({
            cairo::ffi::cairo_glyph_t {
                index: u64::MAX,
                x: libm::scalbn(x, scale_bits),
                y: libm::scalbn(y, scale_bits),
            }
            .into()
        });

        if !clusters.is_empty() {
            let is_backward = crate::hb_direction_is_backward(ffi::hb_buffer_get_direction(buffer));
            let cluster_flags = if is_backward {
                cairo::TextClusterFlags::Backward
            } else {
                cairo::TextClusterFlags::None
            };

            let cluster = 0;
            clusters[cluster].num_glyphs += 1;

            if is_backward {
                for i in (0..((num_glyphs as isize) - 2)).rev() {
                    if (*hb_glyph.offset(i)).cluster != (*hb_glyph.offset(i + 1)).cluster {
                        assert!((*hb_glyph.offset(i)).cluster > (*hb_glyph.offset(i + 1)).cluster);
                        let end = if utf8_clusters {
                            todo!();
                        };
                    }
                    todo!()
                }
            } else {
                todo!()
            }
        }
        todo!()
    }
}

pub trait HelperCairoScaledFont {
    unsafe fn has_color(&mut self) -> bool {
        false
    }
}

#[allow(non_upper_case_globals)]
static _hb_font_cairo_user_data_key: cairo::ffi::cairo_user_data_key_t =
    cairo::ffi::cairo_user_data_key_t { unused: 0 };

unsafe extern "C" fn move_to(
    dfuncs: *mut ffi::hb_draw_funcs_t,
    cr: *mut cairo::ffi::cairo_t,
    st: *mut ffi::hb_draw_state_t,
    to_x: f32,
    to_y: f32,
    _: *mut std::ffi::c_void,
) {
    let cr = cairo::Context::from_raw_none(cr);
    cr.move_to(to_x as f64, to_y as f64);
}

unsafe extern "C" fn line_to(
    dfuncs: *mut ffi::hb_draw_funcs_t,
    cr: *mut cairo::ffi::cairo_t,
    st: *mut ffi::hb_draw_state_t,
    to_x: f32,
    to_y: f32,
    _: *mut std::ffi::c_void,
) {
    let cr = cairo::Context::from_raw_none(cr);
    cr.line_to(to_x as f64, to_y as f64);
}

unsafe extern "C" fn cubic_to(
    dfuncs: *mut ffi::hb_draw_funcs_t,
    cr: *mut cairo::ffi::cairo_t,
    st: *mut ffi::hb_draw_state_t,
    control1_x: f32,
    control1_y: f32,
    control2_x: f32,
    control2_y: f32,
    to_x: f32,
    to_y: f32,
    _: *mut std::ffi::c_void,
) {
    let cr = cairo::Context::from_raw_none(cr);
    cr.curve_to(
        control1_x as f64,
        control1_y as f64,
        control2_x as f64,
        control2_y as f64,
        to_x as f64,
        to_y as f64,
    );
}

unsafe extern "C" fn close_path(
    dfuncs: *mut ffi::hb_draw_funcs_t,
    cr: *mut std::os::raw::c_void,
    st: *mut ffi::hb_draw_state_t,
    _: *mut std::ffi::c_void,
) {
    // cr: *mut cairo::ffi::cairo_t,
    let cr = cairo::Context::from_raw_none(cr as _);
    cr.close_path();
}

struct DrawFuncs(*mut ffi::hb_draw_funcs_t);
unsafe impl Send for DrawFuncs {}
unsafe impl Sync for DrawFuncs {}

static DFUNCS: once_cell::sync::Lazy<DrawFuncs> = once_cell::sync::Lazy::new(|| unsafe {
    let dfuncs = ffi::hb_draw_funcs_create();
    ffi::hb_draw_funcs_set_move_to_func(
        dfuncs,
        Some(std::mem::transmute(move_to as *const ())),
        std::ptr::null_mut(),
        None,
    );
    ffi::hb_draw_funcs_set_line_to_func(
        dfuncs,
        Some(std::mem::transmute(line_to as *const ())),
        std::ptr::null_mut(),
        None,
    );
    ffi::hb_draw_funcs_set_cubic_to_func(
        dfuncs,
        Some(std::mem::transmute(cubic_to as *const ())),
        std::ptr::null_mut(),
        None,
    );
    ffi::hb_draw_funcs_set_close_path_func(dfuncs, Some(close_path), std::ptr::null_mut(), None);
    ffi::hb_draw_funcs_make_immutable(dfuncs);
    DrawFuncs(dfuncs)
});

unsafe extern "C" fn cairo_draw_funcs() -> *mut ffi::hb_draw_funcs_t {
    DFUNCS.0
}

fn render_glyph(
    scaled_font: &cairo::ScaledFont,
    glyph: std::os::raw::c_ulong,
    cr: &cairo::Context,
    extents: &mut cairo::TextExtents,
) -> Result<(), cairo::Error> {
    let font_face = scaled_font.font_face();
    let hb_font = font_face.user_data(&HB_CAIRO_FONT_KEY).unwrap();
    let mut x_scale = 0;
    let mut y_scale = 0;
    unsafe {
        ffi::hb_font_get_scale(
            hb_font.as_ptr(),
            &mut x_scale as *mut _,
            &mut y_scale as *mut _,
        );
    }
    cr.scale(x_scale as f64, y_scale as f64);

    unsafe {
        ffi::hb_font_get_glyph_shape(
            hb_font.as_ptr(),
            glyph as _,
            cairo_draw_funcs(),
            cr.to_raw_none() as *mut _,
        );
    }
    cr.fill()?;
    Ok(())
}

unsafe extern "C" fn _render_glyph(
    scaled_font: *mut cairo::ffi::cairo_scaled_font_t,
    glyph: u32,
    cr: *mut cairo::ffi::cairo_t,
    _extents: *mut cairo::ffi::cairo_text_extents_t,
) -> cairo::ffi::cairo_status_t {
    let font_face = cairo::ffi::cairo_scaled_font_get_font_face(scaled_font);
    let font = cairo::ffi::cairo_font_face_get_user_data(
        font_face,
        &_hb_font_cairo_user_data_key as *const _,
    );
    let font: *mut ffi::hb_font_t = font as _;
    let mut x_scale: ffi::hb_position_t = 0;
    let mut y_scale: ffi::hb_position_t = 0;
    ffi::hb_font_get_scale(font, &mut x_scale, &mut y_scale);
    cairo::ffi::cairo_scale(cr, 1. / x_scale as f64, -1. / y_scale as f64);

    ffi::hb_font_get_glyph_shape(font, glyph, cairo_draw_funcs(), cr as *mut _);
    cairo::ffi::cairo_fill(cr);

    cairo::ffi::STATUS_SUCCESS
}

unsafe extern "C" fn _hb_blob_read_func(
    src: *mut ffi::hb_blob_t,
    data: *mut u8,
    length: u32,
) -> cairo::ffi::cairo_status_t {
    let blob_length = ffi::hb_blob_get_length(src);
    if blob_length < length {
        return cairo::ffi::STATUS_READ_ERROR;
    }
    let mut len = length;
    std::ptr::copy_nonoverlapping(
        ffi::hb_blob_get_data(src, &mut len),
        data as *mut i8,
        length as usize,
    );
    *src = *ffi::hb_blob_create_sub_blob(src, length, blob_length - length);
    cairo::ffi::STATUS_SUCCESS
}

// unsafe extern "C" fn render_color_glyph_png(
//     scaled_font: *mut cairo::ffi::cairo_scaled_font_t,
//     glyph: u32,
//     cr: *mut cairo::ffi::cairo_t,
//     extents: *mut cairo::ffi::cairo_text_extents_t,
// ) -> cairo::ffi::cairo_status_t {
//     let font_face = cairo::ffi::cairo_scaled_font_get_font_face(scaled_font);
//     let font = cairo::ffi::cairo_font_face_get_user_data(
//         font_face,
//         &_hb_font_cairo_user_data_key as *const _,
//     );
//     let font: *mut ffi::hb_font_t = font as _;
//     let blob = ffi::hb_ot_color_glyph_reference_png(font, glyph);
//     if blob == ffi::hb_blob_get_empty() {
//         return cairo::ffi::STATUS_USER_FONT_NOT_IMPLEMENTED;
//     }
//
//     let mut x_scale: ffi::hb_position_t = 0;
//     let mut y_scale: ffi::hb_position_t = 0;
//     ffi::hb_font_get_scale(font, &mut x_scale, &mut y_scale);
//     let cr = cairo::Context::from_raw_none(cr);
//     cr.scale(1. / x_scale as f64, -1. / y_scale as f64);
//
//     let surface = cairo::ffi::cairo_image_surface_create_from_png_stream(
//         Some(std::mem::transmute(_hb_blob_read_func)),
//         blob as _,
//     );
//     let surface = cairo::ImageSurface::from_raw_full(surface).unwrap();
//     ffi::hb_blob_destroy(blob);
//
//     let width = surface.width();
//     let height = surface.height();
//
//     let mut hb_extents = MaybeUninit::uninit();
//     if ffi::hb_font_get_glyph_extents(font, glyph, hb_extents.as_mut_ptr()) == 0 {
//         return cairo::ffi::STATUS_USER_FONT_NOT_IMPLEMENTED;
//     }
//
//     let hb_extents = hb_extents.assume_init();
//
//     let pattern = cairo::SurfacePattern::create(&surface);
//     pattern.set_extend(cairo::Extend::Pad);
//
//     let matrix = cairo::Matrix::new(width as f64, 0., 0., height as f64, 0., 0.);
//     pattern.set_matrix(matrix);
//
//     cr.translate(hb_extents.x_bearing as f64, hb_extents.y_bearing as f64);
//     cr.scale(hb_extents.width as f64, hb_extents.height as f64);
//     cr.set_source(&pattern);
//
//     cr.rectangle(0., 0., 1., 1.);
//     cr.fill();
//
//     cairo::ffi::STATUS_SUCCESS
// }

fn render_color_glyph_png(
    scaled_font: &cairo::ScaledFont,
    glyph: std::os::raw::c_ulong,
    cr: &cairo::Context,
    _extents: &mut cairo::TextExtents,
) -> cairo::Result<()> {
    let font_face = scaled_font.font_face();
    let font = font_face.user_data(&HB_CAIRO_FONT_KEY).unwrap();
    let font: *mut ffi::hb_font_t = font.as_ptr();
    let blob = unsafe {
        let blob = ffi::hb_ot_color_glyph_reference_png(font, glyph as _);
        if blob == ffi::hb_blob_get_empty() {
            return Err(cairo::Error::UserFontNotImplemented);
        }
        blob
    };

    let mut x_scale: ffi::hb_position_t = 0;
    let mut y_scale: ffi::hb_position_t = 0;
    unsafe {
        ffi::hb_font_get_scale(font, &mut x_scale, &mut y_scale);
    }
    cr.scale(1. / x_scale as f64, -1. / y_scale as f64);

    let surface = unsafe {
        let surface = cairo::ffi::cairo_image_surface_create_from_png_stream(
            Some(std::mem::transmute(_hb_blob_read_func as *const ())),
            blob as _,
        );

        ffi::hb_blob_destroy(blob);

        cairo::ImageSurface::from_raw_full(surface).unwrap()
    };

    let width = surface.width();
    let height = surface.height();

    let hb_extents = unsafe {
        let mut hb_extents = MaybeUninit::uninit();
        if ffi::hb_font_get_glyph_extents(font, glyph as _, hb_extents.as_mut_ptr()) == 0 {
            return Err(cairo::Error::UserFontNotImplemented);
        }

        hb_extents.assume_init()
    };

    let pattern = cairo::SurfacePattern::create(&surface);
    pattern.set_extend(cairo::Extend::Pad);

    let matrix = cairo::Matrix::new(width as f64, 0., 0., height as f64, 0., 0.);
    pattern.set_matrix(matrix);

    cr.translate(hb_extents.x_bearing as f64, hb_extents.y_bearing as f64);
    cr.scale(hb_extents.width as f64, hb_extents.height as f64);
    cr.set_source(&pattern);

    cr.rectangle(0., 0., 1., 1.);
    cr.fill();

    Ok(())
}

// unsafe extern "C" fn render_color_glyph_layers(
//     scaled_font: *mut cairo::ffi::cairo_scaled_font_t,
//     glyph: u32,
//     cr: *mut cairo::ffi::cairo_t,
//     extents: *mut cairo::ffi::cairo_text_extents_t,
// ) -> cairo::ffi::cairo_status_t {
//     let font_face = cairo::ffi::cairo_scaled_font_get_font_face(scaled_font);
//     let font = cairo::ffi::cairo_font_face_get_user_data(
//         font_face,
//         &_hb_font_cairo_user_data_key as *const _,
//     );
//     let font: *mut ffi::hb_font_t = font as _;
//
//     let face = ffi::hb_font_get_face(font);
//
//     let count = ffi::hb_ot_color_glyph_get_layers(
//         face,
//         glyph,
//         0,
//         std::ptr::null_mut(),
//         std::ptr::null_mut(),
//     );
//     if count == 0 {
//         return cairo::ffi::STATUS_USER_FONT_NOT_IMPLEMENTED;
//     }
//
//     let cr = cairo::Context::from_raw_none(cr);
//
//     let scaled_font = cairo::ScaledFont::from_raw_none(scaled_font);
//     let layers: [MaybeUninit<ffi::hb_ot_color_layer_t>; 16] = [MaybeUninit::uninit(); 16];
//     let offset = 0;
//     let mut len: usize;
//     loop {
//         let mut color = MaybeUninit::uninit();
//         len = layers.len(); // FIXME: ???
//         ffi::hb_ot_color_glyph_get_layers(
//             face,
//             glyph,
//             offset,
//             &mut len as *mut _ as *mut u32,
//             layers.as_mut_ptr() as _,
//         );
//         for i in 0..len {
//             let clen = 1;
//             let color_index = layers[i].assume_init_ref().color_index;
//             let is_foreground = color_index == 65535;
//             if !is_foreground {
//                 ffi::hb_ot_color_palette_get_colors(
//                     face,
//                     0,
//                     color_index,
//                     &mut clen,
//                     color.as_mut_ptr(),
//                 );
//                 if clen < 1 {
//                     continue;
//                 }
//             }
//             cr.save();
//
//             if !is_foreground {
//                 let color = color.assume_init();
//                 cr.set_source_rgba(
//                     ffi::hb_color_get_red(color) as f64 / 255.,
//                     ffi::hb_color_get_green(color) as f64 / 255.,
//                     ffi::hb_color_get_blue(color) as f64 / 255.,
//                     ffi::hb_color_get_alpha(color) as f64 / 255.,
//                 )
//             }
//             if let Err(err) = render_glyph(
//                 &scaled_font,
//                 layers[i].assume_init_ref().glyph as _,
//                 &cr,
//                 &mut *(extents as cairo::TextExtents),
//             ) {
//                 err.into()
//             }
//             cr.restore()?;
//         }
//         if len as usize != layers.len() {
//             break;
//         }
//     }
//     cairo::ffi::STATUS_SUCCESS
// }
fn render_color_glyph_layers(
    scaled_font: &cairo::ScaledFont,
    glyph: std::os::raw::c_ulong,
    cr: &cairo::Context,
    extents: &mut cairo::TextExtents,
) -> cairo::Result<()> {
    let font_face = scaled_font.font_face();
    let font = font_face.user_data(&HB_CAIRO_FONT_KEY).unwrap();
    let font: *mut ffi::hb_font_t = font.as_ptr();

    let face = unsafe { ffi::hb_font_get_face(font) };

    let count = unsafe {
        ffi::hb_ot_color_glyph_get_layers(
            face,
            glyph as _,
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if count == 0 {
        return Err(cairo::Error::UserFontNotImplemented);
    }

    let mut layers: [ffi::hb_ot_color_layer_t; 16] = [ffi::hb_ot_color_layer_t {
        color_index: 0,
        glyph: 0,
    }; 16];
    let offset = 0;
    let mut len: u32;
    loop {
        len = layers.len() as u32; // FIXME: ???
        unsafe {
            ffi::hb_ot_color_glyph_get_layers(
                face,
                glyph as _,
                offset,
                &mut len as *mut u32,
                layers.as_mut_ptr() as _,
            );
        }
        for i in 0..len {
            let mut color: ffi::hb_color_t = 0;
            let mut clen = 1;
            let color_index = layers[i as usize].color_index;
            let is_foreground = color_index == 65535;
            if !is_foreground {
                unsafe {
                    ffi::hb_ot_color_palette_get_colors(
                        face,
                        0,
                        color_index,
                        &mut clen,
                        &mut color as *mut _,
                    );
                }
                if clen < 1 {
                    continue;
                }
            }
            cr.save();

            if !is_foreground {
                unsafe {
                    cr.set_source_rgba(
                        ffi::hb_color_get_red(color) as f64 / 255.,
                        ffi::hb_color_get_green(color) as f64 / 255.,
                        ffi::hb_color_get_blue(color) as f64 / 255.,
                        ffi::hb_color_get_alpha(color) as f64 / 255.,
                    )
                }
            }
            render_glyph(&scaled_font, layers[i as usize].glyph as _, &cr, extents)?;
            cr.restore();
        }
        if len as usize != layers.len() {
            break;
        }
    }
    Ok(())
}

// unsafe extern "C" fn render_color_glyph(
//     scaled_font: *mut cairo::ffi::cairo_scaled_font_t,
//     glyph: u32,
//     cr: *mut cairo::ffi::cairo_t,
//     extents: *mut cairo::ffi::cairo_text_extents_t,
// ) -> cairo::ffi::cairo_status_t {
//     let mut ret = cairo::ffi::STATUS_USER_FONT_NOT_IMPLEMENTED;
//
//     ret = render_color_glyph_png(scaled_font, glyph, cr, extents);
//     if ret != cairo::ffi::STATUS_USER_FONT_NOT_IMPLEMENTED {
//         return ret;
//     }
//
//     ret = render_color_glyph_layers(scaled_font, glyph, cr, extents);
//     if ret != cairo::ffi::STATUS_USER_FONT_NOT_IMPLEMENTED {
//         return ret;
//     }
//
//     let extents = &mut *(extents as cairo::TextExtents);
//     render_glyph(scaled_font, glyph, cr, extents)
// }
fn render_color_glyph(
    scaled_font: &cairo::ScaledFont,
    glyph: std::os::raw::c_ulong,
    cr: &cairo::Context,
    extents: &mut cairo::TextExtents,
) -> cairo::Result<()> {
    let ret = render_color_glyph_png(scaled_font, glyph, cr, extents);
    if ret != Err(cairo::Error::UserFontNotImplemented) {
        return ret;
    }

    let ret = render_color_glyph_layers(scaled_font, glyph, cr, extents);
    if ret != Err(cairo::Error::UserFontNotImplemented) {
        return ret;
    }

    render_glyph(scaled_font, glyph, cr, extents)
}

fn create_user_font_face(font_opts: &FontOptions) -> anyhow::Result<cairo::UserFontFace> {
    let cairo_face = cairo::UserFontFace::create()?;
    let hb_font = font_opts.font();
    cairo_face.set_user_data(&HB_CAIRO_FONT_KEY, Rc::new(HbFont(hb_font)));
    cairo_face.set_render_glyph_func(render_glyph);
    unsafe {
        let face = ffi::hb_font_get_face(hb_font);
        if ffi::hb_ot_color_has_png(face) == 1 || ffi::hb_ot_color_has_layers(face) == 1 {
            cairo_face.set_render_color_glyph_func(render_color_glyph);
        }
    }
    Ok(cairo_face)
}

fn create_ft_font_face(font_opts: &FontOptions) -> anyhow::Result<cairo::FontFace> {
    todo!()
}

fn use_hb_draw() -> bool {
    let env = std::env::var("HB_DRAW");
    if let Ok(env) = env.as_ref() {
        match env.as_str() {
            "1" | "true" => true,
            _ => false,
        }
    } else {
        let cr_version = cairo::Version::new();
        cr_version.major() >= 1 && cr_version.minor() >= 17 && cr_version.micro() >= 5
    }
}

pub fn create_scaled_font(font_opts: &FontOptions) -> anyhow::Result<cairo::ScaledFont> {
    let font = unsafe { ffi::hb_font_reference(font_opts.font()) };

    let font_size = font_opts.font_size.unwrap_or_default();

    let ctm = cairo::Matrix::identity();
    let mut font_matrix = cairo::Matrix::default();
    font_matrix.scale(font_size.x as _, font_size.y as _);

    if use_hb_draw() {
        font_matrix.set_xy((-font_opts.slant * font_size.x) as f64);
    }
    let mut font_options = cairo::FontOptions::new()?;
    font_options.set_hint_style(cairo::HintStyle::None);
    font_options.set_hint_metrics(cairo::HintMetrics::Off);

    let scaled_font = if use_hb_draw() {
        let face = create_user_font_face(font_opts)?;
        cairo::ScaledFont::new(&face, &font_matrix, &ctm, &font_options)?
    } else {
        let face = create_ft_font_face(font_opts)?;
        cairo::ScaledFont::new(&face, &font_matrix, &ctm, &font_options)?
    };

    scaled_font.set_user_data(&HB_CAIRO_FONT_KEY, Rc::new(HbFont(font)));

    Ok(scaled_font)
}

static HB_CAIRO_FONT_KEY: cairo::UserDataKey<HbFont> = cairo::UserDataKey::new();

pub trait ScaledFontExt {
    fn has_data(&self) -> bool;
    fn has_color(&self) -> bool;
}

impl ScaledFontExt for cairo::ScaledFont {
    fn has_data(&self) -> bool {
        self.user_data(&HB_CAIRO_FONT_KEY).is_some()
    }
    fn has_color(&self) -> bool {
        if let Some(hbfont) = self.font_face().user_data(&HB_CAIRO_FONT_KEY) {
            unsafe {
                let hbface = ffi::hb_font_get_face(hbfont.as_ptr());
                ffi::hb_ot_color_has_png(hbface) == 1 || ffi::hb_ot_color_has_layers(hbface) == 1
            }
        } else {
            false
        }
    }
}

// pub trait UserFontFaceExt {
//     #[inline(always)]
//     fn has_data(&self) -> bool;
//     // fn has_color(&self) -> bool;
// }
//
// impl UserFontFaceExt for cairo::UserFontFace {
//     #[inline(always)]
//     fn has_data(&self) -> bool {
//         self.user_data(&HB_CAIRO_FONT_KEY).is_some()
//     }
//
//     #[inline(always)]
//     fn has_color(&self) -> bool {
//         let font = self.user_data(&HB_CAIRO_FONT_KEY).unwrap();
//         unsafe {
//             let face = ffi::hb_font_get_face(font.as_ptr());
//             ffi::hb_ot_color_has_png(face) == 1 || ffi::hb_ot_color_has_layers(face) == 1
//         }
//     }
// }

enum ImageProtocol {
    None = 0,
    Item2 = 1,
    Kitty = 2,
}

enum SupportedFormat {
    PNG,
}

pub fn create_cairo_context(
    w: f64,
    h: f64,
    view_opts: &ViewOptions,
    out_opts: &OutputAndFormatOptions,
    content: cairo::Content,
) -> anyhow::Result<cairo::Context> {
    let protocol = ImageProtocol::None;

    let extension = if let Some(ext) = out_opts.output_format {
        ext
    } else {
        OutputFormat::PNG
    };

    let mut br = 0;
    let mut bg = 0;
    let mut bb = 0;
    let mut ba = 255;
    let mut color = view_opts.background.as_str();
    if color.starts_with("#") {
        color = &color[1..];
    }
    br = u8::from_str_radix(&color[0..2], 16).unwrap();
    bg = u8::from_str_radix(&color[2..4], 16).unwrap();
    bb = u8::from_str_radix(&color[4..6], 16).unwrap();
    if color.len() > 6 {
        ba = u8::from_str_radix(&color[6..8], 16).unwrap();
    }

    let mut color = view_opts.foreground.as_str();
    if color.starts_with("#") {
        color = &color[1..];
    }
    let mut fr = 0;
    let mut fg = 0;
    let mut fb = 0;
    let mut fa = 255;
    fr = u8::from_str_radix(&color[0..2], 16).unwrap();
    fg = u8::from_str_radix(&color[2..4], 16).unwrap();
    fb = u8::from_str_radix(&color[4..6], 16).unwrap();
    if color.len() > 6 {
        fa = u8::from_str_radix(&color[6..8], 16).unwrap();
    }

    let mut content = content;
    if content == cairo::Content::Alpha {
        if view_opts.annotate || br != bg || bg != bb || fr != fg || fg != fb {
            content = cairo::Content::Color;
        }
    }
    if ba != 255 {
        content = cairo::Content::ColorAlpha;
    }

    let surface = match extension {
        OutputFormat::PNG => {
            //
            create_png_surface_for_stream()
        }
        _ => unimplemented!(),
    };

    let cr = cairo::Context::new(&surface)?;
    // surface.content

    todo!()
}

fn create_png_surface_for_stream() -> cairo::Surface {
    todo!()
}
