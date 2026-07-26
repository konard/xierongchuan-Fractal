//! Encryption of the sessions with a key of the Android Keystore.
//!
//! Android has no Secret Service. What it has is the Keystore: the system
//! generates and holds the key material, and an application can only ask it to
//! encrypt and decrypt. The key of Fractal therefore never enters this process,
//! and the file that holds the sessions is useless to anything that reads it
//! from outside the sandbox of the application.
//!
//! Everything here is JNI. The Java classes are the ones of the platform
//! (`java.security.KeyStore`, `javax.crypto.Cipher`,
//! `android.security.keystore.KeyGenParameterSpec`), so no Java or Kotlin code
//! of ours has to be compiled into the package.

use std::sync::OnceLock;

use gtk::{
    gdk::{Display, ffi::GdkDisplay},
    glib::translate::ToGlibPtr,
};
use jni::{
    JNIEnv, JavaVM,
    objects::{JByteArray, JObject, JString, JValue},
    sys::JNIEnv as RawJNIEnv,
};
use thiserror::Error;

/// The alias of the key of Fractal in the Android Keystore.
const KEY_ALIAS: &str = "org.gnome.Fractal.sessions";
/// The name of the Android Keystore provider.
const KEYSTORE_PROVIDER: &str = "AndroidKeyStore";
/// The transformation used to encrypt the sessions.
///
/// AES-GCM authenticates the ciphertext, so a file that was tampered with fails
/// to decrypt instead of decrypting to something else.
const TRANSFORMATION: &str = "AES/GCM/NoPadding";
/// The class that holds the constants of the Keystore API.
const KEY_PROPERTIES: &str = "android/security/keystore/KeyProperties";
/// The builder of the parameters of a generated key.
const KEY_GEN_PARAMETER_SPEC_BUILDER: &str =
    "android/security/keystore/KeyGenParameterSpec$Builder";
/// The signature of a method of the builder that returns the builder.
const BUILDER_SIGNATURE: &str = "Landroid/security/keystore/KeyGenParameterSpec$Builder;";
/// The size of the generated key, in bits.
const KEY_SIZE: i32 = 256;
/// The size of the authentication tag of AES-GCM, in bits.
const TAG_SIZE: i32 = 128;

unsafe extern "C" {
    /// Get the JNI function table of the current thread.
    ///
    /// This is public GDK API since GTK 4.18, but the `gdk4` crate does not
    /// bind the Android backend, so it is declared here. It only returns the
    /// table of a thread that is attached to the Java virtual machine, which
    /// the thread of the GTK main loop is.
    fn gdk_android_display_get_env(display: *mut GdkDisplay) -> *mut RawJNIEnv;
}

/// The Java virtual machine of the application.
///
/// Unlike the JNI function table, which is thread-local, the virtual machine
/// can be used from any thread, so it is what the tokio tasks that touch the
/// Keystore use. It is captured once, on the thread of the GTK main loop, and
/// only read afterwards.
static JAVA_VM: OnceLock<JavaVM> = OnceLock::new();

/// Capture the Java virtual machine of the application.
///
/// This reaches into the GTK Android backend for the JNI function table of the
/// current thread, so it must run on the thread of the GTK main loop, after
/// `gtk::init()`. It is idempotent, and a failure only means that the Keystore
/// is unavailable, which the callers already handle.
pub(super) fn init() -> Result<(), KeystoreError> {
    if JAVA_VM.get().is_some() {
        return Ok(());
    }

    let display = Display::default().ok_or(KeystoreError::NoJavaVm)?;
    // SAFETY: the display is the one of the GTK Android backend, and the
    // returned table belongs to this thread, which the runtime attached to the
    // virtual machine before it called into Fractal.
    let raw_env = unsafe { gdk_android_display_get_env(display.to_glib_none().0) };
    if raw_env.is_null() {
        return Err(KeystoreError::NoJavaVm);
    }
    // SAFETY: the pointer is a JNI function table of this thread.
    let env = unsafe { JNIEnv::from_raw(raw_env) }.map_err(|_| KeystoreError::NoJavaVm)?;

    let java_vm = env.get_java_vm()?;
    let _ = JAVA_VM.set(java_vm);
    Ok(())
}

/// The Java virtual machine of the application, captured by [`init()`].
///
/// This only reads the value that [`init()`] stored, so it is safe to call from
/// any thread. It never reaches for a GTK object, which would be wrong off the
/// main thread.
fn java_vm() -> Result<&'static JavaVM, KeystoreError> {
    JAVA_VM.get().ok_or(KeystoreError::NoJavaVm)
}

