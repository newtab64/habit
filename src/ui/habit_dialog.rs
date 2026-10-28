use std::fmt;
#[cfg(feature = "ui")]
use std::fmt::Write;

use crate::engine::models::{normalize_hex_color, ColorError, Habit, HabitType};

#[cfg(feature = "ui")]
use gtk4::prelude::*;
#[cfg(feature = "ui")]
use libadwaita::prelude::*;

#[cfg(feature = "ui")]
use crate::engine::models::{
    DEFAULT_ACCENT_COLOR, GNOME_PALETTE, SCHEDULE_ALL_DAYS, SCHEDULE_FRI, SCHEDULE_MON,
    SCHEDULE_SAT, SCHEDULE_SUN, SCHEDULE_THU, SCHEDULE_TUE, SCHEDULE_WED, SCHEDULE_WEEKENDS,
    SCHEDULE_WORKDAYS,
};

/// Error variants encountered when validating a habit form submission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HabitValidationError {
    /// Habit display name was empty or whitespace only.
    EmptyName,
    /// Measurable habit target was less than or equal to zero, or not finite.
    InvalidTarget,
    /// No active scheduled days were selected.
    NoScheduledDays,
    /// Hex color format is invalid or malformed.
    InvalidColor(String),
}

impl fmt::Display for HabitValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyName => write!(f, "habit name cannot be empty"),
            Self::InvalidTarget => write!(f, "measurable target must be greater than zero"),
            Self::NoScheduledDays => write!(f, "at least one scheduled day must be selected"),
            Self::InvalidColor(err) => write!(f, "invalid accent color: {err}"),
        }
    }
}

impl std::error::Error for HabitValidationError {}

impl From<ColorError> for HabitValidationError {
    fn from(err: ColorError) -> Self {
        Self::InvalidColor(err.to_string())
    }
}

/// Pure validation logic for habit creation and edit forms.
///
/// Ensures the name is non-empty, the scheduled days bitmask contains at least one day,
/// any measurable target is strictly greater than zero and finite, and the accent color
/// is normalized to standard 6-digit `#RRGGBB` format.
///
/// # Errors
/// Returns [`HabitValidationError::EmptyName`] if `name` is empty or only whitespace,
/// [`HabitValidationError::InvalidTarget`] if `is_measurable` is true and `target <= 0.0` or non-finite,
/// [`HabitValidationError::NoScheduledDays`] if `scheduled_days == 0`, or
/// [`HabitValidationError::InvalidColor`] if `hex_color` cannot be parsed into a 6-digit hex string.
pub fn validate_habit_form(
    name: &str,
    is_measurable: bool,
    target: f64,
    unit: &str,
    scheduled_days: u8,
    hex_color: &str,
) -> Result<Habit, HabitValidationError> {
    let trimmed_name = name.trim();
    if trimmed_name.is_empty() {
        return Err(HabitValidationError::EmptyName);
    }

    let habit_type = if is_measurable {
        if target <= 0.0 || !target.is_finite() {
            return Err(HabitValidationError::InvalidTarget);
        }
        HabitType::Measurable {
            target,
            unit: unit.trim().to_string(),
        }
    } else {
        HabitType::Binary
    };

    if scheduled_days == 0 {
        return Err(HabitValidationError::NoScheduledDays);
    }

    let normalized_color = normalize_hex_color(hex_color)?;

    Ok(Habit {
        id: None,
        name: trimmed_name.to_string(),
        habit_type,
        accent_color: normalized_color,
        scheduled_days,
        archived: false,
        created_at: 0,
    })
}

#[cfg(feature = "ui")]
fn build_general_group(
    habit: Option<&Habit>,
) -> (
    libadwaita::PreferencesGroup,
    libadwaita::EntryRow,
    libadwaita::ComboRow,
    libadwaita::SpinRow,
    libadwaita::EntryRow,
) {
    let group = libadwaita::PreferencesGroup::new();
    group.set_title("General");

    let name_row = libadwaita::EntryRow::new();
    name_row.set_title("Name");
    if let Some(h) = habit {
        name_row.set_text(&h.name);
    }
    group.add(&name_row);

    let type_model =
        gtk4::StringList::new(&["Binary (Done / Not Done)", "Measurable (Numerical Goal)"]);
    let type_row = libadwaita::ComboRow::new();
    type_row.set_title("Habit Type");
    type_row.set_model(Some(&type_model));

    let (is_init_meas, init_target, init_unit) = habit.map_or_else(
        || (false, 10.0, "pages".to_string()),
        |h| match &h.habit_type {
            HabitType::Binary => (false, 10.0, "pages".to_string()),
            HabitType::Measurable { target, unit } => (true, *target, unit.clone()),
        },
    );

    type_row.set_selected(u32::from(is_init_meas));
    group.add(&type_row);

    let target_row = libadwaita::SpinRow::with_range(0.1, 1_000_000.0, 1.0);
    target_row.set_title("Target Value");
    target_row.set_digits(1);
    target_row.set_value(init_target);
    target_row.set_visible(is_init_meas);
    group.add(&target_row);

    let unit_row = libadwaita::EntryRow::new();
    unit_row.set_title("Unit");
    unit_row.set_text(&init_unit);
    unit_row.set_visible(is_init_meas);
    group.add(&unit_row);

    let target_clone = target_row.clone();
    let unit_clone = unit_row.clone();
    type_row.connect_selected_notify(move |row| {
        let is_meas = row.selected() == 1;
        target_clone.set_visible(is_meas);
        unit_clone.set_visible(is_meas);
    });

    (group, name_row, type_row, target_row, unit_row)
}

