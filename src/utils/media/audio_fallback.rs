//! Collection of methods for audio, for the platforms without `GStreamer`.
//!
//! Audio files are decoded with `GStreamer`, which is not available on every
//! platform. Without it, the duration and the waveform of an audio file cannot
//! be computed, but the file can still be sent and downloaded.

use gtk::gio;
use matrix_sdk::attachment::BaseAudioInfo;

pub(crate) use super::waveform::normalize_waveform;

/// Load information for the audio in the given file.
///
/// Without `GStreamer`, no information can be extracted, so this always returns
/// the default information.
pub(crate) async fn load_audio_info(_file: &gio::File) -> BaseAudioInfo {
    BaseAudioInfo::default()
}
