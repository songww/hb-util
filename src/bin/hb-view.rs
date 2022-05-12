/*
use clap::Parser;

use hb_utils::view_cairo::ViewCairo;

const DEFAULT_FONT_SIZE: usize = 256;
const SUBPIXEL_BITS: usize = 6;

fn main() {
    let driver = FontText::<ShapeConsumer<ViewCairo>, DEFAULT_FONT_SIZE, SUBPIXEL_BITS>::new();
    // driver.main();
    let options = hb_utils::options::Options::parse();
    println!("{:?}", options);
}
*/

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
