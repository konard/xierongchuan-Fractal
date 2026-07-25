//! Desktop entry point of Fractal.
//!
//! The whole application lives in the `fractal` library, so that the desktop
//! binary and the Android library share the exact same initialization.

fn main() -> gtk::glib::ExitCode {
    fractal::run()
}
