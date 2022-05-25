use std::{mem::MaybeUninit, num, rc::Rc};

use harfbuzz_sys as ffi;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Copy)]
#[repr(transparent)]
struct GlyphInfo(ffi::hb_glyph_info_t);
impl GlyphInfo {
    fn codepoint(&self) -> u32 {
        self.0.codepoint
    }
    fn cluster(&self) -> u32 {
        self.0.cluster
    }
    fn mask(&self) -> u32 {
        self.0.mask
    }
}

impl std::fmt::Debug for GlyphInfo {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.debug_struct("GlyphInfo")
            .field("codepoint", &self.0.codepoint)
            .field("cluster", &self.0.cluster)
            .field("mask", &self.0.mask)
            // .field("var1", &self.0.var1)
            // .field("var2", &self.0.var2)
            .finish()
    }
}

#[derive(Clone, Copy)]
#[repr(transparent)]
struct GlyphPosition(ffi::hb_glyph_position_t);
impl GlyphPosition {
    fn x_advance(&self) -> i32 {
        self.0.x_advance
    }
    fn y_advance(&self) -> i32 {
        self.0.y_advance
    }
    fn x_offset(&self) -> i32 {
        self.0.x_offset
    }
    fn y_offset(&self) -> i32 {
        self.0.y_offset
    }
}

impl std::fmt::Debug for GlyphPosition {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.debug_struct("GlyphPosition")
            .field("x_advance", &self.0.x_advance)
            .field("y_advance", &self.0.y_advance)
            .field("x_offset", &self.0.x_offset)
            .field("y_offset", &self.0.y_offset)
            .finish()
    }
}