/// Encrypt the given bytes with the key of Fractal in the Android Keystore,
/// creating that key if it does not exist yet.
///
/// The initialization vector that the system chose is part of the result.
pub(super) fn encrypt(plaintext: &[u8]) -> Result<Vec<u8>, KeystoreError> {
    let (iv, ciphertext) = with_env(java_vm()?, |env| {
        let key = get_or_create_key(env)?;
        let cipher = cipher_instance(env)?;

        let mode = env
            .get_static_field("javax/crypto/Cipher", "ENCRYPT_MODE", "I")?
            .i()?;
        env.call_method(
            &cipher,
            "init",
            "(ILjava/security/Key;)V",
            &[JValue::Int(mode), (&key).into()],
        )?;

        let input = env.byte_array_from_slice(plaintext)?;
        let output = env
            .call_method(&cipher, "doFinal", "([B)[B", &[(&input).into()])?
            .l()?;
        let iv = env.call_method(&cipher, "getIV", "()[B", &[])?.l()?;

        Ok((byte_array(env, iv)?, byte_array(env, output)?))
    })?;

    let iv_len = u8::try_from(iv.len()).map_err(|_| KeystoreError::MalformedCiphertext)?;

    let mut bytes = Vec::with_capacity(2 + iv.len() + ciphertext.len());
    bytes.push(FORMAT_VERSION);
    bytes.push(iv_len);
    bytes.extend_from_slice(&iv);
    bytes.extend_from_slice(&ciphertext);

    Ok(bytes)
}

/// Decrypt bytes produced by [`encrypt()`] with the key of Fractal in the
/// Android Keystore.
pub(super) fn decrypt(bytes: &[u8]) -> Result<Vec<u8>, KeystoreError> {
    let [version, iv_len, rest @ ..] = bytes else {
        return Err(KeystoreError::MalformedCiphertext);
    };
    if *version != FORMAT_VERSION {
        return Err(KeystoreError::MalformedCiphertext);
    }
    let (iv, ciphertext) = rest
        .split_at_checked((*iv_len).into())
        .ok_or(KeystoreError::MalformedCiphertext)?;

    with_env(java_vm()?, |env| {
        let key = existing_key(env)?;
        if key.is_null() {
            return Err(JavaError::Keystore(KeystoreError::NoKey));
        }

        let cipher = cipher_instance(env)?;
        let iv = env.byte_array_from_slice(iv)?;
        let parameters = env.new_object(
            "javax/crypto/spec/GCMParameterSpec",
            "(I[B)V",
            &[JValue::Int(TAG_SIZE), (&iv).into()],
        )?;

        let mode = env
            .get_static_field("javax/crypto/Cipher", "DECRYPT_MODE", "I")?
            .i()?;
        env.call_method(
            &cipher,
            "init",
            "(ILjava/security/Key;Ljava/security/spec/AlgorithmParameterSpec;)V",
            &[JValue::Int(mode), (&key).into(), (&parameters).into()],
        )?;

        let input = env.byte_array_from_slice(ciphertext)?;
        let output = env
            .call_method(&cipher, "doFinal", "([B)[B", &[(&input).into()])?
            .l()?;

        byte_array(env, output)
    })
}

/// Delete the key of Fractal from the Android Keystore.
///
/// Without the key, the file that holds the sessions cannot be decrypted by
/// anyone, so this is what makes forgetting the sessions final.
pub(super) fn delete_key() -> Result<(), KeystoreError> {
    with_env(java_vm()?, |env| {
        let keystore = load_keystore(env)?;
        let alias = env.new_string(KEY_ALIAS)?;
        env.call_method(
            &keystore,
            "deleteEntry",
            "(Ljava/lang/String;)V",
            &[(&alias).into()],
        )?;
        Ok(())
    })
}

/// The version of the layout of the encrypted bytes.
const FORMAT_VERSION: u8 = 1;

