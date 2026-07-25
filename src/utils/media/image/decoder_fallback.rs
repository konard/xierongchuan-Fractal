//! Image decoding with GDK, for the platforms without glycin.
//!
//! glycin decodes images in a sandboxed subprocess, which is not available on
//! every platform. GDK can decode the most common image formats on its own,
//! but it does not support animations, so animated images are shown as a still
//! image of their first frame.

use std::time::Duration;

use gtk::{gdk, gio, glib, prelude::*};

use crate::utils::media::FrameDimensions;

/// The source of the bytes to decode.
pub(crate) enum DecoderSource {
    /// The bytes containing the encoded image.
    Data(Vec<u8>),
    /// The file containing the encoded image.
    File(gio::File),
}

impl DecoderSource {
    /// Decode this source into a texture.
    fn decode(self) -> Result<gdk::Texture, glib::Error> {
        match self {
            Self::Data(bytes) => gdk::Texture::from_bytes(&glib::Bytes::from_owned(bytes)),
            Self::File(file) => gdk::Texture::from_file(&file),
        }
    }
}

/// A decoder for an image, able to load its successive frames.
#[derive(Debug, Clone)]
pub(crate) struct Decoder(gdk::Texture);

impl Decoder {
    /// The width of the image.
    pub(crate) fn width(&self) -> u32 {
        self.0.width().try_into().unwrap_or_default()
    }

    /// The height of the image.
    pub(crate) fn height(&self) -> u32 {
        self.0.height().try_into().unwrap_or_default()
    }

    /// Load the next frame of the image.
    ///
    /// GDK cannot decode animations, so an image always has a single frame and
    /// this always returns the same frame.
    pub(crate) async fn next_frame(&self) -> Result<Frame, glib::Error> {
        Ok(Frame(self.0.clone()))
    }
}

/// A decoded frame of an image.
#[derive(Debug, Clone)]
pub(crate) struct Frame(gdk::Texture);

impl Frame {
    /// The width of the frame.
    pub(crate) fn width(&self) -> u32 {
        self.0.width().try_into().unwrap_or_default()
    }

    /// The height of the frame.
    pub(crate) fn height(&self) -> u32 {
        self.0.height().try_into().unwrap_or_default()
    }

    /// Whether the frame has a delay, which means that the image is animated.
    ///
    /// GDK cannot decode animations, so this is always `false`.
    pub(crate) fn has_delay(&self) -> bool {
        false
    }

    /// How long to show this frame for if the image is animated.
    ///
    /// GDK cannot decode animations, so this is always `None`.
    pub(crate) fn delay_duration(&self) -> Option<Duration> {
        None
    }

    /// Convert this frame to a [`gdk::Texture`].
    pub(crate) fn texture(&self) -> gdk::Texture {
        self.0.clone()
    }
}

/// Decode the given source and load its first frame.
///
/// GDK always decodes an image at its natural size, so `request_dimensions` is
/// ignored.
pub(crate) async fn decode(
    source: DecoderSource,
    _request_dimensions: Option<FrameDimensions>,
) -> Result<(Decoder, Frame), glib::Error> {
    let texture = source.decode()?;

    Ok((Decoder(texture.clone()), Frame(texture)))
}

/// Whether the given error means that the image format is not supported.
pub(crate) fn is_unsupported_format(error: &glib::Error) -> bool {
    matches!(
        error.kind(),
        Some(gdk::TextureError::UnsupportedContent | gdk::TextureError::UnsupportedFormat)
    )
}
