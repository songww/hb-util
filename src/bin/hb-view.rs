use clap::Parser;

use hb_util as lib;

use lib::font_text::FontText;
use lib::shape_consumer::ShapeConsumer;
use lib::view_cairo::ViewCairo;

#[no_mangle]
pub static DEFAULT_FONT_SIZE: usize = 256;
#[no_mangle]
pub static SUBPIXEL_BITS: usize = 6;

fn main() {
    let mut driver = FontText::<ShapeConsumer<ViewCairo>>::new();
    driver.run();
    // let options = hb_utils::options::Options::parse();
    // println!("{:?}", options);
}

/*
fn main() {
    use harfbuzz_sys as ffi;
    println!("shapers:");
    unsafe {
        let mut hb_shapers = ffi::hb_shape_list_shapers();
        let end = std::ptr::NonNull::dangling().as_ptr();
        println!("non null {:p}", end);
        while !(*hb_shapers).is_null() {
            println!("ptr {:p}", hb_shapers);
            if hb_shapers == end {
                break;
            }
            let cstr = std::ffi::CStr::from_ptr(*hb_shapers as *const _);
            println!("    {:?}", cstr);
            hb_shapers = hb_shapers.offset(1);
        }
    }
}
*/
