use std::fmt::Write;

use gtk4::prelude::*;
use libadwaita::prelude::*;

use crate::engine::models::{
    Habit, HabitType, DEFAULT_ACCENT_COLOR, GNOME_PALETTE, SCHEDULE_ALL_DAYS, SCHEDULE_FRI,
    SCHEDULE_MON, SCHEDULE_SAT, SCHEDULE_SUN, SCHEDULE_THU, SCHEDULE_TUE, SCHEDULE_WED,
    SCHEDULE_WEEKENDS, SCHEDULE_WORKDAYS,
};

pub(super) struct HabitFormRows {
    pub(super) name_row: libadwaita::EntryRow,
    pub(super) type_row: libadwaita::ComboRow,
    pub(super) target_row: libadwaita::SpinRow,
    pub(super) unit_row: libadwaita::EntryRow,
    pub(super) switch_rows: Vec<(u8, libadwaita::SwitchRow)>,
    pub(super) hex_row: libadwaita::EntryRow,
}

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

fn wire_preset_buttons(
    all_days_btn: &gtk4::Button,
    weekdays_btn: &gtk4::Button,
    weekends_btn: &gtk4::Button,
    switch_rows: &[(u8, libadwaita::SwitchRow)],
) {
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
            s.set_active((*b & SCHEDULE_WORKDAYS) != 0);
        }
    });

    let switches_for_weekends = switch_rows
        .iter()
        .map(|(b, s)| (*b, s.clone()))
        .collect::<Vec<_>>();
    weekends_btn.connect_clicked(move |_| {
        for (b, s) in &switches_for_weekends {
            s.set_active((*b & SCHEDULE_WEEKENDS) != 0);
        }
    });
}

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
    presets_row.set_title("Presets");

    let presets_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    presets_box.set_valign(gtk4::Align::Center);
    let all_days_btn = gtk4::Button::with_label("All Days");
    all_days_btn.add_css_class("quick-preset-btn");
    let weekdays_btn = gtk4::Button::with_label("Weekdays");
    weekdays_btn.add_css_class("quick-preset-btn");
    let weekends_btn = gtk4::Button::with_label("Weekends");
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

    let mut switch_rows = Vec::with_capacity(days_data.len());
    for (day_name, bit) in days_data {
        let switch = libadwaita::SwitchRow::new();
        switch.set_title(day_name);
        switch.set_active((initial_schedule & bit) != 0);
        group.add(&switch);
        switch_rows.push((bit, switch));
    }

    wire_preset_buttons(&all_days_btn, &weekdays_btn, &weekends_btn, &switch_rows);
    (group, switch_rows)
}

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

pub(super) fn build_form_page(
    habit: Option<&Habit>,
) -> (libadwaita::PreferencesPage, HabitFormRows) {
    let pref_page = libadwaita::PreferencesPage::new();

    let (group_general, name_row, type_row, target_row, unit_row) = build_general_group(habit);
    pref_page.add(&group_general);

    let initial_schedule = habit.map_or(SCHEDULE_ALL_DAYS, |h| h.scheduled_days);
    let (group_schedule, switch_rows) = build_schedule_group(initial_schedule);
    pref_page.add(&group_schedule);

    let initial_color = habit.map_or(DEFAULT_ACCENT_COLOR, |h| h.accent_color.as_str());
    let (group_color, hex_row) = build_color_group(initial_color);
    pref_page.add(&group_color);

    let form = HabitFormRows {
        name_row,
        type_row,
        target_row,
        unit_row,
        switch_rows,
        hex_row,
    };
    (pref_page, form)
}
