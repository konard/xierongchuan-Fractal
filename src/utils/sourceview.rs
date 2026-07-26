//! Collection of methods for interacting with the text editing widgets.
//!
//! `GtkSourceView` provides syntax highlighting for the composer and for code
//! blocks, but it is not available on every platform. When the `sourceview`
//! feature is disabled, the plain `GtkTextView` and `GtkTextBuffer` of GTK are
//! used instead: the text can still be read, written and sent, only the syntax
//! highlighting is missing.

use gtk::glib::{self, object::IsA};

cfg_if::cfg_if! {
    if #[cfg(feature = "sourceview")] {
        /// The buffer used by the text views of the application.
        pub(crate) type Buffer = sourceview::Buffer;
        /// The text view used by the application.
        pub(crate) type View = sourceview::View;

        /// The traits needed to interact with a [`Buffer`] and a [`View`].
        pub(crate) mod prelude {
            // Some of these traits are the same as the ones of GTK, so they
            // might already be in scope where this prelude is imported.
            #[allow(unused_imports)]
            pub(crate) use sourceview::prelude::*;
        }

        /// Setup the style scheme for the given buffer.
        pub(crate) fn setup_style_scheme(buffer: &Buffer) {
            use sourceview::prelude::*;

            let manager = adw::StyleManager::default();

            buffer.set_style_scheme(style_scheme().as_ref());

            manager.connect_dark_notify(glib::clone!(
                #[weak]
                buffer,
                move |_| {
                    buffer.set_style_scheme(style_scheme().as_ref());
                }
            ));
        }

        /// Get the style scheme for the current appearance.
        fn style_scheme() -> Option<sourceview::StyleScheme> {
            let manager = adw::StyleManager::default();
            let scheme_name = if manager.is_dark() {
                "Adwaita-dark"
            } else {
                "Adwaita"
            };

            sourceview::StyleSchemeManager::default().scheme(scheme_name)
        }

        /// Set the language used for syntax highlighting in the given buffer.
        ///
        /// The language is identified by the ID used by `GtkSourceView`, like
        /// `markdown`. An unknown language disables syntax highlighting.
        pub(crate) fn set_language(buffer: &Buffer, language_id: Option<&str>) {
            use sourceview::prelude::*;

            let language =
                language_id.and_then(|id| sourceview::LanguageManager::default().language(id));
            buffer.set_language(language.as_ref());
        }

        /// Create a buffer for a code block with the given text.
        pub(crate) fn new_code_buffer(text: &str) -> Buffer {
            let buffer = sourceview::Buffer::builder()
                .highlight_matching_brackets(false)
                .text(text)
                .build();
            setup_style_scheme(&buffer);

            buffer
        }
    } else {
        /// The buffer used by the text views of the application.
        pub(crate) type Buffer = gtk::TextBuffer;
        /// The text view used by the application.
        pub(crate) type View = gtk::TextView;

        /// The traits needed to interact with a [`Buffer`] and a [`View`].
        pub(crate) mod prelude {
            // The traits are the same as the ones of GTK, so they might already
            // be in scope where this prelude is imported.
            #[allow(unused_imports)]
            pub(crate) use gtk::prelude::*;
        }

        /// Setup the style scheme for the given buffer.
        ///
        /// `GtkTextBuffer` has no style scheme, so this does nothing.
        pub(crate) fn setup_style_scheme(_buffer: &Buffer) {}

        /// Set the language used for syntax highlighting in the given buffer.
        ///
        /// `GtkTextBuffer` has no syntax highlighting, so this does nothing.
        pub(crate) fn set_language(_buffer: &Buffer, _language_id: Option<&str>) {}

        /// Create a buffer for a code block with the given text.
        pub(crate) fn new_code_buffer(text: &str) -> Buffer {
            use gtk::prelude::*;

            let buffer = gtk::TextBuffer::new(None);
            buffer.set_text(text);

            buffer
        }
    }
}

/// The property of a buffer that toggles syntax highlighting.
///
/// It is only defined by `GtkSourceBuffer`.
const HIGHLIGHT_SYNTAX_PROPERTY: &str = "highlight-syntax";

/// Bind the given boolean property of `source` to the syntax highlighting of
/// the given buffer.
///
/// Returns the binding, so it can be undone with [`glib::Binding::unbind()`],
/// or `None` when the buffer does not support syntax highlighting.
///
/// The property is looked up first because binding a property that does not
/// exist panics, and a panic in a GObject callback aborts the process. That
/// is what happens with the plain `GtkTextBuffer` used when the `sourceview`
/// feature is disabled.
pub(crate) fn bind_highlight_syntax(
    source: &impl IsA<glib::Object>,
    source_property: &str,
    buffer: &Buffer,
) -> Option<glib::Binding> {
    use gtk::prelude::*;

    if !buffer.has_property(HIGHLIGHT_SYNTAX_PROPERTY) {
        return None;
    }

    let binding = source
        .bind_property(source_property, buffer, HIGHLIGHT_SYNTAX_PROPERTY)
        .sync_create()
        .build();

    Some(binding)
}

#[cfg(test)]
mod tests {
    use gtk::prelude::*;

    use super::*;

    /// Binding the syntax highlighting must work with `GtkSourceBuffer` and
    /// must be a no-op with the plain `GtkTextBuffer` of the fallback, instead
    /// of aborting the process.
    #[test]
    fn bind_highlight_syntax_supports_both_buffers() {
        // Construct the objects with `glib`, so the test does not need GTK to
        // be initialized, which needs a display.
        let source = glib::Object::new::<gtk::TextBuffer>();
        let buffer = glib::Object::new::<Buffer>();

        let binding = bind_highlight_syntax(&source, "enable-undo", &buffer);

        assert_eq!(binding.is_some(), cfg!(feature = "sourceview"));

        if let Some(binding) = binding {
            source.set_enable_undo(false);
            assert!(!buffer.property::<bool>(HIGHLIGHT_SYNTAX_PROPERTY));

            source.set_enable_undo(true);
            assert!(buffer.property::<bool>(HIGHLIGHT_SYNTAX_PROPERTY));

            binding.unbind();
        }
    }
}
