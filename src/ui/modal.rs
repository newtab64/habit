//! Shared building blocks for the centered card popups (habit dialog, history editor).

use gtk4::prelude::*;

/// Margin kept between a card and the window edges, and inside the card header.
const CARD_MARGIN: i32 = 16;

/// Pieces of a modal: an overlay with a dimming backdrop and a centered card.
pub struct Modal {
    /// Overlay to add on top of the window content.
    pub container: gtk4::Overlay,
    /// Dimming backdrop; clicking it should dismiss the modal.
    pub backdrop: gtk4::Box,
    /// Centered card; append the header and content to it.
    pub card: gtk4::Box,
}

/// Builds a dimming backdrop with a centered card of the given width on top.
#[must_use]
pub fn build_modal(width: i32) -> Modal {
    let backdrop = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    backdrop.set_vexpand(true);
    backdrop.set_hexpand(true);
    backdrop.add_css_class("editor-dimming-backdrop");

    let card = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    card.set_halign(gtk4::Align::Center);
    card.set_valign(gtk4::Align::Center);
    card.set_width_request(width);
    card.set_margin_start(CARD_MARGIN);
    card.set_margin_end(CARD_MARGIN);
    card.set_margin_top(CARD_MARGIN);
    card.set_margin_bottom(CARD_MARGIN);
    card.add_css_class("editor-card");

    let container = gtk4::Overlay::new();
    container.set_child(Some(&backdrop));
    container.add_overlay(&card);

    Modal {
        container,
        backdrop,
        card,
    }
}

/// Builds a card header with `end_margin` on the right, where the end widget sits.
#[must_use]
pub fn build_card_header(end_margin: i32) -> gtk4::CenterBox {
    let header = gtk4::CenterBox::new();
    header.set_margin_start(CARD_MARGIN);
    header.set_margin_end(end_margin);
    header.set_margin_top(12);
    header.set_margin_bottom(10);
    header
}

/// Calls `close` when the backdrop is clicked.
pub fn connect_backdrop_click(backdrop: &gtk4::Box, close: impl Fn() + 'static) {
    let gesture = gtk4::GestureClick::new();
    gesture.connect_pressed(move |_, _, _, _| close());
    backdrop.add_controller(gesture);
}