fn main() -> anyhow::Result<()> {
    let text = std::fs::read_to_string("src/bin/hb-view.rs").unwrap();

    let scale_bits = 6;

    // Init the library
    let lib = freetype::Library::init().unwrap();
    // Load a font face
    let mut ft_face = lib.new_face("/home/songww/.local/share/fonts/LXGWWenKai-Regular.ttf", 0)?;

    let cairo_face =
        cairo::FontFace::create_from_ft(&ft_face).map_err(|err| anyhow::anyhow!(err))?;

    // let font_data =
    //     std::fs::read("/home/songww/.local/share/fonts/LXGWWenKai-Regular.ttf").unwrap();
    //
    // let blob = unsafe {
    //     ffi::hb_blob_create_or_fail(
    //         font_data.as_ptr() as *const _,
    //         font_data.len() as _,
    //         ffi::HB_MEMORY_MODE_READONLY,
    //         std::ptr::null_mut(),
    //         None,
    //     )
    // };
    // anyhow::ensure!(!blob.is_null(), "Couldn't create hb blob.");
    //
    // let face = unsafe { ffi::hb_face_create(blob, 0) };
    // anyhow::ensure!(
    //     unsafe { ffi::hb_face_get_empty() } != face,
    //     "Couldn't create hb face."
    // );
    ft_face.set_char_size(0, 12, 0, 0).unwrap();
    ft_face.reference()?;
    let font = unsafe { ffi::hb_ft_font_create_referenced(ft_face.raw_mut() as *mut _) };

    // let font = unsafe { ffi::hb_font_create(face) };
    anyhow::ensure!(
        unsafe { ffi::hb_font_get_empty() } != font,
        "Couldn't create hb font."
    );

    let x_scale = libm::scalbnf(12., scale_bits);
    let y_scale = libm::scalbnf(12., scale_bits);

    dbg!(x_scale, y_scale);

    // unsafe {
    //     ffi::hb_font_set_ppem(font, 0, 0);
    //     ffi::hb_font_set_ptem(font, 0.);
    //     ffi::hb_font_set_scale(font, x_scale as i32, y_scale as i32);
    //     ffi::hb_ot_font_set_funcs(font);
    // }

    let buf = unsafe { ffi::hb_buffer_create() };

    let target = cairo::ImageSurface::create(cairo::Format::ARgb32, 300, 1000).unwrap();
    let cr = cairo::Context::new(&target).unwrap();

    cr.set_scaled_font(&create_scaled_font(&cairo_face));
    // cr.set_scaled_font(&create_user_scaled_font(font));
    // cr.set_font_face(
    //     &cairo::FontFace::toy_create(
    //         "LXGW WenKai",
    //         cairo::FontSlant::Normal,
    //         cairo::FontWeight::Normal,
    //     )
    //     .unwrap(),
    // );

    // cr.select_font_face(
    //     "LXGW WenKai",
    //     cairo::FontSlant::Normal,
    //     cairo::FontWeight::Normal,
    // );
    // cr.set_font_size(12.);

    let direction = ffi::HB_DIRECTION_LTR;
    let is_backward = hb_util::hb_direction_is_backward(direction);

    let mut hb_extents = MaybeUninit::uninit();
    unsafe { ffi::hb_font_get_extents_for_direction(font, direction, hb_extents.as_mut_ptr()) };

    let extents: ffi::hb_font_extents_t = unsafe { hb_extents.assume_init() };

    let is_vertical = hb_util::hb_direction_is_vertical(direction);
    dbg!(is_vertical);
    dbg!(extents);

    let ascent = libm::scalbn(extents.ascender as f64, scale_bits);
    let descent = libm::scalbn(extents.descender as f64, scale_bits);
    let line_gap = libm::scalbn(extents.line_gap as f64, scale_bits);
    let leading = ascent + descent + line_gap;

    // cr.translate(0., ascent);
    // cr.translate(0., -leading);

    dbg!(ascent, descent, line_gap, leading);

    cr.set_source_rgb(1., 1., 1.);
    cr.paint().unwrap();
    cr.set_source_rgb(0.1, 0.1, 0.1);
    // println!("{}", &text);
    // cr.move_to(5., 20.);

    for (no, line) in text.lines().enumerate() {
        cr.move_to(1., 12. * no as f64 + 12.);
        dbg!(line);
        let graphemes: Vec<_> = line.grapheme_indices(true).collect();
        // dbg!(&graphemes);
        let s = std::ffi::CString::new(line).unwrap();
        unsafe {
            println!("add {:?}", s);
            ffi::hb_buffer_reset(buf);
            ffi::hb_buffer_add_utf8(buf, s.as_ptr(), -1, 0, -1);
            ffi::hb_buffer_set_direction(buf, direction);
            ffi::hb_buffer_guess_segment_properties(buf);
            ffi::hb_shape(font, buf, std::ptr::null(), 0);

            let num_glyphs = ffi::hb_buffer_get_length(buf);

            let glyph_count: u32 = num_glyphs;
            let glyph_info = ffi::hb_buffer_get_glyph_infos(buf, std::ptr::null_mut());
            let glyph_pos = ffi::hb_buffer_get_glyph_positions(buf, std::ptr::null_mut());

            let mut glyphs: Vec<MaybeUninit<ffi::hb_glyph_info_t>> =
                vec![MaybeUninit::uninit(); glyph_count as usize];
            std::ptr::copy_nonoverlapping(
                glyph_info,
                glyphs.as_mut_ptr() as *mut _,
                glyph_count as usize,
            );
            let glyph_infos: Vec<GlyphInfo> = std::mem::transmute(glyphs);
            // dbg!(&glyphs);

            let mut positions: Vec<MaybeUninit<ffi::hb_glyph_position_t>> =
                vec![MaybeUninit::uninit(); glyph_count as usize];
            std::ptr::copy(
                glyph_pos,
                positions.as_mut_ptr() as *mut _,
                glyph_count as usize,
            );
            let glyph_positions: Vec<ffi::hb_glyph_position_t> = std::mem::transmute(positions);
            // dbg!(&glyph_positions);

            // let mut cursor_x: ffi::hb_position_t = 0;
            // let mut cursor_y: ffi::hb_position_t = 0;

            assert_eq!(glyph_count, num_glyphs);

            let default_glyph = cairo::Glyph::new(0, 0., 0.);
            let mut glyphs: Vec<cairo::Glyph> = vec![default_glyph; glyph_count as usize + 1];

            let mut cluster_count = 0;
            if glyph_count > 0 {
                cluster_count = 1;
            }
            for i in 1..glyph_count as usize {
                let pos = glyph_infos.get_unchecked(i);
                let ppos = glyph_infos.get_unchecked(i - 1);
                if pos.cluster() != ppos.cluster() {
                    cluster_count += 1;
                }
            }

            let default_cluster = cairo::TextCluster::new(0, 0);
            let mut clusters: Vec<cairo::TextCluster> =
                vec![default_cluster; cluster_count as usize];

            let mut x: ffi::hb_position_t = 0;
            let mut y: ffi::hb_position_t = 0;

            for i in 0..glyph_count as usize {
                let pos = glyph_positions.get_unchecked(i);
                let glyph = &mut glyphs[i];
                glyph.set_index(glyph_infos.get_unchecked(i).codepoint() as _);
                glyph.set_x(libm::scalbn((pos.x_offset + x) as f64, scale_bits));
                glyph.set_y(libm::scalbn((-pos.y_offset + y) as f64, scale_bits));

                x += pos.x_advance;
                y += -pos.y_advance;
            }

            let glyph = &mut glyphs[glyph_count as usize];
            glyph.set_index(u64::MAX);
            glyph.set_x(x as _);
            glyph.set_y(y as _);

            let cluster_flags = if is_backward {
                cairo::TextClusterFlags::Backward
            } else {
                cairo::TextClusterFlags::None
            };

            dbg!(is_backward);

            dbg!(cluster_count);
            dbg!(glyph_count);

            if cluster_count > 0 {
                // TODO: backward check
                let mut cluster = 0;

                let text_cluster = &mut clusters[cluster];
                text_cluster.set_num_glyphs(text_cluster.num_bytes() + 1);

                let mut iter = graphemes.iter();

                let mut total_bytes = 0;

                for i in 1..num_glyphs as usize {
                    let hb_glyph = glyph_infos.get_unchecked(i);
                    let hb_glyph_l = glyph_infos.get_unchecked(i - 1);
                    if hb_glyph.cluster() != hb_glyph_l.cluster() {
                        assert!(hb_glyph.cluster() > hb_glyph_l.cluster());
                        let num_bytes = hb_glyph.cluster() - hb_glyph_l.cluster();
                        let v = iter.next().unwrap();
                        assert!(v.0 == hb_glyph_l.cluster() as usize);
                        // dbg!(1, cluster, num_bytes);
                        clusters[cluster].set_num_bytes(num_bytes as i32);
                        total_bytes += num_bytes;
                        cluster += 1;
                    }
                    let num_glyphs = clusters[cluster].num_glyphs() + 1;
                    clusters[cluster].set_num_glyphs(num_glyphs);
                    // dbg!(2, cluster, num_glyphs);
                }
                // dbg!(3, cluster);
                clusters[cluster].set_num_bytes(line.len() as i32 - total_bytes as i32);
            }

            assert_eq!(glyph_count as usize + 1, glyphs.len());

            // dbg!(&clusters);
            // dbg!(&glyphs);
            // dbg!(cluster_count, &clusters.len(), &glyphs.len());
            // cr.set_source_rgb(0.4, 0.4, 0.4);
            cr.show_text_glyphs(
                &line,
                &glyphs[..glyph_count as usize],
                &clusters,
                cluster_flags,
            )
            .unwrap();
            // cr.set_source_rgb(0.6, 0.6, 0.6);
            // cr.show_glyphs(&glyphs[..glyph_count as usize]).unwrap();
            // cr.set_source_rgb(0.2, 0.6, 0.8);
            // cr.show_text(&line).unwrap();
        }
    }
    unsafe {
        ffi::hb_buffer_destroy(buf);
        ffi::hb_font_destroy(font);
        // ffi::hb_face_destroy(face);
        // ffi::hb_blob_destroy(blob);
    }

    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .append(false)
        .truncate(true)
        .open("/tmp/emoji.png")
        .unwrap();
    target.write_to_png(&mut f).unwrap();
    // let cfgs = viuer::Config::default();
    // viuer::print_from_file("/tmp/emoji.png", &cfgs).unwrap();
    Ok(())
}