/// Run the given JNI calls on this thread.
///
/// The thread is attached to the virtual machine for the duration of the call,
/// and a Java exception left pending by a failed call is turned into an error
/// and cleared: leaving it pending would abort the process at the next JNI
/// call, wherever that happens to be.
fn with_env<T>(
    java_vm: &JavaVM,
    call: impl FnOnce(&mut JNIEnv<'_>) -> Result<T, JavaError>,
) -> Result<T, KeystoreError> {
    let mut env = java_vm.attach_current_thread()?;

    match call(&mut env) {
        Ok(value) => Ok(value),
        Err(error) => {
            let exception = take_exception(&mut env);
            Err(match (error, exception) {
                (_, Some(exception)) => KeystoreError::Java(exception),
                (JavaError::Keystore(error), None) => error,
                (JavaError::Jni(error), None) => error.into(),
            })
        }
    }
}

/// Describe the Java exception that is pending on this thread, if any, and
/// clear it.
fn take_exception(env: &mut JNIEnv<'_>) -> Option<String> {
    if !env.exception_check().unwrap_or(false) {
        return None;
    }

    let throwable = env.exception_occurred().ok();
    // The exception must be cleared before any other call, including the one
    // that asks it for its message.
    if env.exception_clear().is_err() {
        return Some("unknown Java exception".to_owned());
    }

    let description = throwable.and_then(|throwable| {
        let string = env
            .call_method(&throwable, "toString", "()Ljava/lang/String;", &[])
            .ok()?
            .l()
            .ok()?;
        env.get_string(&JString::from(string)).ok().map(Into::into)
    });

    Some(description.unwrap_or_else(|| "unknown Java exception".to_owned()))
}

/// Get a `Cipher` for the transformation used by Fractal.
fn cipher_instance<'local>(env: &mut JNIEnv<'local>) -> Result<JObject<'local>, JavaError> {
    let transformation = env.new_string(TRANSFORMATION)?;
    Ok(env
        .call_static_method(
            "javax/crypto/Cipher",
            "getInstance",
            "(Ljava/lang/String;)Ljavax/crypto/Cipher;",
            &[(&transformation).into()],
        )?
        .l()?)
}

/// Get the Android Keystore, loaded and ready to be queried.
fn load_keystore<'local>(env: &mut JNIEnv<'local>) -> Result<JObject<'local>, JavaError> {
    let provider = env.new_string(KEYSTORE_PROVIDER)?;
    let keystore = env
        .call_static_method(
            "java/security/KeyStore",
            "getInstance",
            "(Ljava/lang/String;)Ljava/security/KeyStore;",
            &[(&provider).into()],
        )?
        .l()?;

    // The Android Keystore has no store to load from and no password: `null` is
    // what its documentation asks for.
    env.call_method(
        &keystore,
        "load",
        "(Ljava/security/KeyStore$LoadStoreParameter;)V",
        &[(&JObject::null()).into()],
    )?;

    Ok(keystore)
}

/// Get the key of Fractal from the Android Keystore.
///
/// Returns a null object when the key does not exist, which is the case before
/// the first session is stored and after the key was deleted or invalidated.
fn existing_key<'local>(env: &mut JNIEnv<'local>) -> Result<JObject<'local>, JavaError> {
    let keystore = load_keystore(env)?;
    let alias = env.new_string(KEY_ALIAS)?;

    Ok(env
        .call_method(
            &keystore,
            "getKey",
            "(Ljava/lang/String;[C)Ljava/security/Key;",
            &[(&alias).into(), (&JObject::null()).into()],
        )?
        .l()?)
}

/// Get the key of Fractal from the Android Keystore, generating it if it does
/// not exist yet.
fn get_or_create_key<'local>(env: &mut JNIEnv<'local>) -> Result<JObject<'local>, JavaError> {
    let key = existing_key(env)?;
    if !key.is_null() {
        return Ok(key);
    }

    let algorithm = env.new_string("AES")?;
    let provider = env.new_string(KEYSTORE_PROVIDER)?;
    let generator = env
        .call_static_method(
            "javax/crypto/KeyGenerator",
            "getInstance",
            "(Ljava/lang/String;Ljava/lang/String;)Ljavax/crypto/KeyGenerator;",
            &[(&algorithm).into(), (&provider).into()],
        )?
        .l()?;

    let purposes = env
        .get_static_field(KEY_PROPERTIES, "PURPOSE_ENCRYPT", "I")?
        .i()?
        | env
            .get_static_field(KEY_PROPERTIES, "PURPOSE_DECRYPT", "I")?
            .i()?;
    let alias = env.new_string(KEY_ALIAS)?;
    let builder = env.new_object(
        KEY_GEN_PARAMETER_SPEC_BUILDER,
        "(Ljava/lang/String;I)V",
        &[(&alias).into(), JValue::Int(purposes)],
    )?;

    let builder = call_builder_with_strings(env, &builder, "setBlockModes", "BLOCK_MODE_GCM")?;
    let builder = call_builder_with_strings(
        env,
        &builder,
        "setEncryptionPaddings",
        "ENCRYPTION_PADDING_NONE",
    )?;
    let builder = env
        .call_method(
            &builder,
            "setKeySize",
            &format!("(I){BUILDER_SIGNATURE}"),
            &[JValue::Int(KEY_SIZE)],
        )?
        .l()?;

    let spec = env
        .call_method(
            &builder,
            "build",
            "()Landroid/security/keystore/KeyGenParameterSpec;",
            &[],
        )?
        .l()?;

    env.call_method(
        &generator,
        "init",
        "(Ljava/security/spec/AlgorithmParameterSpec;)V",
        &[(&spec).into()],
    )?;

    Ok(env
        .call_method(&generator, "generateKey", "()Ljavax/crypto/SecretKey;", &[])?
        .l()?)
}

