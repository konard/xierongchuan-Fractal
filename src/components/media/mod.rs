mod animated_image_paintable;
mod audio_player;
mod content_viewer;
#[cfg_attr(not(feature = "shumate"), path = "location_viewer_fallback.rs")]
mod location_viewer;
#[cfg_attr(not(feature = "gstreamer"), path = "video_player_fallback.rs")]
mod video_player;
#[cfg(feature = "gstreamer")]
mod video_player_renderer;

pub(crate) use self::{
    animated_image_paintable::AnimatedImagePaintable,
    audio_player::*,
    content_viewer::{ContentType, MediaContentViewer},
    location_viewer::LocationViewer,
    video_player::VideoPlayer,
};
