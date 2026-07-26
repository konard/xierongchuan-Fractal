//! Platform integration for Android.
//!
//! Android applications have no writable installation prefix: everything that
//! the desktop build loads from `PKGDATADIR` must either be compiled into the
//! library or be resolved inside the sandbox of the application at runtime.

use std::path::PathBuf;

use gtk::{gio, glib};
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