/// Call the given method of the given `KeyGenParameterSpec.Builder` with an
/// array holding the single string constant of `KeyProperties` with the given
/// name.
fn call_builder_with_strings<'local>(
    env: &mut JNIEnv<'local>,
    builder: &JObject<'_>,
    method: &str,
    constant: &str,
) -> Result<JObject<'local>, JavaError> {
    let value = env
        .get_static_field(KEY_PROPERTIES, constant, "Ljava/lang/String;")?
        .l()?;
    let values = env.new_object_array(1, "java/lang/String", &value)?;

    Ok(env
        .call_method(
            builder,
            method,
            &format!("([Ljava/lang/String;){BUILDER_SIGNATURE}"),
            &[(&values).into()],
        )?
        .l()?)
}

/// Copy the given Java byte array into a vector.
fn byte_array(env: &mut JNIEnv<'_>, array: JObject<'_>) -> Result<Vec<u8>, JavaError> {
    // SAFETY: the object is the byte array returned by the method that was
    // called, as declared by its signature.
    let array = unsafe { JByteArray::from_raw(array.into_raw()) };
    Ok(env.convert_byte_array(&array)?)
}

/// An error that interrupted a sequence of JNI calls.
///
/// It exists so that the calls can use `?`, and so that a failure of the
/// Keystore that is not a JNI failure can travel the same way.
#[derive(Debug, Error)]
enum JavaError {
    /// A JNI call failed.
    #[error(transparent)]
    Jni(#[from] jni::errors::Error),

    /// The Keystore is not in the expected state.
    #[error(transparent)]
    Keystore(#[from] KeystoreError),
}

/// All errors that can occur when using the Android Keystore.
#[derive(Debug, Error)]
pub(super) enum KeystoreError {
    /// The Java virtual machine of the application could not be found.
    #[error("The Java virtual machine of the application is not available")]
    NoJavaVm,

    /// The key of Fractal is not in the Keystore.
    ///
    /// The system removes it when the user disables the screen lock or enrolls
    /// a new biometric, and it is also gone after the data of the application
    /// was cleared.
    #[error("The encryption key of Fractal is not in the Android Keystore")]
    NoKey,

    /// The stored bytes are not what [`encrypt()`] produces.
    #[error("The stored sessions are malformed")]
    MalformedCiphertext,

    /// A Java exception was raised.
    #[error("Android Keystore error: {0}")]
    Java(String),

    /// A JNI call failed.
    #[error("JNI error: {0}")]
    Jni(#[from] jni::errors::Error),
}

impl KeystoreError {
    /// Whether this error means that the stored sessions can never be read
    /// again.
    ///
    /// The system invalidates the key of an application on its own, and a file
    /// that cannot be decrypted is not going to become decryptable later, so
    /// the only thing left to do with it is to remove it.
    pub(super) fn is_permanent(&self) -> bool {
        match self {
            Self::NoKey | Self::MalformedCiphertext => true,
            // `AEADBadTagException` for a file that does not match the key, and
            // `KeyPermanentlyInvalidatedException` for a key that the system
            // dropped, are both raised by `Cipher`.
            Self::Java(message) => {
                message.contains("AEADBadTagException")
                    || message.contains("KeyPermanentlyInvalidatedException")
                    || message.contains("BadPaddingException")
            }
            Self::NoJavaVm | Self::Jni(_) => false,
        }
    }
}
