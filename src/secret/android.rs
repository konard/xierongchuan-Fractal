//! Android API to store the data of a session.
//!
//! Sessions must only be persisted behind a key of the Android Keystore. Until
//! that backend exists, this implementation behaves like a system where no
//! session was ever stored: restoring returns an empty list, and storing fails
//! with a user-facing error instead of writing secrets in the clear.

use gettextrs::gettext;
use tracing::warn;

use super::{SecretError, SecretExt, StoredSession};

/// Secret API under Android.
#[derive(Debug)]
pub(crate) struct AndroidSecret;

impl SecretExt for AndroidSecret {
    async fn restore_sessions() -> Result<Vec<StoredSession>, SecretError> {
        // There is no secret backend yet, so there cannot be a stored session.
        Ok(Vec::new())
    }

    async fn store_session(_session: StoredSession) -> Result<(), SecretError> {
        warn!("Could not store session: the Android secret backend is not implemented yet");
        Err(SecretError::Service(gettext(
            "Sessions cannot be saved on this device yet, so you will have to log in again the next time you open Fractal",
        )))
    }

    async fn delete_session(_session: &StoredSession) {
        // Nothing was ever stored, so there is nothing to delete.
    }
}
