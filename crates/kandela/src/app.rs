#[cxx::bridge]
mod ffi {
    unsafe extern "C++" {
        include!("app.h");
        fn run_app(start_hidden: bool, single_instance: bool) -> i32;
    }
}

pub use ffi::run_app;
