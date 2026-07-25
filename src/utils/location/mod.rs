//! Location API.

use futures_util::Stream;
use geo_uri::GeoUri;

#[cfg(target_os = "linux")]
mod linux;

cfg_if::cfg_if! {
    if #[cfg(target_os = "linux")] {
        /// The secret API.
        pub(crate) type Location = linux::LinuxLocation;
    } else {
        /// The secret API.
        pub(crate) type Location = unimplemented::UnimplementedLocation;
    }
}

/// Trait implemented by location backends.
pub(crate) trait LocationExt {
    /// Whether the location API is available.
    fn is_available(&self) -> bool;

    /// Initialize the location API.
    async fn init(&self) -> Result<(), LocationError>;

    /// Listen to a stream of location updates.
    async fn updates_stream(&self) -> Result<impl Stream<Item = GeoUri> + '_, LocationError>;
}

/// The fallback location API, used on platforms where it is unimplemented.
///
/// It must not panic: it always advertises itself as unavailable, and returns
/// an error if it is used anyway.
#[cfg(not(target_os = "linux"))]
mod unimplemented {
    use super::*;

    #[derive(Debug, Default)]
    pub(crate) struct UnimplementedLocation;

    impl LocationExt for UnimplementedLocation {
        /// Whether the location API is available.
        fn is_available(&self) -> bool {
            false
        }

        /// Initialize the location API.
        async fn init(&self) -> Result<(), LocationError> {
            tracing::error!("The location API is not supported on this platform");
            Err(LocationError::Other)
        }

        /// Listen to a stream of location updates.
        async fn updates_stream(&self) -> Result<impl Stream<Item = GeoUri> + '_, LocationError> {
            tracing::error!("The location API is not supported on this platform");
            Err::<futures_util::stream::Empty<GeoUri>, _>(LocationError::Other)
        }
    }
}

/// High-level errors that can occur while fetching the location.
#[derive(Debug, Clone, Copy)]
pub(crate) enum LocationError {
    /// The user cancelled the request to get the location.
    Cancelled,
    /// The location services are disabled on the system.
    Disabled,
    /// Another error occurred.
    Other,
}
