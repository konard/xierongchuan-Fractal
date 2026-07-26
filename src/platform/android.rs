//! Platform integration for Android.
//!
//! Android applications have no writable installation prefix: everything that
//! the desktop build loads from `PKGDATADIR` must either be compiled into the
//! library or be resolved inside the sandbox of the application at runtime.

use std::{panic, path::PathBuf, thread};

use gtk::{gio, glib};
use tracing::error;
use tracing_subscriber::{EnvFilter, prelude::*};

use super::ensure_dir;

/// The GResources of the application, compiled into the library.
///
/// `FRACTAL_RESOURCES_FILE` is set by the Android build to the GResource
/// bundle compiled by Meson. Because the bundles are embedded, no absolute
/// path from the desktop installation layout is needed at runtime.
const RESOURCES: &[u8] = include_bytes!(env!("FRACTAL_RESOURCES_FILE"));
/// The UI GResources of the application, compiled into the library.
const UI_RESOURCES: &[u8] = include_bytes!(env!("FRACTAL_UI_RESOURCES_FILE"));

/// Initialize the logger, so that traces end up in `logcat`.
pub(crate) fn init_logging() {
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("fractal=info,warn"));

    tracing_subscriber::registry()
        .with(tracing_android::layer("Fractal").expect("logcat layer should be created"))
        .with(env_filter)
        .init();

    // The logger is in place, so panics can be routed to it before any code
    // that might panic runs.
    init_panic_logging();
}

/// Route panics to `logcat`.
///
/// The default panic hook writes to standard error, which Android discards, so
/// a panic there aborts the process without leaving anything in `logcat` -- the
/// crash looks like a bare `SIGABRT` with no reason. This forwards the panic to
/// the tracing logger, which does reach `logcat`, and then calls the previous
/// hook so a backtrace is still emitted for anyone attached to standard error.
fn init_panic_logging() {
    let previous_hook = panic::take_hook();

    panic::set_hook(Box::new(move |info| {
        let thread = thread::current();
        let name = thread.name().unwrap_or("<unnamed>");
        let location = info
            .location()
            .map_or_else(|| "an unknown location".to_owned(), ToString::to_string);

        error!(
            "Thread '{name}' panicked at {location}: {}",
            panic_message(info.payload())
        );

        previous_hook(info);
    }));
}

/// The message a panic carries, whether it was a `&str` or a formatted
/// `String`.
///
/// [`panic::PanicHookInfo::message`] is not yet stable, so the payload is
/// inspected by hand, exactly as the standard hook does it.
fn panic_message(payload: &(dyn std::any::Any + Send)) -> &str {
    if let Some(message) = payload.downcast_ref::<&str>() {
        message
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message
    } else {
        "Box<dyn Any>"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panic_message_from_str() {
        let payload = panic::catch_unwind(|| panic!("a static message")).unwrap_err();
        assert_eq!(panic_message(&*payload), "a static message");
    }

    #[test]
    fn panic_message_from_string() {
        let value = 42;
        let payload = panic::catch_unwind(|| panic!("a formatted message: {value}")).unwrap_err();
        assert_eq!(panic_message(&*payload), "a formatted message: 42");
    }

    #[test]
    fn panic_message_from_other_payload() {
        let payload = panic::catch_unwind(|| panic::panic_any(42_u8)).unwrap_err();
        assert_eq!(panic_message(&*payload), "Box<dyn Any>");
    }
}

/// The directory where the translations are installed.
///
/// Unlike the data of the user, the translations are part of the installation:
/// they are shipped as assets and unpacked into the read-only data directory
/// of the application. That is the first of the GLib system data directories,
/// which the GTK Android backend sets from the context of the application.
pub(crate) fn localedir() -> PathBuf {
    if let Some(dir) = std::env::var_os("FRACTAL_LOCALEDIR") {
        return dir.into();
    }

    let mut dir = glib::system_data_dirs()
        .into_iter()
        .next()
        .unwrap_or_else(glib::user_data_dir);
    dir.push("locale");
    dir
}

/// Register the GResources of the application.
pub(crate) fn register_resources() {
    for bytes in [RESOURCES, UI_RESOURCES] {
        let resource = gio::Resource::from_data(&glib::Bytes::from_static(bytes))
            .expect("embedded gresource data should be valid");
        gio::resources_register(&resource);
    }
}

/// The directory where persistent data is stored.
pub(crate) fn data_dir() -> PathBuf {
    ensure_dir(app_dir("FRACTAL_DATA_DIR", "data"))
}

/// The directory where cache data is stored.
pub(crate) fn cache_dir() -> PathBuf {
    ensure_dir(app_dir("FRACTAL_CACHE_DIR", "cache"))
}

/// The writable directory for the given purpose inside the sandbox of the
/// application.
///
/// It is derived from the GLib user data directory, which the GTK Android
/// backend initializes from the context of the application. The environment
/// variable overrides it, so that a directory can be pointed elsewhere for
/// debugging without rebuilding.
fn app_dir(env_var: &str, fallback_name: &str) -> PathBuf {
    if let Some(dir) = std::env::var_os(env_var) {
        return dir.into();
    }

    let mut dir = glib::user_data_dir();
    dir.push(fallback_name);
    dir
}
