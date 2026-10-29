#[path = "form_groups.rs"]
mod form_groups;

use gtk4::prelude::*;
use libadwaita::prelude::*;

use crate::engine::models::Habit;
use crate::ui::habit_dialog::validate_habit_form;

use form_groups::{build_form_page, HabitFormRows};

fn build_sheet_header(title: &str) -> (gtk4::CenterBox, gtk4::Button, gtk4::Button) {
    let sheet_header = gtk4::CenterBox::new();
    sheet_header.add_css_class("editor-sheet-header");
    sheet_header.set_margin_start(16);
    sheet_header.set_margin_end(16);
    sheet_header.set_margin_top(12);
    sheet_header.set_margin_bottom(10);

    let cancel_btn = gtk4::Button::with_label("Cancel");
    cancel_btn.add_css_class("destructive-action");
    sheet_header.set_start_widget(Some(&cancel_btn));

    let window_title = libadwaita::WindowTitle::new(title, "");
    sheet_header.set_center_widget(Some(&window_title));

    let save_btn = gtk4::Button::with_label("Save");
    save_btn.add_css_class("suggested-action");
    sheet_header.set_end_widget(Some(&save_btn));

    (sheet_header, cancel_btn, save_btn)
}

fn build_sheet_revealer(
    sheet_header: &gtk4::CenterBox,
    pref_page: &libadwaita::PreferencesPage,
) -> (libadwaita::ToastOverlay, gtk4::Revealer) {
    let sheet_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    sheet_box.set_valign(gtk4::Align::End);
    sheet_box.set_hexpand(true);
    sheet_box.add_css_class("editor-bottom-sheet");
    sheet_box.append(sheet_header);

    let header_sep = gtk4::Separator::new(gtk4::Orientation::Horizontal);
    header_sep.set_margin_bottom(6);
    sheet_box.append(&header_sep);

    let toast_overlay = libadwaita::ToastOverlay::new();
    toast_overlay.set_child(Some(pref_page));

    let scrolled = gtk4::ScrolledWindow::new();
    scrolled.set_hscrollbar_policy(gtk4::PolicyType::Never);
    scrolled.set_vscrollbar_policy(gtk4::PolicyType::Automatic);
    scrolled.set_max_content_height(480);
    scrolled.set_propagate_natural_height(true);
    scrolled.set_margin_bottom(12);
    scrolled.set_child(Some(&toast_overlay));
    sheet_box.append(&scrolled);

    let sheet_revealer = gtk4::Revealer::new();
    sheet_revealer.set_transition_type(gtk4::RevealerTransitionType::SlideUp);
    sheet_revealer.set_transition_duration(180);
    sheet_revealer.set_child(Some(&sheet_box));
    sheet_revealer.set_valign(gtk4::Align::End);
    sheet_revealer.set_hexpand(true);
    sheet_revealer.set_reveal_child(false);

    (toast_overlay, sheet_revealer)
}

fn create_close_callback(
    dialog_container: &gtk4::Box,
    sheet_revealer: &gtk4::Revealer,
    root_overlay: &gtk4::Overlay,
) -> impl Fn() + Clone + 'static {
    let closed = std::rc::Rc::new(std::cell::Cell::new(false));
    let overlay_for_close = root_overlay.clone();
    let container_for_close = dialog_container.clone();
    let revealer_for_close = sheet_revealer.clone();
    move || {
        if !closed.get() {
            closed.set(true);
            revealer_for_close.set_reveal_child(false);
            let overlay_rm = overlay_for_close.clone();
            let cont_rm = container_for_close.clone();
            let removed = std::rc::Rc::new(std::cell::Cell::new(false));
            let removed_clone = removed.clone();
            revealer_for_close.connect_child_revealed_notify(move |rev| {
                if !rev.is_child_revealed() && !removed_clone.get() {
                    removed_clone.set(true);
                    overlay_rm.remove_overlay(&cont_rm);
                }
            });
            if !revealer_for_close.is_child_revealed() && !removed.get() {
                removed.set(true);
                overlay_for_close.remove_overlay(&container_for_close);
            }
        }
    }
}

