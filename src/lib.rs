#![doc(
    html_logo_url = "https://gitlab.gnome.org/World/fractal/-/raw/main/data/icons/org.gnome.Fractal.svg?inline=false",
    html_favicon_url = "https://gitlab.gnome.org/World/fractal/-/raw/main/data/icons/org.gnome.Fractal-symbolic.svg?inline=false"
)]

mod account_chooser_dialog;
mod account_switcher;
mod application;
mod components;
#[rustfmt::skip]
mod config;
mod account_settings;
mod contrib;
mod error_page;
mod i18n;
mod identity_verification_view;
mod intent;
mod login;
mod platform;
mod prelude;
mod secret;
mod session;
mod session_list;
mod session_view;
mod system_settings;
mod user_facing_error;
mod utils;
mod window;

use std::sync::LazyLock;

use gettextrs::*;
use gtk::{IconTheme, gdk::Display, glib};

use self::{application::*, config::*, i18n::*, utils::OneshotNotifier, window::Window};

/// The default tokio runtime to be used for async tasks
static RUNTIME: LazyLock<tokio::runtime::Runtime> = LazyLock::new(|| {
    tokio::runtime::Runtime::new().expect("creating tokio runtime should succeed")
});

/// The notifier to make sure that only one `GtkMediaFile` is played at a single
/// time.
static MEDIA_FILE_NOTIFIER: LazyLock<OneshotNotifier> =
    LazyLock::new(|| OneshotNotifier::new("MEDIA_FILE_NOTIFIER"));

/// Initialize the application and run it until it exits.
///
/// This is the single entry point shared by every platform: the desktop binary
/// calls it from `main()`, and the Android library calls it from the entry
/// point of the Android runtime. Platform differences are behind
/// [`platform`], they must not be duplicated here.
pub fn run() -> glib::ExitCode {
    // Initialize logger, debug is carried out via debug!, info!, warn! and error!.
    // Default to the INFO level for this crate and WARN for everything else.
    // It can be overridden with the RUST_LOG environment variable.
    platform::init_logging();

    // Prepare i18n
    setlocale(LocaleCategory::LcAll, "");
    bindtextdomain(GETTEXT_PACKAGE, platform::localedir()).expect("Invalid locale directory");
    textdomain(GETTEXT_PACKAGE).expect("Invalid string passed to textdomain");

    glib::set_application_name("Fractal");

    gtk::init().expect("Could not start GTK4");
    #[cfg(feature = "gstreamer")]
    gst::init().expect("Could not initialize gst");

    #[cfg(target_os = "linux")]
    aperture::init(APP_ID);

    // Capture what the secret backend needs from the main thread before any
    // session work is spawned onto the tokio runtime.
    secret::init();

    // Resources must be registered before the first widget is created.
    platform::register_resources();

    IconTheme::for_display(&Display::default().expect("GDK display should be initialized"))
        .add_resource_path("/org/gnome/Fractal/icons");

    let app = Application::new();
    app.run()
}
