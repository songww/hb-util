use std::mem::MaybeUninit;

use harfbuzz_sys as ffi;

fn main() -> anyhow::Result<()> {
    let text = std::fs::read_to_string("emoji.txt").unwrap();

    let font_data = std::fs::read("/usr/share/fonts/truetype/noto/NotoColorEmoji.ttf").unwrap();

    let blob = unsafe {
        ffi::hb_blob_create_or_fail(
            font_data.as_ptr() as *const _,
            font_data.len() as _,
            ffi::HB_MEMORY_MODE_READONLY,
            std::ptr::null_mut(),
            None,
        )
    };
    anyhow::ensure!(!blob.is_null(), "Couldn't create hb blob.");

    let face = unsafe { ffi::hb_face_create(blob, 0) };
    anyhow::ensure!(
        unsafe { ffi::hb_face_get_empty() } != face,
        "Couldn't create hb face."
    );

    let font = unsafe { ffi::hb_font_create(face) };
    anyhow::ensure!(
        unsafe { ffi::hb_font_get_empty() } != font,
        "Couldn't create hb font."
    );

    unsafe {
        ffi::hb_font_set_ppem(font, 128, 128);
        ffi::hb_ot_font_set_funcs(font);
    }

    let buf = unsafe { ffi::hb_buffer_create() };

    let mut target = cairo::ImageSurface::create(cairo::Format::ARgb32, 600, 800).unwrap();
    let cr = cairo::Context::new(&target).unwrap();

    for line in text.lines() {
        let s = std::ffi::CString::new(line).unwrap();
        unsafe {
            println!("add {}", line);
            ffi::hb_buffer_reset(buf);
            ffi::hb_buffer_add_utf8(buf, s.as_ptr(), -1, 0, -1);
            ffi::hb_buffer_set_direction(buf, ffi::HB_DIRECTION_LTR);
            ffi::hb_buffer_guess_segment_properties(buf);
            ffi::hb_shape(font, buf, std::ptr::null(), 0);

            let num_glyphs = ffi::hb_buffer_get_length(buf);

            let mut glyph_count: u32 = num_glyphs;
            let glyph_info = ffi::hb_buffer_get_glyph_infos(buf, &mut glyph_count as *mut _);
            let glyph_pos = ffi::hb_buffer_get_glyph_positions(buf, &mut glyph_count as *mut _);

            let mut cursor_x: ffi::hb_position_t = 0;
            let mut cursor_y: ffi::hb_position_t = 0;

            assert_eq!(glyph_count, num_glyphs);

            let default_glyph = cairo::ffi::cairo_glyph_t {
                index: 0,
                x: 0.,
                y: 0.,
            };
            let default_glyph: cairo::Glyph = std::mem::transmute(default_glyph);

            let mut glyphs: Vec<cairo::Glyph> = vec![default_glyph; glyph_count as usize + 1];

            let mut cluster_count = 0;
            if glyph_count > 0 {
                cluster_count = 1;
            }
            for i in 1..glyph_count as isize {
                let pos = glyph_info.offset(i);
                let ppos = glyph_info.offset(i - 1);
                if (*pos).cluster != (*ppos).cluster {
                    cluster_count += 1;
                }
            }

            let default_cluster = cairo::ffi::cairo_text_cluster_t {
                num_bytes: 0,
                num_glyphs: 0,
            };
            let default_cluster: cairo::TextCluster = std::mem::transmute(default_cluster);

            let mut clusters: Vec<cairo::TextCluster> =
                vec![default_cluster; cluster_count as usize];

            let mut x: ffi::hb_position_t = 0;
            let mut y: ffi::hb_position_t = 0;

            for i in 0..glyph_count as usize {
                let pos = glyph_pos.offset(i as isize);
                let glyph = &mut glyphs[i] as *mut _ as *mut cairo::ffi::cairo_glyph_t;
                (*glyph).index = (*glyph_info.offset(i as isize)).codepoint as _;
                (*glyph).x = ((*pos).x_offset + x) as f64;
                (*glyph).y = ((*pos).y_offset + y) as f64;

                x += (*pos).x_advance;
                y += (*pos).y_advance;
            }

            let glyph =
                &mut glyphs[glyph_count as usize] as *mut _ as *mut cairo::ffi::cairo_glyph_t;
            (*glyph).index = u64::MAX;
            (*glyph).x = x as _;
            (*glyph).y = y as _;

            let cluster_flags = cairo::TextClusterFlags::None;

            if cluster_count > 0 {
                // TODO: backward check
                let mut cluster = 0;

                let chars: Vec<_> = line.chars().enumerate().collect();

                let text_cluster =
                    &mut clusters[cluster] as *mut _ as *mut cairo::ffi::cairo_text_cluster_t;
                (*text_cluster).num_glyphs += 1;

                let mut iter = chars.iter();

                let mut start = &chars[0];
                let mut end = &chars[0];

                let mut num_bytes = 0;

                for i in 1..num_glyphs as isize {
                    let hb_glyph = glyph_info.offset(i);
                    let hb_glyph_l = glyph_info.offset(i - 1);
                    if (*hb_glyph).cluster != (*hb_glyph_l).cluster {
                        assert!((*hb_glyph).cluster > (*hb_glyph_l).cluster);
                        for _ in (*hb_glyph).cluster..(*hb_glyph_l).cluster {
                            let v = iter.next().unwrap();
                            num_bytes += v.1.len_utf8();
                        }
                        end = iter.next().unwrap();
                        num_bytes += end.1.len_utf8();
                        let text_cluster = &mut clusters[cluster] as *mut _
                            as *mut cairo::ffi::cairo_text_cluster_t;
                        (*text_cluster).num_bytes = num_bytes as i32;
                        num_bytes = 0;
                        cluster += 1;
                        start = end;
                    }
                    let text_cluster =
                        &mut clusters[cluster] as *mut _ as *mut cairo::ffi::cairo_text_cluster_t;
                    (*text_cluster).num_glyphs += 1;
                }
                let text_cluster =
                    &mut clusters[cluster] as *mut _ as *mut cairo::ffi::cairo_text_cluster_t;
                (*text_cluster).num_bytes = iter.map(|v| v.1.len_utf8()).sum::<usize>() as i32;
            }
            // for i in 0..glyph_count {
            //     let glyphid: ffi::hb_codepoint_t = (*glyph_info.offset(i as isize)).codepoint;
            //     let x_offset: ffi::hb_position_t = (*pos).x_offset;
            //     let y_offset = (*pos).y_offset;
            //     let x_advance = (*pos).x_advance;
            //     let y_advance = (*pos).y_advance;
            //     /* draw_glyph(glyphid, cursor_x + x_offset, cursor_y + y_offset); */
            //     cursor_x += x_advance;
            //     cursor_y += y_advance;
            // }
            let glyphs: Vec<cairo::Glyph> = std::mem::transmute(glyphs);
            let clusters: Vec<cairo::TextCluster> = std::mem::transmute(clusters);
            dbg!(&glyphs);
            for cluster in clusters.iter() {
                dbg!(cluster.num_bytes(), cluster.num_glyphs());
            }
            cr.show_text_glyphs(&line, &glyphs, &clusters, cluster_flags)
                .unwrap();
        }
    }
    unsafe {
        ffi::hb_buffer_destroy(buf);
        ffi::hb_font_destroy(font);
        ffi::hb_face_destroy(face);
        ffi::hb_blob_destroy(blob);
    }

    let img = image::RgbaImage::from_vec(600, 800, target.data().unwrap().to_vec()).unwrap();
    let cfgs = viuer::Config::default();
    viuer::print(&img.into(), &cfgs).unwrap();
    Ok(())
}

/*
fn serialize_gr_command(m: u8, payload: &[u8]) -> Vec<u8>{
    let cmd = format!("a=T,f=100,m={}", m);
    let mut ans = Vec::new();
    ans.extend(b"\033_G");
    ans.extend(cmd.as_bytes());
    if !payload.is_empty(){
        ans.push(b';');
        ans.extend(payload);
    }
    ans.extend(b"\033\\");
    ans
}


fn write_chunked(data: &[u8]) {
    let mut data = standard_b64encode(data);
    while !data.is_empty() {
        let (chunk, data) = data.split_at(4096);
        let m = if data {1} else {0};
        // sys.stdout.buffer.write(serialize_gr_command(payload=chunk, m=m, **cmd))
        // sys.stdout.flush()
    }
}
*/

// write_chunked(a='T', f=100, data=f.read())
