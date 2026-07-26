//! Collection of methods for videos, for the platforms without `GStreamer`.
//!
//! Videos are decoded with `GStreamer`, which is not available on every
//! platform. Without it, neither the dimensions nor a thumbnail of a video can
//! be computed, but the file can still be sent and downloaded.

use gtk::{gio, prelude::*};
use matrix_sdk::attachment::{BaseVideoInfo, Thumbnail};

/// Load information and try to generate a thumbnail for the video in the given
/// file.
///
/// Without `GStreamer`, no information can be extracted, so this always returns
/// the default information and no thumbnail.
pub(crate) async fn load_video_info(
    _file: &gio::File,
    _widget: &impl IsA<gtk::Widget>,
) -> (BaseVideoInfo, Option<Thumbnail>) {
    (BaseVideoInfo::default(), None)
}
