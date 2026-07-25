//! Platform integration.
//!
//! Everything that must be done differently depending on the platform Fractal
//! runs on lives here, behind a single API. The rest of the crate is platform
//! agnostic and must not use `cfg(target_os = …)` for these concerns.

cfg_if::cfg_if! {
    if #[cfg(target_os = "android")] {
        mod android;
        use self::android as imp;
    } else {
        mod desktop;
        use self::desktop as imp;
    }
}

pub(crate) use self::imp::{cache_dir, data_dir, init_logging, localedir, register_resources};

/// Create the given directory and all its parents, if they do not exist.
///
/// Returns the directory unchanged, so it can be used inline. The desktop
/// directories are created by the platform, so this is only needed on Android.
#[cfg(target_os = "android")]
fn ensure_dir(dir: std::path::PathBuf) -> std::path::PathBuf {
    if let Err(error) = std::fs::create_dir_all(&dir) {
        tracing::error!("Could not create directory `{}`: {error}", dir.display());
    }

    dir
}
