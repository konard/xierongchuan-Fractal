//! Host-runnable copy of the session serialization format.
//!
//! `src/secret/session_data.rs` is `#[cfg(any(target_os = "android", test))]`
//! and lives inside a crate that needs the full GTK stack to build, which is
//! not available on this host. This experiment mirrors the module and the
//! fields of `StoredSession` so the format logic and its tests can be run
//! directly with `cargo test`.

use matrix_sdk::authentication::oauth::ClientId;
use ruma::{OwnedDeviceId, OwnedUserId};
use url::Url;
use zeroize::Zeroizing;

/// A copy of `crate::secret::StoredSession` with the same field types.
#[derive(Clone)]
pub struct StoredSession {
    pub homeserver: Url,
    pub user_id: OwnedUserId,
    pub device_id: OwnedDeviceId,
    pub id: String,
    pub client_id: Option<ClientId>,
    pub passphrase: Zeroizing<String>,
}

#[path = "session_data.rs"]
mod session_data;

impl std::fmt::Debug for StoredSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StoredSession")
            .field("homeserver", &self.homeserver)
            .field("user_id", &self.user_id)
            .field("device_id", &self.device_id)
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}
