//! Platform integration for desktop systems.

use std::path::PathBuf;

use gtk::{gio, glib};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

use crate::config::{LOCALEDIR, RESOURCES_FILE, UI_RESOURCES_FILE};

/// Initialize the logger.
///
/// The default level is INFO for this crate and WARN for everything else. It
/// can be overridden with the `RUST_LOG` environment variable.
pub(crate) fn init_logging() {
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("fractal=info,warn"));

    tracing_subscriber::registry()
        .with(fmt::layer().with_filter(env_filter))
        .init();
}

/// The directory where the translations are installed.
pub(crate) fn localedir() -> PathBuf {
    LOCALEDIR.into()
}

/// Register the GResources of the application.
///
/// They are installed next to the binary, so they are loaded from the data
/// directory of the application.
pub(crate) fn register_resources() {
    for path in [RESOURCES_FILE, UI_RESOURCES_FILE] {
        let resource = gio::Resource::load(path)
            .unwrap_or_else(|error| panic!("Could not load gresource file `{path}`: {error}"));
        gio::resources_register(&resource);
    }
}

/// The directory where persistent data is stored.
pub(crate) fn data_dir() -> PathBuf {
    glib::user_data_dir()
}

/// The directory where cache data is stored.
pub(crate) fn cache_dir() -> PathBuf {
    glib::user_cache_dir()
}