fn create_scaled_font(font_face: &cairo::FontFace) -> cairo::ScaledFont {
    // let font = unsafe { ffi::hb_font_reference(font) };

    // let font_face = create_ft_font_face(font).unwrap();

    let ctm = cairo::Matrix::identity();
    let mut font_matrix = cairo::Matrix::default();
    font_matrix.scale(128., 128.);

    let mut options = cairo::FontOptions::new().unwrap();
    options.set_hint_style(cairo::HintStyle::None);
    options.set_hint_metrics(cairo::HintMetrics::Off);

    let scaled_font = cairo::ScaledFont::new(&font_face, &font_matrix, &ctm, &options).unwrap();

    // set user data

    scaled_font
}

fn create_user_scaled_font(font: *mut ffi::hb_font_t) -> cairo::ScaledFont {
    let font = unsafe { ffi::hb_font_reference(font) };

    let font_face = create_user_font_face(font).unwrap();

    let ctm = cairo::Matrix::identity();
    let mut font_matrix = cairo::Matrix::default();
    font_matrix.scale(128., 128.);

    let mut options = cairo::FontOptions::new().unwrap();
    options.set_hint_style(cairo::HintStyle::None);
    options.set_hint_metrics(cairo::HintMetrics::Off);

    let scaled_font = cairo::ScaledFont::new(&font_face, &font_matrix, &ctm, &options).unwrap();

    // set user data

    scaled_font
}

fn create_user_font_face(font: *mut ffi::hb_font_t) -> anyhow::Result<cairo::UserFontFace> {
    let cairo_face = cairo::UserFontFace::create()?;
    let rcfont = Rc::new(unsafe { hb_util::HbFont::from_raw(font) });
    cairo_face.set_user_data(&hb_util::HB_CAIRO_FONT_KEY, rcfont)?;
    cairo_face.set_render_glyph_func(hb_util::render_glyph);
    unsafe {
        let face = ffi::hb_font_get_face(font);
        if ffi::hb_ot_color_has_png(face) == 1 || ffi::hb_ot_color_has_layers(face) == 1 {
            cairo_face.set_render_color_glyph_func(hb_util::render_color_glyph);
        }
    }
    Ok(cairo_face)
}