#[cfg(feature = "ui")]
fn build_schedule_group(
    initial_schedule: u8,
) -> (
    libadwaita::PreferencesGroup,
    Vec<(u8, libadwaita::SwitchRow)>,
) {
    let group = libadwaita::PreferencesGroup::new();
    group.set_title("Schedule &amp; Rest Days");
    group.set_description(Some(
        "Unchecked days are pre-defined rest days (streaks preserved with no penalty)",
    ));

    let presets_row = libadwaita::ActionRow::new();
    presets_row.set_title("Quick Presets");

    let presets_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 2);
    presets_box.set_valign(gtk4::Align::Center);
    let all_days_btn = gtk4::Button::with_label("All Days");
    all_days_btn.add_css_class("flat");
    all_days_btn.add_css_class("caption");
    all_days_btn.add_css_class("quick-preset-btn");
    let weekdays_btn = gtk4::Button::with_label("Weekdays");
    weekdays_btn.add_css_class("flat");
    weekdays_btn.add_css_class("caption");
    weekdays_btn.add_css_class("quick-preset-btn");
    let weekends_btn = gtk4::Button::with_label("Weekends");
    weekends_btn.add_css_class("flat");
    weekends_btn.add_css_class("caption");
    weekends_btn.add_css_class("quick-preset-btn");

    presets_box.append(&all_days_btn);
    presets_box.append(&weekdays_btn);
    presets_box.append(&weekends_btn);
    presets_row.add_suffix(&presets_box);
    group.add(&presets_row);

    let days_data = [
        ("Monday", SCHEDULE_MON),
        ("Tuesday", SCHEDULE_TUE),
        ("Wednesday", SCHEDULE_WED),
        ("Thursday", SCHEDULE_THU),
        ("Friday", SCHEDULE_FRI),
        ("Saturday", SCHEDULE_SAT),
        ("Sunday", SCHEDULE_SUN),
    ];

    let mut switch_rows = Vec::new();
    for (day_name, bit) in days_data {
        let switch = libadwaita::SwitchRow::new();
        switch.set_title(day_name);
        switch.set_active((initial_schedule & bit) != 0);
        group.add(&switch);
        switch_rows.push((bit, switch));
    }

    let switches_for_all = switch_rows
        .iter()
        .map(|(_, s)| s.clone())
        .collect::<Vec<_>>();
    all_days_btn.connect_clicked(move |_| {
        for s in &switches_for_all {
            s.set_active(true);
        }
    });

    let switches_for_work = switch_rows
        .iter()
        .map(|(b, s)| (*b, s.clone()))
        .collect::<Vec<_>>();
    weekdays_btn.connect_clicked(move |_| {
        for (b, s) in &switches_for_work {
            let is_workday = (*b & SCHEDULE_WORKDAYS) != 0;
            s.set_active(is_workday);
        }
    });

    let switches_for_weekends = switch_rows
        .iter()
        .map(|(b, s)| (*b, s.clone()))
        .collect::<Vec<_>>();
    weekends_btn.connect_clicked(move |_| {
        for (b, s) in &switches_for_weekends {
            let is_weekend = (*b & SCHEDULE_WEEKENDS) != 0;
            s.set_active(is_weekend);
        }
    });

    (group, switch_rows)
}

#[cfg(feature = "ui")]
fn build_color_group(initial_color: &str) -> (libadwaita::PreferencesGroup, libadwaita::EntryRow) {
    let group = libadwaita::PreferencesGroup::new();
    group.set_title("Accent Color");

    let hex_row = libadwaita::EntryRow::new();
    hex_row.set_title("Hex Code");
    hex_row.set_text(initial_color);
    group.add(&hex_row);

    let palette_row = libadwaita::ActionRow::new();
    palette_row.set_title("Preset Palette");

    let palette_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    palette_box.set_valign(gtk4::Align::Center);

    let mut css = String::new();
    for (idx, color) in GNOME_PALETTE.iter().enumerate() {
        let _ = writeln!(
            css,
            "button.color-swatch-{idx} {{ background: {color}; border-radius: 9999px; min-width: 24px; min-height: 24px; padding: 0; border: 1px solid rgba(0,0,0,0.2); }}"
        );
    }
    let provider = gtk4::CssProvider::new();
    provider.load_from_data(&css);
    if let Some(display) = gtk4::gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }

    for (idx, color) in GNOME_PALETTE.iter().enumerate() {
        let swatch = gtk4::Button::new();
        swatch.set_tooltip_text(Some(color));
        swatch.add_css_class(&format!("color-swatch-{idx}"));
        swatch.add_css_class("circular");
        swatch.add_css_class("flat");

        let hex_clone = hex_row.clone();
        let color_str = (*color).to_string();
        swatch.connect_clicked(move |_| {
            hex_clone.set_text(&color_str);
        });

        palette_box.append(&swatch);
    }

    palette_row.add_suffix(&palette_box);
    group.add(&palette_row);

    (group, hex_row)
}

