//! Fallback video player for the platforms without GStreamer.
//!
//! Fractal decodes videos with GStreamer, which is not available on every
//! platform. This widget has the same API as the GStreamer video player, but
//! it only shows a placeholder instead of the first frames of the video. The
//! video itself can still be downloaded and opened with another application.

use adw::{prelude::*, subclass::prelude::*};
use gtk::{gio, glib};

use crate::utils::LoadingState;

mod imp {
    use std::cell::{Cell, RefCell};

    use super::*;

    #[derive(Debug, glib::Properties)]
    #[properties(wrapper_type = super::VideoPlayer)]
    pub struct VideoPlayer {
        /// The placeholder shown instead of the video.
        placeholder: gtk::Image,
        /// The file that would be played.
        file: RefCell<Option<gio::File>>,
        /// Whether the player is displayed in its compact form.
        #[property(get, set = Self::set_compact, explicit_notify)]
        compact: Cell<bool>,
        /// The state of the video in this player.
        #[property(get, builder(LoadingState::default()))]
        state: Cell<LoadingState>,
        /// The current error, if any.
        pub(super) error: RefCell<Option<glib::Error>>,
    }

    impl Default for VideoPlayer {
        fn default() -> Self {
            Self {
                placeholder: gtk::Image::builder()
                    .icon_name("video-symbolic")
                    .pixel_size(48)
                    .build(),
                file: RefCell::default(),
                compact: Cell::default(),
                state: Cell::default(),
                error: RefCell::default(),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for VideoPlayer {
        const NAME: &'static str = "VideoPlayer";
        type Type = super::VideoPlayer;
        type ParentType = adw::Bin;
    }

    #[glib::derived_properties]
    impl ObjectImpl for VideoPlayer {
        fn constructed(&self) {
            self.parent_constructed();

            self.placeholder.add_css_class("dimmed");
            self.obj().set_child(Some(&self.placeholder));
        }
    }

    impl WidgetImpl for VideoPlayer {}
    impl BinImpl for VideoPlayer {}

    impl VideoPlayer {
        /// Set whether this player should be displayed in a compact format.
        fn set_compact(&self, compact: bool) {
            if self.compact.get() == compact {
                return;
            }

            self.placeholder.set_pixel_size(if compact { 24 } else { 48 });

            self.compact.set(compact);
            self.obj().notify_compact();
        }

        /// Set the state of the media.
        fn set_state(&self, state: LoadingState) {
            if self.state.get() == state {
                return;
            }

            self.state.set(state);
            self.obj().notify_state();
        }

        /// Set the video file to play.
        pub(super) fn play_video_file(&self, file: gio::File) {
            self.file.replace(Some(file));
            // There is nothing to decode, the placeholder is ready to be shown.
            self.set_state(LoadingState::Ready);
        }
    }
}

glib::wrapper! {
    /// A widget to preview a video file without controls or sound.
    pub struct VideoPlayer(ObjectSubclass<imp::VideoPlayer>)
        @extends gtk::Widget, adw::Bin,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl VideoPlayer {
    /// Create a new video player.
    pub fn new() -> Self {
        glib::Object::new()
    }

    /// Set the video file to play.
    pub(crate) fn play_video_file(&self, file: gio::File) {
        self.imp().play_video_file(file);
    }

    /// The current error, if any.
    pub(crate) fn error(&self) -> Option<glib::Error> {
        self.imp().error.borrow().clone()
    }
}
