use std::ffi::CString;

fn main() {
    let font_config = fontconfig::Fontconfig::new().unwrap();
    let mut pattern = fontconfig::Pattern::new(&font_config);
    pattern.add_string(
        &CString::new("family").unwrap(),
        &CString::new("Cascadia Code").unwrap(),
    );

    let sets = pattern.font_sort();

    // sets.print();
    for pattern in sets.iter() {
        // pattern.print();
        println!("-------------------------> font");
        println!("face index: {:?}", pattern.face_index());
        println!("name: {:?}", pattern.name());
        println!(
            "full name: {:?}",
            pattern.get_string(&CString::new("fullname").unwrap())
        );
        println!(
            "style: {:?}",
            pattern.get_string(&CString::new("style").unwrap())
        );
        println!("slant: {:?}", pattern.slant());
        println!("width: {:?}", pattern.width());
        println!("weight: {:?}", pattern.weight());
    }
}