/// Presents an in-window bottom-sheet panel for creating a new habit or editing an existing habit.
///
/// # Arguments
/// * `root_overlay` - The root overlay of the application window where this panel docks.
/// * `habit` - `None` when creating a new habit, or `Some(&habit)` when editing.
/// * `on_save` - Callback invoked with the validated `Habit` when confirmed.
#[cfg(feature = "ui")]
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

#[cfg(feature = "ui")]
fn setup_dialog_dismiss(
    dialog_container: &gtk4::Box,
    dialog_dimming: &gtk4::Box,
    sheet_revealer: &gtk4::Revealer,
    cancel_btn: &gtk4::Button,
    root_overlay: &gtk4::Overlay,
) -> impl Fn() + Clone + 'static {
    let closed = std::rc::Rc::new(std::cell::RefCell::new(false));
    let overlay_for_close = root_overlay.clone();
    let container_for_close = dialog_container.clone();
    let revealer_for_close = sheet_revealer.clone();
    let close_dialog = move || {
        let mut is_closed = closed.borrow_mut();
        if !*is_closed {
            *is_closed = true;
            revealer_for_close.set_reveal_child(false);
            let overlay_rm = overlay_for_close.clone();
            let cont_rm = container_for_close.clone();
            let removed = std::rc::Rc::new(std::cell::RefCell::new(false));
            let removed_clone = removed.clone();
            revealer_for_close.connect_child_revealed_notify(move |rev| {
                if !rev.is_child_revealed() && !*removed_clone.borrow() {
                    *removed_clone.borrow_mut() = true;
                    overlay_rm.remove_overlay(&cont_rm);
                }
            });
            if !revealer_for_close.is_child_revealed() && !*removed.borrow() {
                *removed.borrow_mut() = true;
                overlay_for_close.remove_overlay(&container_for_close);
            }
        }
    };

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

    close_dialog
}

#[cfg(feature = "ui")]
struct HabitFormRows {
    name_row: libadwaita::EntryRow,
    type_row: libadwaita::ComboRow,
    target_row: libadwaita::SpinRow,
    unit_row: libadwaita::EntryRow,
    switch_rows: Vec<(u8, libadwaita::SwitchRow)>,
    hex_row: libadwaita::EntryRow,
}

#[cfg(feature = "ui")]
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
#[cfg(feature = "ui")]
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

    let sheet_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    sheet_box.set_valign(gtk4::Align::End);
    sheet_box.set_hexpand(true);
    sheet_box.add_css_class("editor-bottom-sheet");

    let (sheet_header, cancel_btn, save_btn) = build_sheet_header(title);
    sheet_box.append(&sheet_header);

    let header_sep = gtk4::Separator::new(gtk4::Orientation::Horizontal);
    header_sep.set_margin_bottom(6);
    sheet_box.append(&header_sep);

    let pref_page = libadwaita::PreferencesPage::new();

    // Group 1: General
    let (group_general, name_row, type_row, target_row, unit_row) = build_general_group(habit);
    pref_page.add(&group_general);

    // Group 2: Schedule & Rest Days
    let initial_schedule = habit.map_or(SCHEDULE_ALL_DAYS, |h| h.scheduled_days);
    let (group_schedule, switch_rows) = build_schedule_group(initial_schedule);
    pref_page.add(&group_schedule);

    // Group 3: Accent Color
    let initial_color = habit.map_or(DEFAULT_ACCENT_COLOR, |h| h.accent_color.as_str());
    let (group_color, hex_row) = build_color_group(initial_color);
    pref_page.add(&group_color);

    let toast_overlay = libadwaita::ToastOverlay::new();
    toast_overlay.set_child(Some(&pref_page));

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

    dialog_container.append(&sheet_revealer);

    let close_dialog = setup_dialog_dismiss(
        &dialog_container,
        &dialog_dimming,
        &sheet_revealer,
        &cancel_btn,
        root_overlay,
    );

    let form = HabitFormRows {
        name_row,
        type_row,
        target_row,
        unit_row,
        switch_rows,
        hex_row,
    };
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
