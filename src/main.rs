use std::ffi::{c_char, c_void, CString};

use guile_3_sys::{scm_boot_guile, scm_shell};
use log::info;

mod flashkick;
mod jack_adapter;
mod remote_executor;
mod scheme_adapter;
mod simian;
mod track;

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Warn)
        .init();

    let env_args = std::env::args();
    info!("{:?}", env_args);
    let argv: Vec<CString> = env_args.map(CString::new).map(Result::unwrap).collect();
    let args: Vec<*const c_char> = argv.into_iter().map(|arg| arg.as_ptr()).collect();

    unsafe {
        scm_boot_guile(
            args.len() as _,
            args.as_ptr() as _,
            Some(inner_main),
            std::ptr::null_mut(),
        );
    };
}

unsafe extern "C" fn inner_main(_: *mut c_void, argc: i32, argv: *mut *mut i8) {
    scheme_adapter::register_functions();
    info!("Running Scheme shell.");
    scm_shell(argc, argv);
}
