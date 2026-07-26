//! Android API to store the data of a session.
//!
//! Android has no Secret Service. What every application has is its own private
//! directory and the Android Keystore, a store of key material that the system
//! holds on the behalf of the application. Fractal therefore keeps all of its
//! sessions in a single file of its private directory, encrypted with a key
//! that lives in the Keystore and never leaves the system.
//!
//! The serialized form of the sessions is in [`super::session_data`], apart
//! from this platform code so that it is unit tested on every platform. The
//! bridge to the Keystore is in [`keystore`].

mod keystore;

use std::{fs, path::PathBuf, sync::Mutex};

use gettextrs::gettext;
use tracing::{error, warn};

use self::keystore::KeystoreError;
use super::{
    SecretError, SecretExt, StoredSession,
    session_data::{SerializedSessions, SessionsFormatError},
};
use crate::{spawn_tokio, utils::DataType};

/// The name of the file that holds the encrypted sessions.
///
/// It sits next to the per-session directories, whose names are eight
/// alphanumeric characters, so the dot keeps it from ever colliding with one.
const SESSIONS_FILE: &str = "sessions.mpack";

/// Serializes the read-modify-write of the sessions file.
///
/// Storing and deleting both read the whole file, change it, and write it back.
/// The operations run on the tokio runtime, so without this two of them could
/// interleave and lose a session.
static FILE_LOCK: Mutex<()> = Mutex::new(());

/// Capture what the Keystore needs from the GTK main thread.
///
/// This must run on the thread of the GTK main loop, after `gtk::init()`, so
/// that the Java virtual machine can be reached and kept for the tokio tasks
/// that store and restore sessions. A failure is not fatal: the sessions simply
/// cannot be persisted, exactly as before this backend existed.
pub(super) fn init() {
    if let Err(error) = keystore::init() {
        warn!("Sessions will not be persisted on this device: {error}");
    }
}

/// Secret API under Android.
#[derive(Debug)]
pub(crate) struct AndroidSecret;

impl SecretExt for AndroidSecret {
    async fn restore_sessions() -> Result<Vec<StoredSession>, SecretError> {
        let handle = spawn_tokio!(async move {
            let _guard = FILE_LOCK.lock().expect("session file lock should not be poisoned");
            load_sessions()
        });

        Ok(handle.await.expect("task was not aborted"))
    }

    async fn store_session(session: StoredSession) -> Result<(), SecretError> {
        let handle = spawn_tokio!(async move {
            let _guard = FILE_LOCK.lock().expect("session file lock should not be poisoned");
            store_session_inner(session)
        });

        handle.await.expect("task was not aborted")
    }

    async fn delete_session(session: &StoredSession) {
        let id = session.id.clone();

        let handle = spawn_tokio!(async move {
            let _guard = FILE_LOCK.lock().expect("session file lock should not be poisoned");
            delete_session_inner(&id);
        });

        handle.await.expect("task was not aborted");
    }
}

/// The path of the file that holds the encrypted sessions.
fn sessions_path() -> PathBuf {
    DataType::Persistent.dir_path().join(SESSIONS_FILE)
}

/// Load the sessions from the file, returning an empty list if there are none
/// or if the stored sessions can no longer be read.
///
/// The system invalidates the key of Fractal when the user changes the security
/// of the device, and the file becomes undecryptable then. That is not an error
/// the user can act on -- the sessions are simply gone -- so it is treated like
/// an empty store, and the useless key and file are removed.
fn load_sessions() -> Vec<StoredSession> {
    match read_sessions() {
        Ok(sessions) => sessions,
        Err(LoadError::NoSessions) => Vec::new(),
        Err(LoadError::Unreadable(error)) => {
            warn!("Could not read stored sessions, starting fresh: {error}");
            forget_everything();
            Vec::new()
        }
        Err(LoadError::UnsupportedVersion(error)) => {
            // The file was written by a later version of Fractal. It must be
            // kept untouched, or downgrading once would destroy the sessions.
            error!("Could not read stored sessions: {error}");
            Vec::new()
        }
        Err(LoadError::Retained(error)) => {
            // The failure might be temporary, so the file is kept for the next
            // attempt instead of being discarded.
            error!("Could not read stored sessions this time: {error}");
            Vec::new()
        }
    }
}

/// Read, decrypt and deserialize the sessions from the file.
fn read_sessions() -> Result<Vec<StoredSession>, LoadError> {
    let bytes = match fs::read(sessions_path()) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(LoadError::NoSessions);
        }
        Err(error) => return Err(LoadError::Unreadable(error.into())),
    };

    let plaintext = keystore::decrypt(&bytes).map_err(|error| {
        if error.is_permanent() {
            LoadError::Unreadable(error.into())
        } else {
            // A transient failure, such as the Keystore being momentarily
            // unavailable. The file must not be discarded over it.
            LoadError::Retained(error.into())
        }
    })?;

    SerializedSessions::decode(&plaintext).map_err(|error| match error {
        SessionsFormatError::UnsupportedVersion(_) => LoadError::UnsupportedVersion(error),
        _ => LoadError::Unreadable(error.into()),
    })
}

