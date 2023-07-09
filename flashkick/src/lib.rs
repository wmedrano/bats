use std::ffi::{c_char, c_void, CStr, CString};

pub use guile_3_sys::scm_c_bind_keyword_arguments;
use guile_3_sys::*;
pub use scm::*;

mod scm;

pub type SetupFn = extern "C" fn(argc: i32, argv: *mut *mut i8);

/// Boots Guile scheme, runs the setup function and enters the Guile Scheme shell.
pub fn boot_with_shell(args: std::env::Args, setup: SetupFn) {
    boot(args, run_setup_and_shell, setup as *mut c_void);
}

extern "C" fn run_setup_and_shell(setup: *mut c_void, argc: i32, argv: *mut *mut i8) {
    let setup_fn: SetupFn = unsafe { std::mem::transmute(setup) };
    (setup_fn)(argc, argv);
    unsafe { scm_shell(argc, argv) };
}

fn boot(
    args: std::env::Args,
    main: extern "C" fn(closure: *mut c_void, argc: i32, argv: *mut *mut i8),
    closure: *mut c_void,
) {
    let argv: Vec<CString> = args.map(CString::new).map(Result::unwrap).collect();
    let arg_ptrs: Vec<*const c_char> = argv.into_iter().map(|arg| arg.as_ptr()).collect();

    unsafe {
        scm_boot_guile(
            arg_ptrs.len() as _,
            arg_ptrs.as_ptr() as _,
            Some(main),
            closure,
        );
    };
}

/// Define a subroutine.
///
/// `name` - The name of the subroutine.
/// `req`  - The number of required arguments.
/// `opt`  - The number of optional arguments.
/// `rst`  - The number of rest arguments.
/// `fcn`  - The function implementation. The function must be of type
///          `extern "C"` or `unsafe extern "C"`. It must take the
///          appropriate amount of `Scm` as arguments and return a
///          single `Scm` object.
///
/// # Safety
/// Undefined behavior if `fcn` does not have the right signature.
pub unsafe fn define_subr(name: &CStr, req: usize, opt: usize, rst: usize, fcn: scm_t_subr) -> Scm {
    scm::Scm::new(scm_c_define_gsubr(
        name.as_ptr(),
        req as _,
        opt as _,
        rst as _,
        fcn,
    ))
}

/// Raises a Scheme error. This is similar to a Rust panic.
///
/// # Safety
/// Uses unsafe functions.
pub unsafe fn scm_error(k: Scm, subr: &CStr, message: &CStr, args: Scm, rest: Scm) -> ! {
    guile_3_sys::scm_error(
        k.raw(),
        subr.as_ptr(),
        message.as_ptr(),
        args.raw(),
        rest.raw(),
    );
    unreachable!()
}