fn wire_dismiss_signals(
    dialog_container: &gtk4::Box,
    dialog_dimming: &gtk4::Box,
    cancel_btn: &gtk4::Button,
    close_dialog: &(impl Fn() + Clone + 'static),
) {
    let close_cancel = close_dialog.clone();
    cancel_btn.connect_clicked(move |_| {
        close_cancel();
    });

    let close_dimming = close_dialog.clone();
    let dim_gesture = gtk4::GestureClick::new();
    dim_gesture.connect_pressed(move |_, _, _, _| {
        close_dimming();
    });
    dialog_dimming.add_controller(dim_gesture);

    let close_key = close_dialog.clone();
    let key_controller = gtk4::EventControllerKey::new();
    key_controller.connect_key_pressed(move |_, key, _, _| {
        if key == gtk4::gdk::Key::Escape {
            close_key();
            gtk4::glib::Propagation::Stop
        } else {
            gtk4::glib::Propagation::Proceed
        }
    });
    dialog_container.add_controller(key_controller);
}

fn connect_save_button(
    save_btn: &gtk4::Button,
    form: HabitFormRows,
    toast_overlay: libadwaita::ToastOverlay,
    habit_orig: Option<Habit>,
    close_save: impl Fn() + 'static,
    on_save: impl Fn(Habit) + 'static,
) {
    save_btn.connect_clicked(move |_| {
        let name = form.name_row.text().to_string();
        let is_meas = form.type_row.selected() == 1;
        let target = form.target_row.value();
        let unit = form.unit_row.text().to_string();

        let mut scheduled_days = 0u8;
        for (bit, s) in &form.switch_rows {
            if s.is_active() {
                scheduled_days |= *bit;
            }
        }

        let hex_color = form.hex_row.text().to_string();

        match validate_habit_form(&name, is_meas, target, &unit, scheduled_days, &hex_color) {
            Ok(mut validated_habit) => {
                if let Some(ref orig) = habit_orig {
                    validated_habit.id = orig.id;
                    validated_habit.archived = orig.archived;
                    validated_habit.created_at = orig.created_at;
                }
                close_save();
                on_save(validated_habit);
            }
            Err(err) => {
                let toast = libadwaita::Toast::new(&err.to_string());
                toast.set_priority(libadwaita::ToastPriority::High);
                toast_overlay.add_toast(toast);
            }
        }
    });
}

/// Displays an animated bottom sheet dialog for creating or editing a habit.
///
/// # Arguments
/// * `root_overlay` - The root overlay of the application window where this panel docks.
/// * `habit` - `None` when creating a new habit, or `Some(&habit)` when editing.
/// * `on_save` - Callback invoked with the validated `Habit` when confirmed.
pub fn show_habit_dialog(
    root_overlay: &gtk4::Overlay,
    habit: Option<&Habit>,
    on_save: impl Fn(Habit) + 'static,
) {
    let title = if habit.is_some() {
        "Edit Habit"
    } else {
        "New Habit"
    };

    let dialog_container = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    dialog_container.add_css_class("editor-dimming-backdrop");

    let dialog_dimming = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    dialog_dimming.set_vexpand(true);
    dialog_dimming.set_hexpand(true);
    dialog_container.append(&dialog_dimming);

    let (sheet_header, cancel_btn, save_btn) = build_sheet_header(title);
    let (pref_page, form) = build_form_page(habit);
    let (toast_overlay, sheet_revealer) = build_sheet_revealer(&sheet_header, &pref_page);
    dialog_container.append(&sheet_revealer);

    let close_dialog = create_close_callback(&dialog_container, &sheet_revealer, root_overlay);
    wire_dismiss_signals(
        &dialog_container,
        &dialog_dimming,
        &cancel_btn,
        &close_dialog,
    );
    connect_save_button(
        &save_btn,
        form,
        toast_overlay,
        habit.cloned(),
        close_dialog,
        on_save,
    );

    root_overlay.add_overlay(&dialog_container);
    sheet_revealer.set_reveal_child(true);
}
