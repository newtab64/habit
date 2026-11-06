#[path = "form_groups.rs"]
mod form_groups;

use gtk4::prelude::*;
use libadwaita::prelude::*;

use crate::engine::models::Habit;
use crate::ui::habit_dialog::validate_habit_form;
use crate::ui::modal::{build_card_header, build_modal, connect_backdrop_click, Modal};

use form_groups::{build_form_page, HabitFormRows};

const DIALOG_WIDTH: i32 = 440;

fn build_dialog_header(title: &str) -> (gtk4::CenterBox, gtk4::Button, gtk4::Button) {
    let header = build_card_header(16);

    let cancel_btn = gtk4::Button::with_label("Cancel");
    cancel_btn.add_css_class("destructive-action");
    header.set_start_widget(Some(&cancel_btn));
    header.set_center_widget(Some(&libadwaita::WindowTitle::new(title, "")));

    let save_btn = gtk4::Button::with_label("Save");
    save_btn.add_css_class("suggested-action");
    header.set_end_widget(Some(&save_btn));

    (header, cancel_btn, save_btn)
}

fn fill_dialog_card(
    card: &gtk4::Box,
    header: &gtk4::CenterBox,
    pref_page: &libadwaita::PreferencesPage,
) -> libadwaita::ToastOverlay {
    card.append(header);

    let header_sep = gtk4::Separator::new(gtk4::Orientation::Horizontal);
    header_sep.set_margin_bottom(6);
    card.append(&header_sep);

    let toast_overlay = libadwaita::ToastOverlay::new();
    toast_overlay.set_child(Some(pref_page));

    let scrolled = gtk4::ScrolledWindow::new();
    scrolled.set_hscrollbar_policy(gtk4::PolicyType::Never);
    scrolled.set_vscrollbar_policy(gtk4::PolicyType::Automatic);
    scrolled.set_max_content_height(480);
    scrolled.set_propagate_natural_height(true);
    scrolled.set_margin_bottom(12);
    scrolled.set_child(Some(&toast_overlay));
    card.append(&scrolled);

    toast_overlay
}

fn create_close_callback(
    dialog_container: &gtk4::Overlay,
    root_overlay: &gtk4::Overlay,
) -> impl Fn() + Clone + 'static {
    let overlay = root_overlay.clone();
    let container = dialog_container.clone();
    move || {
        if container.parent().is_some() {
            overlay.remove_overlay(&container);
        }
    }
}

fn wire_dismiss_signals(
    modal: &Modal,
    cancel_btn: &gtk4::Button,
    close_dialog: &(impl Fn() + Clone + 'static),
) {
    let close_cancel = close_dialog.clone();
    cancel_btn.connect_clicked(move |_| close_cancel());

    connect_backdrop_click(&modal.backdrop, close_dialog.clone());

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
    modal.container.add_controller(key_controller);
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

        let scheduled_days = form
            .switch_rows
            .iter()
            .filter(|(_, row)| row.is_active())
            .fold(0u8, |mask, (bit, _)| mask | bit);

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

/// Displays a centered card dialog for creating or editing a habit.
///
/// # Arguments
/// * `root_overlay` - The root overlay of the application window where the dialog is shown.
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

    let modal = build_modal(DIALOG_WIDTH);
    let (header, cancel_btn, save_btn) = build_dialog_header(title);
    let (pref_page, form) = build_form_page(habit);
    let name_row = form.name_row.clone();
    let toast_overlay = fill_dialog_card(&modal.card, &header, &pref_page);

    let close_dialog = create_close_callback(&modal.container, root_overlay);
    wire_dismiss_signals(&modal, &cancel_btn, &close_dialog);
    connect_save_button(
        &save_btn,
        form,
        toast_overlay,
        habit.cloned(),
        close_dialog,
        on_save,
    );

    root_overlay.add_overlay(&modal.container);
    name_row.grab_focus();
}
