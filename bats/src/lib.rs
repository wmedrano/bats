use std::ffi::{c_char, c_void, CString};

use log::info;

mod bats;
mod jack_adapter;
mod remote_executor;
mod track;

pub mod scheme_lib;

pub fn run_guile_scheme() {
    let args = std::env::args();
    info!("{:?}", args);
    boot_with_shell(args, inner_main);
}

extern "C" fn inner_main(_argc: i32, _argv: *mut *mut i8) {
    unsafe { scheme_lib::init_bats() };
}

pub type SetupFn = extern "C" fn(argc: i32, argv: *mut *mut i8);

/// Boots Guile scheme, runs the setup function and enters the Guile Scheme shell.
pub fn boot_with_shell(args: std::env::Args, setup: SetupFn) {
    boot(args, run_setup_and_shell, setup as *mut c_void);
}

extern "C" fn run_setup_and_shell(setup: *mut c_void, argc: i32, argv: *mut *mut i8) {
    let setup_fn: SetupFn = unsafe { std::mem::transmute(setup) };
    (setup_fn)(argc, argv);
    unsafe { flashkick::ffi::scm_shell(argc, argv) };
}

fn boot(
    args: std::env::Args,
    main: extern "C" fn(closure: *mut c_void, argc: i32, argv: *mut *mut i8),
    closure: *mut c_void,
) {
    let argv: Vec<CString> = args.map(CString::new).map(Result::unwrap).collect();
    let arg_ptrs: Vec<*const c_char> = argv.into_iter().map(|arg| arg.as_ptr()).collect();

    unsafe {
        flashkick::ffi::scm_boot_guile(
            arg_ptrs.len() as _,
            arg_ptrs.as_ptr() as _,
            Some(main),
            closure,
        );
    };
}
