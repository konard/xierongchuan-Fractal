//! The Android library of Fractal.
//!
//! The Android runtime starts a regular native executable, which is a small C
//! shim (`android/shim/main.c`). The shim immediately hands over to
//! [`fractal_main()`], so that the initialization of the application is the
//! same on every platform.

use std::os::raw::{c_char, c_int};

/// Run Fractal.
///
/// This is the only symbol the Android shim needs. `argc` and `argv` are
/// forwarded by the runtime and are only used by GApplication, which does not
/// keep a reference to them after it returns.
///
/// # Safety
///
/// `argv` must be a valid, `NULL`-terminated array of `argc` C strings, as
/// passed to `main()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fractal_main(_argc: c_int, _argv: *mut *mut c_char) -> c_int {
    // GTK on Android does not use the command line of the process, and Fractal
    // has no command line options of its own, so the arguments are not
    // forwarded to GApplication.
    let exit_code: i32 = fractal::run().into();
    exit_code
}
