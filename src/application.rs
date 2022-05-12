pub trait Application {
    unsafe fn readline(&mut self) -> &str;
}