/// Store the given session, keeping every other session that is already stored.
fn store_session_inner(session: StoredSession) -> Result<(), SecretError> {
    let mut sessions = match read_sessions() {
        Ok(sessions) => sessions,
        // The previous sessions cannot be read. They are lost either way, so a
        // fresh store is the best that can be done -- except when they were
        // written by a newer Fractal, which must never be overwritten, or when
        // the failure might be temporary, which must not lose the sessions.
        Err(LoadError::NoSessions | LoadError::Unreadable(_)) => Vec::new(),
        Err(error @ (LoadError::UnsupportedVersion(_) | LoadError::Retained(_))) => {
            error!("Could not store session: {error}");
            return Err(service_error());
        }
    };

    // Replace any session with the same identifier, so that storing is
    // idempotent, then add the new one.
    sessions.retain(|stored| stored.id != session.id);
    sessions.push(session);

    write_sessions(&sessions).map_err(|error| {
        error!("Could not store session: {error}");
        service_error()
    })
}

/// Delete the session with the given identifier, keeping every other session.
fn delete_session_inner(id: &str) {
    let mut sessions = match read_sessions() {
        Ok(sessions) => sessions,
        Err(LoadError::NoSessions) => return,
        Err(error) => {
            // The file is unreadable anyway, so there is nothing to delete from
            // and nothing worth keeping.
            warn!("Could not delete session, removing all stored sessions: {error}");
            forget_everything();
            return;
        }
    };

    let before = sessions.len();
    sessions.retain(|stored| stored.id != id);
    if sessions.len() == before {
        // Nothing was stored for this session, so there is nothing to write.
        return;
    }

    if sessions.is_empty() {
        // With no session left, both the file and the key can go, which also
        // guarantees a clean key the next time a session is stored.
        forget_everything();
        return;
    }

    if let Err(error) = write_sessions(&sessions) {
        error!("Could not delete session: {error}");
    }
}

/// Serialize, encrypt and write the given sessions to the file.
///
/// The file is written whole, through a temporary file that is renamed into
/// place, so that a crash halfway cannot leave a truncated, undecryptable file.
fn write_sessions(sessions: &[StoredSession]) -> Result<(), WriteError> {
    let path = sessions_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let plaintext = SerializedSessions::encode(sessions.iter().cloned())?;
    let bytes = keystore::encrypt(&plaintext)?;

    let temp_path = path.with_extension("mpack.tmp");
    fs::write(&temp_path, &bytes)?;
    fs::rename(&temp_path, &path)?;

    Ok(())
}

/// Remove the file and the Keystore key, so that no trace of the sessions is
/// left.
fn forget_everything() {
    if let Err(error) = fs::remove_file(sessions_path())
        && error.kind() != std::io::ErrorKind::NotFound
    {
        error!("Could not remove stored sessions file: {error}");
    }

    if let Err(error) = keystore::delete_key() {
        error!("Could not remove the Android Keystore key: {error}");
    }
}

/// The user-facing error returned when a session cannot be stored.
fn service_error() -> SecretError {
    SecretError::Service(gettext(
        "Sessions could not be saved securely on this device, so you may have to log in again the next time you open Fractal",
    ))
}

/// Why the stored sessions could not be produced.
#[derive(Debug)]
enum LoadError {
    /// There are no stored sessions.
    NoSessions,
    /// The stored sessions cannot be read and never will be, so they are lost.
    Unreadable(SessionStoreError),
    /// The stored sessions could not be read this time, but might be later, so
    /// the file must be kept.
    Retained(SessionStoreError),
    /// The stored sessions were written by a later version of Fractal.
    UnsupportedVersion(SessionsFormatError),
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoSessions => f.write_str("there are no stored sessions"),
            Self::Unreadable(error) | Self::Retained(error) => error.fmt(f),
            Self::UnsupportedVersion(error) => error.fmt(f),
        }
    }
}

/// An error while reading or writing the sessions file.
#[derive(Debug, thiserror::Error)]
enum SessionStoreError {
    /// An input/output error.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// An error with the serialized form of the sessions.
    #[error(transparent)]
    Format(#[from] SessionsFormatError),

    /// An error with the Android Keystore.
    #[error(transparent)]
    Keystore(#[from] KeystoreError),
}

/// An error while writing the sessions file.
///
/// Writing is a strict subset of the store errors, but a distinct alias keeps
/// the signatures honest about what each function can fail with.
type WriteError = SessionStoreError;
