//! Fallback location viewer for the platforms without libshumate.
//!
//! Fractal shows locations on an interactive map with libshumate, which is not
//! available on every platform. This widget has the same API as the map, but
//! it only shows the coordinates of the location.

use adw::{prelude::*, subclass::prelude::*};
use geo_uri::GeoUri;
use gtk::glib;

use crate::i18n::gettext_f;

mod imp {
    use std::cell::Cell;

    use super::*;

    #[derive(Debug, glib::Properties)]
    #[properties(wrapper_type = super::LocationViewer)]
    pub struct LocationViewer {
        /// The box containing the marker and the coordinates.
        container: gtk::Box,
        /// The marker of the location.
        marker_img: gtk::Image,
        /// The coordinates of the location.
        label: gtk::Label,
        /// Whether to display this location in a compact format.
        #[property(get, set = Self::set_compact, explicit_notify)]
        compact: Cell<bool>,
    }

    impl Default for LocationViewer {
        fn default() -> Self {
            Self {
                container: gtk::Box::builder()
                    .orientation(gtk::Orientation::Vertical)
                    .spacing(6)
                    .halign(gtk::Align::Center)
                    .valign(gtk::Align::Center)
                    .build(),
                marker_img: gtk::Image::builder()
                    .icon_name("map-marker-symbolic")
                    .pixel_size(32)
                    .build(),
                label: gtk::Label::builder()
                    .wrap(true)
                    .justify(gtk::Justification::Center)
                    .build(),
                compact: Cell::default(),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for LocationViewer {
        const NAME: &'static str = "LocationViewer";
        type Type = super::LocationViewer;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.set_css_name("location-viewer");
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for LocationViewer {
        fn constructed(&self) {
            self.parent_constructed();

            self.marker_img.add_css_class("map-marker");
            self.container.append(&self.marker_img);
            self.container.append(&self.label);
            self.obj().set_child(Some(&self.container));
        }
    }

    impl WidgetImpl for LocationViewer {}
    impl BinImpl for LocationViewer {}

    impl LocationViewer {
        /// Set the compact format of this location.
        fn set_compact(&self, compact: bool) {
            if self.compact.get() == compact {
                return;
            }

            self.label.set_visible(!compact);

            self.compact.set(compact);
            self.obj().notify_compact();
        }

        /// Show the provided coordinates.
        pub(super) fn set_location(&self, geo_uri: &GeoUri) {
            let latitude = geo_uri.latitude();
            let longitude = geo_uri.longitude();

            let description = gettext_f(
                "Location at latitude {latitude} and longitude {longitude}",
                &[
                    ("latitude", &latitude.to_string()),
                    ("longitude", &longitude.to_string()),
                ],
            );

            self.label.set_label(&description);
            self.obj()
                .update_property(&[gtk::accessible::Property::Description(&description)]);
        }
    }
}

glib::wrapper! {
    /// A widget displaying a location.
    pub struct LocationViewer(ObjectSubclass<imp::LocationViewer>)
        @extends gtk::Widget, adw::Bin,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl LocationViewer {
    /// Create a new location message.
    pub fn new() -> Self {
        glib::Object::new()
    }

    // Move the map viewport to the provided coordinates and draw a marker.
    pub(crate) fn set_location(&self, geo_uri: &GeoUri) {
        self.imp().set_location(geo_uri);
    }
}

impl Default for LocationViewer {
    fn default() -> Self {
        Self::new()
    }
}
