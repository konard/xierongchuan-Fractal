//! The serialized form of the sessions of a secret backend that stores them all
//! in a single blob.
//!
//! The Linux backend stores one item per session in the Secret Service, with
//! the metadata as searchable attributes of the item. A backend that has only a
//! key at its disposal -- the Android Keystore -- has to serialize everything
//! itself, so the format lives here, apart from the platform code, and is unit
//! tested on every platform.

use matrix_sdk::authentication::oauth::ClientId;
use ruma::{OwnedDeviceId, UserId};
use serde::{Deserialize, Serialize};
use url::Url;

use super::StoredSession;

/// The version of the format written by this version of Fractal.
const CURRENT_VERSION: u8 = 1;

/// All the sessions of a secret backend, as they are serialized.
///
/// The version is the first field so that a format written by a later version
/// of Fractal is recognized before anything else is interpreted.
#[derive(Debug, Serialize, Deserialize)]
pub(super) struct SerializedSessions {
    /// The version of the format.
    version: u8,
    /// The sessions.
    sessions: Vec<SerializedSession>,
}

impl SerializedSessions {
    /// Serialize the given sessions.
    pub(super) fn encode(
        sessions: impl IntoIterator<Item = StoredSession>,
    ) -> Result<Vec<u8>, SessionsFormatError> {
        let sessions = Self {
            version: CURRENT_VERSION,
            sessions: sessions.into_iter().map(SerializedSession::from).collect(),
        };

        Ok(rmp_serde::to_vec_named(&sessions)?)
    }

    /// Deserialize the sessions in the given bytes.
    pub(super) fn decode(bytes: &[u8]) -> Result<Vec<StoredSession>, SessionsFormatError> {
        let sessions = rmp_serde::from_slice::<Self>(bytes)?;

        if sessions.version > CURRENT_VERSION {
            // The sessions were written by a later version of Fractal. They
            // cannot be interpreted, and must not be overwritten either, or
            // downgrading once would log the user out for good.
            return Err(SessionsFormatError::UnsupportedVersion(sessions.version));
        }

        sessions
            .sessions
            .into_iter()
            .map(StoredSession::try_from)
            .collect()
    }
}

/// A single session, as it is serialized.
///
/// The identifiers are stored as the strings they are rather than as the types
/// of the Matrix SDK, so that the stored format does not follow the types.
#[derive(Debug, Serialize, Deserialize)]
struct SerializedSession {
    /// The URL of the homeserver where the account lives.
    homeserver: String,
    /// The unique identifier of the user.
    user_id: String,
    /// The unique identifier of the session on the homeserver.
    device_id: String,
    /// The unique local identifier of the session.
    id: String,
    /// The unique identifier of the client with the homeserver.
    client_id: Option<String>,
    /// The passphrase used to encrypt the local databases.
    passphrase: String,
}

impl From<StoredSession> for SerializedSession {
    fn from(value: StoredSession) -> Self {
        Self {
            homeserver: value.homeserver.to_string(),
            user_id: value.user_id.to_string(),
            device_id: value.device_id.to_string(),
            id: value.id,
            client_id: value
                .client_id
                .map(|client_id| client_id.as_str().to_owned()),
            passphrase: value.passphrase.to_string(),
        }
    }
}

impl TryFrom<SerializedSession> for StoredSession {
    type Error = SessionsFormatError;

    fn try_from(value: SerializedSession) -> Result<Self, Self::Error> {
        let SerializedSession {
            homeserver,
            user_id,
            device_id,
            id,
            client_id,
            passphrase,
        } = value;

        Ok(Self {
            homeserver: Url::parse(&homeserver)
                .map_err(|_| SessionsFormatError::InvalidField("homeserver"))?,
            user_id: UserId::parse(&user_id)
                .map_err(|_| SessionsFormatError::InvalidField("user_id"))?,
            device_id: OwnedDeviceId::from(device_id.as_str()),
            id,
            client_id: client_id.map(ClientId::new),
            passphrase: passphrase.into(),
        })
    }
}

/// All errors that can occur when serializing or deserializing sessions.
#[derive(Debug, thiserror::Error)]
pub(super) enum SessionsFormatError {
    /// The sessions were written by a later version of Fractal.
    #[error("Sessions were stored with the unsupported version {0}")]
    UnsupportedVersion(u8),

    /// A field of a session does not have the expected form.
    #[error("Stored session has an invalid `{0}` field")]
    InvalidField(&'static str),

    /// An error occurred when decoding the sessions.
    #[error(transparent)]
    Decode(#[from] rmp_serde::decode::Error),

    /// An error occurred when encoding the sessions.
    #[error(transparent)]
    Encode(#[from] rmp_serde::encode::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(id: &str, client_id: Option<&str>) -> StoredSession {
        StoredSession {
            homeserver: Url::parse("https://matrix.example.org/").unwrap(),
            user_id: UserId::parse("@alice:example.org").unwrap(),
            device_id: OwnedDeviceId::from("ABCDEFGHIJ"),
            id: id.to_owned(),
            client_id: client_id.map(|client_id| ClientId::new(client_id.to_owned())),
            passphrase: "correct horse battery staple".to_owned().into(),
        }
    }

    #[test]
    fn round_trip() {
        let sessions = vec![
            session("12345678", Some("client-id")),
            session("abcdefgh", None),
        ];

        let bytes = SerializedSessions::encode(sessions.clone()).unwrap();
        let decoded = SerializedSessions::decode(&bytes).unwrap();

        assert_eq!(decoded.len(), sessions.len());

        for (decoded, session) in decoded.iter().zip(&sessions) {
            assert_eq!(decoded.homeserver, session.homeserver);
            assert_eq!(decoded.user_id, session.user_id);
            assert_eq!(decoded.device_id, session.device_id);
            assert_eq!(decoded.id, session.id);
            assert_eq!(
                decoded
                    .client_id
                    .as_ref()
                    .map(|client_id| client_id.as_str()),
                session
                    .client_id
                    .as_ref()
                    .map(|client_id| client_id.as_str())
            );
            assert_eq!(*decoded.passphrase, *session.passphrase);
        }
    }

    #[test]
    fn no_sessions() {
        let bytes = SerializedSessions::encode(Vec::new()).unwrap();
        assert!(SerializedSessions::decode(&bytes).unwrap().is_empty());
    }

    #[test]
    fn unsupported_version() {
        let sessions = SerializedSessions {
            version: CURRENT_VERSION + 1,
            sessions: vec![session("12345678", None).into()],
        };
        let bytes = rmp_serde::to_vec_named(&sessions).unwrap();

        let error = SerializedSessions::decode(&bytes).unwrap_err();
        assert!(matches!(error, SessionsFormatError::UnsupportedVersion(_)));
    }

    #[test]
    fn invalid_field() {
        let mut serialized = SerializedSession::from(session("12345678", None));
        serialized.user_id = "not a user ID".to_owned();

        let sessions = SerializedSessions {
            version: CURRENT_VERSION,
            sessions: vec![serialized],
        };
        let bytes = rmp_serde::to_vec_named(&sessions).unwrap();

        let error = SerializedSessions::decode(&bytes).unwrap_err();
        assert!(matches!(
            error,
            SessionsFormatError::InvalidField("user_id")
        ));
    }

    #[test]
    fn not_sessions_at_all() {
        let error = SerializedSessions::decode(b"this is not MessagePack").unwrap_err();
        assert!(matches!(error, SessionsFormatError::Decode(_)));
    }
}
