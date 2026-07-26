//! Host-runnable copy of `panic_message` from `src/platform/android.rs`.
//!
//! Keep this in sync with the platform module.

/// The message a panic carries, whether it was a `&str` or a formatted
/// `String`.
pub fn panic_message(payload: &(dyn std::any::Any + Send)) -> &str {
    if let Some(message) = payload.downcast_ref::<&str>() {
        message
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message
    } else {
        "Box<dyn Any>"
    }
}

#[cfg(test)]
mod tests {
    use std::panic;

    use super::*;

    /// Silence the default hook so the deliberate panics do not clutter the
    /// test output.
    fn without_hook<R>(body: impl FnOnce() -> R + panic::UnwindSafe) -> R {
        let previous = panic::take_hook();
        panic::set_hook(Box::new(|_| {}));
        let result = panic::catch_unwind(body);
        panic::set_hook(previous);
        result.expect("body should return")
    }

    #[test]
    fn panic_message_from_str() {
        let payload = without_hook(|| panic::catch_unwind(|| panic!("a static message")))
            .expect_err("should panic");
        assert_eq!(panic_message(&*payload), "a static message");
    }

    #[test]
    fn panic_message_from_string() {
        let value = 42;
        let payload =
            without_hook(|| panic::catch_unwind(|| panic!("a formatted message: {value}")))
                .expect_err("should panic");
        assert_eq!(panic_message(&*payload), "a formatted message: 42");
    }

    #[test]
    fn panic_message_from_other_payload() {
        let payload = without_hook(|| panic::catch_unwind(|| panic::panic_any(42_u8)))
            .expect_err("should panic");
        assert_eq!(panic_message(&*payload), "Box<dyn Any>");
    }
}
