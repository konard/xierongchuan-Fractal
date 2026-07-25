//! Image decoding with glycin.

use std::time::Duration;

use gtk::{gdk, gio, glib};

use crate::{DISABLE_GLYCIN_SANDBOX, utils::media::FrameDimensions};

/// The source of the bytes to decode.
pub(crate) enum DecoderSource {
    /// The bytes containing the encoded image.
    Data(Vec<u8>),
    /// The file containing the encoded image.
    File(gio::File),
}

impl DecoderSource {
    /// Convert this source into a glycin loader.
    fn into_loader(self) -> glycin::Loader {
        let loader = match self {
            Self::Data(bytes) => glycin::Loader::for_bytes(&glib::Bytes::from_owned(bytes)),
            Self::File(file) => glycin::Loader::new(&file),
        };

        if DISABLE_GLYCIN_SANDBOX {
            loader.set_sandbox_selector(glycin::SandboxSelector::NotSandboxed);
        }

        loader
    }
}

/// A decoder for an image, able to load its successive frames.
#[derive(Debug, Clone)]
pub(crate) struct Decoder(glycin::Image);

impl Decoder {
    /// The width of the image.
    pub(crate) fn width(&self) -> u32 {
        self.0.width()
    }

    /// The height of the image.
    pub(crate) fn height(&self) -> u32 {
        self.0.height()
    }

    /// Load the next frame of the image.
    pub(crate) async fn next_frame(&self) -> Result<Frame, glib::Error> {
        self.0.next_frame_future().await.map(Frame)
    }
}

/// A decoded frame of an image.
#[derive(Debug, Clone)]
pub(crate) struct Frame(glycin::Frame);

impl Frame {
    /// The width of the frame.
    pub(crate) fn width(&self) -> u32 {
        self.0.width()
    }

    /// The height of the frame.
    pub(crate) fn height(&self) -> u32 {
        self.0.height()
    }

    /// Whether the frame has a delay, which means that the image is animated.
    pub(crate) fn has_delay(&self) -> bool {
        // glycin always computes a suitable delay if the image is animated but its
        // delay is set to 0, so 0 should mean that the image is not animated.
        self.0.delay() > 0
    }

    /// How long to show this frame for if the image is animated.
    pub(crate) fn delay_duration(&self) -> Option<Duration> {
        self.has_delay()
            .then(|| u64::try_from(self.0.delay()).ok())
            .flatten()
            .map(Duration::from_micros)
    }

    /// Convert this frame to a [`gdk::Texture`].
    pub(crate) fn texture(&self) -> gdk::Texture {
        glycin_gtk4::frame_get_texture(&self.0)
    }
}

/// Decode the given source and load its first frame.
///
/// Set `request_dimensions` if the image will be shown at specific dimensions.
/// To show the image at its natural size, set it to `None`.
pub(crate) async fn decode(
    source: DecoderSource,
    request_dimensions: Option<FrameDimensions>,
) -> Result<(Decoder, Frame), glib::Error> {
    let decoder = source.into_loader().load_future().await?;

    let first_frame = if let Some(requested) = request_dimensions {
        let original = FrameDimensions {
            width: decoder.width(),
            height: decoder.height(),
        };
        let scaled = original.scale_to_fit(requested, gtk::ContentFit::Cover);

        let request = glycin::FrameRequest::new();
        request.set_scale(scaled.width, scaled.height);

        decoder.specific_frame_future(&request).await?
    } else {
        decoder.next_frame_future().await?
    };

    Ok((Decoder(decoder), Frame(first_frame)))
}

/// Whether the given error means that the image format is not supported.
pub(crate) fn is_unsupported_format(error: &glib::Error) -> bool {
    matches!(error.kind(), Some(glycin::LoaderError::UnknownImageFormat))
}
