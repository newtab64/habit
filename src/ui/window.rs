use chrono::NaiveDate;

#[cfg(feature = "ui")]
use std::cell::RefCell;
#[cfg(feature = "ui")]
use std::collections::HashMap;
#[cfg(feature = "ui")]
use std::rc::Rc;

#[cfg(feature = "ui")]
use chrono::Datelike;
#[cfg(feature = "ui")]
use gtk4::prelude::*;
#[cfg(feature = "ui")]
use libadwaita::prelude::*;

use crate::engine::models::{
    EntryStatus, Habit, HabitEntry, HabitType, SCHEDULE_ALL_DAYS, SCHEDULE_FRI, SCHEDULE_MON,
    SCHEDULE_SAT, SCHEDULE_SUN, SCHEDULE_THU, SCHEDULE_TUE, SCHEDULE_WED, SCHEDULE_WEEKENDS,
    SCHEDULE_WORKDAYS,
};
use crate::engine::scoring::{calculate_habit_strength, calculate_streaks};
use crate::ui::habit_row::format_measurable_value;

#[cfg(feature = "ui")]
use crate::engine::db::Database;
#[cfg(feature = "ui")]
use crate::engine::scoring::calculate_strength_history;
#[cfg(feature = "ui")]
use crate::ui::charts::{
    days_in_month, draw_monthly_heatmap, draw_strength_curve, heatmap_hit_test, Rgba,
};
#[cfg(feature = "ui")]
use crate::ui::habit_dialog::show_habit_dialog;
#[cfg(feature = "ui")]
use crate::ui::habit_row::HabitRowWidget;
#[cfg(feature = "ui")]
use crate::ui::portal::{trigger_export_dialog, trigger_import_dialog};

/// Navigation state for the monthly calendar heatmap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MonthNavigation {
    /// Four-digit calendar year (e.g. 2026).
    pub year: i32,
    /// Calendar month (1 through 12).
    pub month: u32,
}

impl MonthNavigation {
    /// Creates a new `MonthNavigation` instance, clamping the month to `1..=12`.
    #[must_use]
    pub fn new(year: i32, month: u32) -> Self {
        let valid_month = month.clamp(1, 12);
        Self {
            year,
            month: valid_month,
        }
    }

    /// Transitions to the previous calendar month, rolling over to December of the previous year if needed.
    #[must_use]
    pub const fn prev(&self) -> Self {
        if self.month <= 1 {
            Self {
                year: self.year - 1,
                month: 12,
            }
        } else {
            Self {
                year: self.year,
                month: self.month - 1,
            }
        }
    }

    /// Transitions to the next calendar month, rolling over to January of the following year if needed.
    #[must_use]
    pub const fn next(&self) -> Self {
        if self.month >= 12 {
            Self {
                year: self.year + 1,
                month: 1,
            }
        } else {
            Self {
                year: self.year,
                month: self.month + 1,
            }
        }
    }

    /// Formats the month and year into a human-readable title (e.g. "October 2026").
    #[must_use]
    pub fn format_title(&self) -> String {
        let month_name = match self.month {
            1 => "January",
            2 => "February",
            3 => "March",
            4 => "April",
            5 => "May",
            6 => "June",
            7 => "July",
            8 => "August",
            9 => "September",
            10 => "October",
            11 => "November",
            12 => "December",
            _ => "Unknown",
        };
        format!("{month_name} {}", self.year)
    }
}

/// Formats a weekday bitmask into human-readable schedule text.
///
/// For example:
/// - All 7 days -> `"Daily"`
/// - Mon-Fri -> `"Weekdays (Mon-Fri)"`
/// - Sat-Sun -> `"Weekends (Sat-Sun)"`
/// - Mon, Wed, Fri, Sat -> `"Mon, Wed, Fri, Sat"`
#[must_use]
pub fn format_schedule_summary(scheduled_days: u8) -> String {
    if scheduled_days == SCHEDULE_ALL_DAYS {
        return "Daily".to_string();
    }
    if scheduled_days == SCHEDULE_WORKDAYS {
        return "Weekdays (Mon-Fri)".to_string();
    }
    if scheduled_days == SCHEDULE_WEEKENDS {
        return "Weekends (Sat-Sun)".to_string();
    }
    if scheduled_days == 0 {
        return "None".to_string();
    }

    let mut days = Vec::new();
    if (scheduled_days & SCHEDULE_MON) != 0 {
        days.push("Mon");
    }
    if (scheduled_days & SCHEDULE_TUE) != 0 {
        days.push("Tue");
    }
    if (scheduled_days & SCHEDULE_WED) != 0 {
        days.push("Wed");
    }
    if (scheduled_days & SCHEDULE_THU) != 0 {
        days.push("Thu");
    }
    if (scheduled_days & SCHEDULE_FRI) != 0 {
        days.push("Fri");
    }
    if (scheduled_days & SCHEDULE_SAT) != 0 {
        days.push("Sat");
    }
    if (scheduled_days & SCHEDULE_SUN) != 0 {
        days.push("Sun");
    }
    days.join(", ")
}

/// Formats habit strength in `[0.0, 1.0]` into a rounded percentage string (e.g. `"88%"`).
#[must_use]
pub fn format_strength_percentage(strength: f64) -> String {
    let clamped = strength.clamp(0.0, 1.0);
    let pct = (clamped * 100.0).round();
    format!("{pct:.0}%")
}

/// Formats a streak count into a badge string (e.g. `"4d"`).
#[must_use]
pub fn format_streak_badge(streak: u32) -> String {
    format!("{streak}d")
}

/// Formats the target and unit description for a habit (e.g. `"1 session"` or `"30 pages"`).
#[must_use]
pub fn format_target_summary(habit: &Habit) -> String {
    match &habit.habit_type {
        HabitType::Binary => "1 session".to_string(),
        HabitType::Measurable { target, unit } => {
            format!("{} {unit}", format_measurable_value(*target))
        }
    }
}

/// Computed analytics metrics for a habit.
#[derive(Debug, Clone, PartialEq)]
pub struct HabitMetrics {
    /// Habit strength score in `[0.0, 1.0]`.
    pub strength: f64,
    /// Formatted strength percentage (e.g. `"88%"`).
    pub strength_percent: String,
    /// Current consecutive active day streak count.
    pub current_streak: u32,
    /// Formatted current streak badge (e.g. `"4d"`).
    pub current_streak_text: String,
    /// All-time best consecutive active day streak count.
    pub best_streak: u32,
    /// Formatted best streak badge (e.g. `"18d"`).
    pub best_streak_text: String,
}

/// Calculates strength and streak analytics for a habit given its entries and evaluation date.
#[must_use]
pub fn calculate_habit_metrics(
    habit: &Habit,
    entries: &[HabitEntry],
    as_of: NaiveDate,
) -> HabitMetrics {
    let strength = calculate_habit_strength(habit, entries, as_of);
    let (current_streak, best_streak) = calculate_streaks(habit, entries, as_of);
    HabitMetrics {
        strength,
        strength_percent: format_strength_percentage(strength),
        current_streak,
        current_streak_text: format_streak_badge(current_streak),
        best_streak,
        best_streak_text: format_streak_badge(best_streak),
    }
}

/// Computes the toggled status and recorded value when a user clicks or logs a habit date.
#[must_use]
pub fn toggle_entry_state(
    habit: &Habit,
    current_entry: Option<&HabitEntry>,
    new_value: Option<f64>,
) -> (EntryStatus, f64) {
    new_value.map_or_else(
        || match &habit.habit_type {
            HabitType::Binary => current_entry.map_or((EntryStatus::Completed, 1.0), |entry| {
                if entry.status == EntryStatus::Completed {
                    (EntryStatus::Missed, 0.0)
                } else {
                    (EntryStatus::Completed, 1.0)
                }
            }),
            HabitType::Measurable { target, .. } => {
                current_entry.map_or((EntryStatus::Completed, *target), |entry| {
                    if entry.status == EntryStatus::Completed {
                        (EntryStatus::Missed, 0.0)
                    } else {
                        (EntryStatus::Completed, *target)
                    }
                })
            }
        },
        |val| match &habit.habit_type {
            HabitType::Binary => {
                if val >= 1.0 {
                    (EntryStatus::Completed, 1.0)
                } else {
                    (EntryStatus::Missed, 0.0)
                }
            }
            HabitType::Measurable { target, .. } => {
                if val >= *target {
                    (EntryStatus::Completed, val)
                } else {
                    (EntryStatus::Missed, val)
                }
            }
        },
    )
}

#[cfg(feature = "ui")]
#[derive(Clone)]
struct DrawCaches {
    curve_data: Rc<RefCell<Vec<f64>>>,
    curve_color: Rc<RefCell<Rgba>>,
    month_nav: Rc<RefCell<MonthNavigation>>,
    heatmap_entries: Rc<RefCell<HashMap<NaiveDate, EntryStatus>>>,
    heatmap_color: Rc<RefCell<Rgba>>,
}

#[cfg(feature = "ui")]
struct WindowInner {
    split_view: libadwaita::NavigationSplitView,
    db: Rc<RefCell<Database>>,
    selected_habit_id: Option<i64>,
    month_nav: MonthNavigation,
    habits_list_box: gtk4::Box,
    detail_stack: gtk4::Stack,
    title_label: gtk4::Label,
    subtitle_label: gtk4::Label,
    color_badge: gtk4::Label,
    edit_btn: gtk4::Button,
    strength_card_value: gtk4::Label,
    current_streak_card_value: gtk4::Label,
    best_streak_card_value: gtk4::Label,
    curve_drawing_area: gtk4::DrawingArea,
    month_title_label: gtk4::Label,
    heatmap_drawing_area: gtk4::DrawingArea,
}

#[cfg(feature = "ui")]
struct SidebarWidgets {
    page: libadwaita::NavigationPage,
    habits_list_box: gtk4::Box,
    add_btn: gtk4::Button,
    menu_btn: gtk4::MenuButton,
}

#[cfg(feature = "ui")]
#[derive(Clone)]
struct DetailsWidgets {
    page: libadwaita::NavigationPage,
    detail_stack: gtk4::Stack,
    title_label: gtk4::Label,
    subtitle_label: gtk4::Label,
    color_badge: gtk4::Label,
    edit_btn: gtk4::Button,
    strength_card_value: gtk4::Label,
    current_streak_card_value: gtk4::Label,
    best_streak_card_value: gtk4::Label,
    curve_drawing_area: gtk4::DrawingArea,
    month_title_label: gtk4::Label,
    prev_month_btn: gtk4::Button,
    next_month_btn: gtk4::Button,
    heatmap_drawing_area: gtk4::DrawingArea,
}

#[cfg(feature = "ui")]
fn create_metric_card(title: &str, value_label: &gtk4::Label) -> gtk4::Box {
    let card = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
    card.add_css_class("card");
    card.set_hexpand(true);
    card.set_margin_start(4);
    card.set_margin_end(4);
    card.set_margin_top(4);
    card.set_margin_bottom(4);

    let title_lbl = gtk4::Label::new(Some(title));
    title_lbl.add_css_class("dim-label");
    title_lbl.add_css_class("caption");
    title_lbl.set_halign(gtk4::Align::Center);
    card.append(&title_lbl);

    value_label.add_css_class("title-1");
    value_label.set_halign(gtk4::Align::Center);
    card.append(value_label);

    card
}

#[cfg(feature = "ui")]
fn handle_toggle(
    inner_rc: &Rc<RefCell<WindowInner>>,
    caches: &DrawCaches,
    habit_id: i64,
    date: NaiveDate,
    val_opt: Option<f64>,
) {
    let (habit, current_entry) = {
        let inner = inner_rc.borrow();
        let db = inner.db.borrow();
        let habit = db.get_habit(habit_id).ok().flatten();
        let entry = db.get_entry(habit_id, date).ok().flatten();
        (habit, entry)
    };

    if let Some(habit) = habit {
        let (status, val) = toggle_entry_state(&habit, current_entry.as_ref(), val_opt);
        {
            let inner = inner_rc.borrow();
            let _ = inner.db.borrow().log_entry(habit_id, date, status, val);
        }
        refresh(inner_rc, caches);
    }
}

#[cfg(feature = "ui")]
fn handle_select(
    inner_rc: &Rc<RefCell<WindowInner>>,
    caches: &DrawCaches,
    habit_id: i64,
) {
    {
        let mut inner = inner_rc.borrow_mut();
        inner.selected_habit_id = Some(habit_id);
        inner.split_view.set_show_content(true);
    }
    refresh(inner_rc, caches);
}

#[cfg(feature = "ui")]
fn refresh(inner_rc: &Rc<RefCell<WindowInner>>, caches: &DrawCaches) {
    let today = chrono::Local::now().date_naive();
    let mut inner = inner_rc.borrow_mut();

    let habits = inner.db.borrow().get_habits().unwrap_or_default();

    // Reconcile selected habit
    if inner.selected_habit_id.is_none() && !habits.is_empty() {
        inner.selected_habit_id = habits.first().and_then(|h| h.id);
    } else if let Some(sel_id) = inner.selected_habit_id {
        if !habits.iter().any(|h| h.id == Some(sel_id)) {
            inner.selected_habit_id = habits.first().and_then(|h| h.id);
        }
    }

    // 1. Rebuild left pane list
    while let Some(child) = inner.habits_list_box.first_child() {
        inner.habits_list_box.remove(&child);
    }

    for habit in &habits {
        let habit_id = habit.id.unwrap_or(0);
        let start_bound = NaiveDate::from_ymd_opt(2000, 1, 1).unwrap_or(today);
        let entries = inner.db.borrow().get_entries(habit_id, start_bound, today).unwrap_or_default();
        let strength = calculate_habit_strength(habit, &entries, today);

        let inner_toggle = Rc::clone(inner_rc);
        let caches_toggle = caches.clone();
        let on_toggle = move |hid: i64, d: NaiveDate, v: Option<f64>| {
            handle_toggle(&inner_toggle, &caches_toggle, hid, d, v);
        };

        let inner_sel = Rc::clone(inner_rc);
        let caches_sel = caches.clone();
        let on_select = move |hid: i64| {
            handle_select(&inner_sel, &caches_sel, hid);
        };

        let row_widget = HabitRowWidget::new(habit, strength, &entries, today, on_toggle, on_select);
        if habit.id == inner.selected_habit_id {
            row_widget.widget().add_css_class("selected");
        }
        inner.habits_list_box.append(row_widget.widget());
    }

    // 2. Update right pane details
    if let Some(sel_id) = inner.selected_habit_id {
        if let Some(selected_habit) = habits.iter().find(|h| h.id == Some(sel_id)) {
            inner.edit_btn.set_visible(true);
            inner.detail_stack.set_visible_child_name("details");
            inner.title_label.set_text(&selected_habit.name);

            let sched_text = format_schedule_summary(selected_habit.scheduled_days);
            let target_text = format_target_summary(selected_habit);
            inner.subtitle_label.set_text(&format!("Schedule: {sched_text} • Target: {target_text}"));
            inner.color_badge.set_text(&selected_habit.accent_color);

            let start_bound = NaiveDate::from_ymd_opt(2000, 1, 1).unwrap_or(today);
            let all_entries = inner.db.borrow().get_entries(sel_id, start_bound, today).unwrap_or_default();
            let metrics = calculate_habit_metrics(selected_habit, &all_entries, today);

            inner.strength_card_value.set_text(&metrics.strength_percent);
            inner.current_streak_card_value.set_text(&metrics.current_streak_text);
            inner.best_streak_card_value.set_text(&metrics.best_streak_text);

            let curve_pts = calculate_strength_history(selected_habit, &all_entries, 30, today);
            let rgba = Rgba::from_hex(&selected_habit.accent_color)
                .unwrap_or(Rgba::new(0.21, 0.52, 0.89, 1.0));

            *caches.curve_data.borrow_mut() = curve_pts;
            *caches.curve_color.borrow_mut() = rgba;
            inner.curve_drawing_area.queue_draw();

            // Monthly heatmap data
            inner.month_title_label.set_text(&inner.month_nav.format_title());
            let days = days_in_month(inner.month_nav.year, inner.month_nav.month);
            let m_from = NaiveDate::from_ymd_opt(inner.month_nav.year, inner.month_nav.month, 1)
                .unwrap_or(today);
            let m_to = NaiveDate::from_ymd_opt(inner.month_nav.year, inner.month_nav.month, days)
                .unwrap_or(today);
            let m_entries = inner.db.borrow().get_entries(sel_id, m_from, m_to).unwrap_or_default();

            let mut map = HashMap::with_capacity(m_entries.len() + usize::try_from(days).unwrap_or(0));
            for e in m_entries {
                map.insert(e.date, e.status);
            }
            for d in 1..=days {
                if let Some(date) = NaiveDate::from_ymd_opt(inner.month_nav.year, inner.month_nav.month, d) {
                    if !map.contains_key(&date) && date <= today && !selected_habit.is_scheduled_on(date.weekday()) {
                        map.insert(date, EntryStatus::Skipped);
                    }
                }
            }

            *caches.heatmap_entries.borrow_mut() = map;
            *caches.heatmap_color.borrow_mut() = rgba;
            *caches.month_nav.borrow_mut() = inner.month_nav;
            inner.heatmap_drawing_area.queue_draw();
            return;
        }
    }

    inner.edit_btn.set_visible(false);
    inner.detail_stack.set_visible_child_name("empty");
}

#[cfg(feature = "ui")]
fn default_database_path() -> std::path::PathBuf {
    let data_dir = std::env::var_os("XDG_DATA_HOME").map_or_else(
        || {
            let home = std::env::var_os("HOME").unwrap_or_default();
            std::path::PathBuf::from(home)
                .join(".local")
                .join("share")
                .join("habit")
        },
        |val| std::path::PathBuf::from(val).join("habit"),
    );
    let _ = std::fs::create_dir_all(&data_dir);
    data_dir.join("habits.db")
}

#[cfg(feature = "ui")]
fn build_sidebar() -> SidebarWidgets {
    let sidebar_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);

    let sidebar_header = libadwaita::HeaderBar::new();
    let sidebar_title = libadwaita::WindowTitle::new("Habits", "");
    sidebar_header.set_title_widget(Some(&sidebar_title));

    let add_btn = gtk4::Button::from_icon_name("list-add-symbolic");
    add_btn.set_tooltip_text(Some("New Habit"));
    add_btn.add_css_class("flat");
    sidebar_header.pack_end(&add_btn);

    let menu_btn = gtk4::MenuButton::new();
    menu_btn.set_icon_name("open-menu-symbolic");
    menu_btn.add_css_class("flat");
    sidebar_header.pack_end(&menu_btn);

    sidebar_box.append(&sidebar_header);

    // Quick-check column headers
    let quick_check_header = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
    quick_check_header.set_margin_start(12);
    quick_check_header.set_margin_end(12);
    quick_check_header.set_margin_top(8);
    quick_check_header.set_margin_bottom(8);

    let habit_progress_label = gtk4::Label::new(Some("HABIT & PROGRESS"));
    habit_progress_label.set_halign(gtk4::Align::Start);
    habit_progress_label.set_hexpand(true);
    habit_progress_label.add_css_class("dim-label");
    habit_progress_label.add_css_class("caption");
    quick_check_header.append(&habit_progress_label);

    let days_header_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    days_header_box.set_halign(gtk4::Align::End);
    for name in &["Today", "Yest", "-2d", "-3d", "-4d", "-5d", "-6d"] {
        let lbl = gtk4::Label::new(Some(name));
        lbl.add_css_class("dim-label");
        lbl.add_css_class("caption");
        lbl.set_width_chars(3);
        days_header_box.append(&lbl);
    }
    quick_check_header.append(&days_header_box);
    sidebar_box.append(&quick_check_header);

    let sep1 = gtk4::Separator::new(gtk4::Orientation::Horizontal);
    sidebar_box.append(&sep1);

    let habits_list_box = gtk4::Box::new(gtk4::Orientation::Vertical, 6);
    habits_list_box.set_margin_start(12);
    habits_list_box.set_margin_end(12);
    habits_list_box.set_margin_top(8);
    habits_list_box.set_margin_bottom(8);

    let sidebar_scrolled = gtk4::ScrolledWindow::new();
    sidebar_scrolled.set_vexpand(true);
    sidebar_scrolled.set_child(Some(&habits_list_box));
    sidebar_box.append(&sidebar_scrolled);

    let page = libadwaita::NavigationPage::new(&sidebar_box, "Habits");
    page.set_tag(Some("sidebar"));

    SidebarWidgets {
        page,
        habits_list_box,
        add_btn,
        menu_btn,
    }
}

#[cfg(feature = "ui")]
fn build_metrics_section() -> (gtk4::Box, gtk4::Label, gtk4::Label, gtk4::Label) {
    let metrics_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 10);
    metrics_box.set_homogeneous(true);

    let strength_card_value = gtk4::Label::new(Some("0%"));
    let current_streak_card_value = gtk4::Label::new(Some("0d"));
    let best_streak_card_value = gtk4::Label::new(Some("0d"));

    let card_strength = create_metric_card("Strength", &strength_card_value);
    let card_current = create_metric_card("Current Streak", &current_streak_card_value);
    let card_best = create_metric_card("Best Streak", &best_streak_card_value);

    metrics_box.append(&card_strength);
    metrics_box.append(&card_current);
    metrics_box.append(&card_best);

    (metrics_box, strength_card_value, current_streak_card_value, best_streak_card_value)
}

#[cfg(feature = "ui")]
fn build_heatmap_section() -> (gtk4::Box, gtk4::Label, gtk4::Button, gtk4::Button, gtk4::DrawingArea) {
    let heatmap_nav_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    heatmap_nav_box.set_margin_top(4);
    heatmap_nav_box.set_margin_bottom(4);

    let heatmap_title = gtk4::Label::new(Some("History Heatmap"));
    heatmap_title.add_css_class("heading");
    heatmap_title.set_halign(gtk4::Align::Start);
    heatmap_title.set_hexpand(true);
    heatmap_nav_box.append(&heatmap_title);

    let month_title_label = gtk4::Label::new(None);
    month_title_label.add_css_class("heading");
    month_title_label.set_halign(gtk4::Align::Center);
    heatmap_nav_box.append(&month_title_label);

    let nav_buttons = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    nav_buttons.add_css_class("linked");

    let prev_month_btn = gtk4::Button::from_icon_name("go-previous-symbolic");
    prev_month_btn.set_tooltip_text(Some("Previous Month"));
    nav_buttons.append(&prev_month_btn);

    let next_month_btn = gtk4::Button::from_icon_name("go-next-symbolic");
    next_month_btn.set_tooltip_text(Some("Next Month"));
    nav_buttons.append(&next_month_btn);

    heatmap_nav_box.append(&nav_buttons);

    let heatmap_drawing_area = gtk4::DrawingArea::new();
    heatmap_drawing_area.set_content_height(160);
    heatmap_drawing_area.set_hexpand(true);

    (heatmap_nav_box, month_title_label, prev_month_btn, next_month_btn, heatmap_drawing_area)
}

#[cfg(feature = "ui")]
fn build_details() -> DetailsWidgets {
    let content_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);

    let content_header = libadwaita::HeaderBar::new();
    let content_title = libadwaita::WindowTitle::new("Habit Details", "");
    content_header.set_title_widget(Some(&content_title));

    let edit_btn = gtk4::Button::from_icon_name("document-edit-symbolic");
    edit_btn.set_tooltip_text(Some("Edit Habit"));
    edit_btn.add_css_class("flat");
    content_header.pack_end(&edit_btn);

    content_box.append(&content_header);

    let detail_stack = gtk4::Stack::new();
    detail_stack.set_vexpand(true);

    let empty_page = libadwaita::StatusPage::builder()
        .icon_name("view-list-bullet-symbolic")
        .title("No Habit Selected")
        .description("Select a habit from the list or create a new one to view analytics.")
        .build();
    detail_stack.add_named(&empty_page, Some("empty"));

    let details_vbox = gtk4::Box::new(gtk4::Orientation::Vertical, 14);
    details_vbox.set_margin_start(18);
    details_vbox.set_margin_end(18);
    details_vbox.set_margin_top(14);
    details_vbox.set_margin_bottom(18);

    let info_box = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
    let title_label = gtk4::Label::new(None);
    title_label.add_css_class("title-1");
    title_label.set_halign(gtk4::Align::Start);
    info_box.append(&title_label);

    let sub_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    sub_box.set_halign(gtk4::Align::Start);

    let subtitle_label = gtk4::Label::new(None);
    subtitle_label.add_css_class("dim-label");
    sub_box.append(&subtitle_label);

    let color_badge = gtk4::Label::new(None);
    color_badge.add_css_class("caption");
    color_badge.add_css_class("badge");
    sub_box.append(&color_badge);
    info_box.append(&sub_box);
    details_vbox.append(&info_box);

    let sep2 = gtk4::Separator::new(gtk4::Orientation::Horizontal);
    details_vbox.append(&sep2);

    let (metrics_box, strength_card_value, current_streak_card_value, best_streak_card_value) =
        build_metrics_section();
    details_vbox.append(&metrics_box);

    let sep3 = gtk4::Separator::new(gtk4::Orientation::Horizontal);
    details_vbox.append(&sep3);

    let curve_header = gtk4::Label::new(Some("Habit Strength Curve (Last 30 Days)"));
    curve_header.add_css_class("heading");
    curve_header.set_halign(gtk4::Align::Start);
    details_vbox.append(&curve_header);

    let curve_drawing_area = gtk4::DrawingArea::new();
    curve_drawing_area.set_content_height(140);
    curve_drawing_area.set_hexpand(true);
    details_vbox.append(&curve_drawing_area);

    let sep4 = gtk4::Separator::new(gtk4::Orientation::Horizontal);
    details_vbox.append(&sep4);

    let (heatmap_nav_box, month_title_label, prev_month_btn, next_month_btn, heatmap_drawing_area) =
        build_heatmap_section();
    details_vbox.append(&heatmap_nav_box);
    details_vbox.append(&heatmap_drawing_area);

    let details_scrolled = gtk4::ScrolledWindow::new();
    details_scrolled.set_vexpand(true);
    details_scrolled.set_child(Some(&details_vbox));
    detail_stack.add_named(&details_scrolled, Some("details"));

    content_box.append(&detail_stack);

    let page = libadwaita::NavigationPage::new(&content_box, "Details");
    page.set_tag(Some("details"));

    DetailsWidgets {
        page,
        detail_stack,
        title_label,
        subtitle_label,
        color_badge,
        edit_btn,
        strength_card_value,
        current_streak_card_value,
        best_streak_card_value,
        curve_drawing_area,
        month_title_label,
        prev_month_btn,
        next_month_btn,
        heatmap_drawing_area,
    }
}

#[cfg(feature = "ui")]
fn connect_controllers(
    inner_rc: &Rc<RefCell<WindowInner>>,
    caches: &DrawCaches,
    root_window: &gtk4::Window,
    details: &DetailsWidgets,
    add_btn: &gtk4::Button,
) {
    let inner_prev = Rc::clone(inner_rc);
    let caches_prev = caches.clone();
    details.prev_month_btn.connect_clicked(move |_| {
        {
            let mut inner = inner_prev.borrow_mut();
            inner.month_nav = inner.month_nav.prev();
        }
        refresh(&inner_prev, &caches_prev);
    });

    let inner_next = Rc::clone(inner_rc);
    let caches_next = caches.clone();
    details.next_month_btn.connect_clicked(move |_| {
        {
            let mut inner = inner_next.borrow_mut();
            inner.month_nav = inner.month_nav.next();
        }
        refresh(&inner_next, &caches_next);
    });

    let heatmap_gesture = gtk4::GestureClick::new();
    let inner_gesture = Rc::clone(inner_rc);
    let caches_gesture = caches.clone();
    let area_weak = details.heatmap_drawing_area.downgrade();
    heatmap_gesture.connect_pressed(move |_, _, x, y| {
        let Some(area) = area_weak.upgrade() else {
            return;
        };
        let width = f64::from(area.width());
        let height = f64::from(area.height());
        let (year, month, selected_id) = {
            let inner = inner_gesture.borrow();
            (inner.month_nav.year, inner.month_nav.month, inner.selected_habit_id)
        };
        if let Some(habit_id) = selected_id {
            if let Some(clicked_date) = heatmap_hit_test(x, y, width, height, year, month) {
                let today = chrono::Local::now().date_naive();
                if clicked_date <= today {
                    handle_toggle(&inner_gesture, &caches_gesture, habit_id, clicked_date, None);
                }
            }
        }
    });
    details.heatmap_drawing_area.add_controller(heatmap_gesture);

    let inner_add = Rc::clone(inner_rc);
    let caches_add = caches.clone();
    let win_add = root_window.clone();
    add_btn.connect_clicked(move |_| {
        let inner_saved = Rc::clone(&inner_add);
        let caches_saved = caches_add.clone();
        show_habit_dialog(&win_add, None, move |new_habit| {
            let db = {
                let inner = inner_saved.borrow();
                Rc::clone(&inner.db)
            };
            let create_res = db.borrow().create_habit(&new_habit);
            if let Ok(id) = create_res {
                let mut inner = inner_saved.borrow_mut();
                inner.selected_habit_id = Some(id);
                inner.split_view.set_show_content(true);
            }
            refresh(&inner_saved, &caches_saved);
        });
    });

    let inner_edit = Rc::clone(inner_rc);
    let caches_edit = caches.clone();
    let win_edit = root_window.clone();
    details.edit_btn.connect_clicked(move |_| {
        let (selected_id, db) = {
            let inner = inner_edit.borrow();
            (inner.selected_habit_id, Rc::clone(&inner.db))
        };
        let habit_opt = selected_id.and_then(|id| db.borrow().get_habit(id).ok().flatten());
        if let Some(habit) = habit_opt {
            let inner_saved = Rc::clone(&inner_edit);
            let caches_saved = caches_edit.clone();
            show_habit_dialog(&win_edit, Some(&habit), move |updated_habit| {
                let id_opt = updated_habit.id;
                let db = {
                    let inner = inner_saved.borrow();
                    Rc::clone(&inner.db)
                };
                if let Err(e) = db.borrow().update_habit(&updated_habit) {
                    eprintln!("Failed to update habit: {e}");
                }
                {
                    let mut inner = inner_saved.borrow_mut();
                    inner.selected_habit_id = id_opt;
                }
                refresh(&inner_saved, &caches_saved);
            });
        }
    });
}

#[cfg(feature = "ui")]
fn setup_primary_menu(
    window: &libadwaita::ApplicationWindow,
    root_window: &gtk4::Window,
    inner_rc: &Rc<RefCell<WindowInner>>,
    caches: &DrawCaches,
    menu_btn: &gtk4::MenuButton,
    toast_overlay: &libadwaita::ToastOverlay,
) {
    let menu_model = gtk4::gio::Menu::new();
    menu_model.append(Some("Export Backup..."), Some("win.export-backup"));
    menu_model.append(Some("Import Backup..."), Some("win.import-backup"));
    menu_model.append(Some("About Habit"), Some("win.about"));
    menu_btn.set_menu_model(Some(&menu_model));

    // Export action
    let action_export = gtk4::gio::SimpleAction::new("export-backup", None);
    let inner_exp = Rc::clone(inner_rc);
    let root_exp = root_window.clone();
    let toast_exp = toast_overlay.clone();
    action_export.connect_activate(move |_, _| {
        let db = {
            let inner = inner_exp.borrow();
            Rc::clone(&inner.db)
        };
        let toast = toast_exp.clone();
        trigger_export_dialog(&root_exp, db, move |res| match res {
            Ok(path) => {
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("backup.db");
                toast.add_toast(libadwaita::Toast::new(&format!("Backup exported to {name}")));
            }
            Err(err) => {
                if !err.contains("cancelled") && !err.contains("Dismissed") {
                    toast.add_toast(libadwaita::Toast::new(&format!("Export failed: {err}")));
                }
            }
        });
    });
    window.add_action(&action_export);

    // Import action
    let action_import = gtk4::gio::SimpleAction::new("import-backup", None);
    let inner_imp = Rc::clone(inner_rc);
    let caches_imp = caches.clone();
    let root_imp = root_window.clone();
    let toast_imp = toast_overlay.clone();
    action_import.connect_activate(move |_, _| {
        let db = {
            let inner = inner_imp.borrow();
            Rc::clone(&inner.db)
        };
        let toast = toast_imp.clone();
        let inner_done = Rc::clone(&inner_imp);
        let caches_done = caches_imp.clone();
        trigger_import_dialog(&root_imp, db, move |res| match res {
            Ok(_path) => {
                toast.add_toast(libadwaita::Toast::new("Database restored successfully"));
                {
                    let mut inner = inner_done.borrow_mut();
                    inner.selected_habit_id = None;
                }
                refresh(&inner_done, &caches_done);
            }
            Err(err) => {
                if !err.contains("cancelled") && !err.contains("Dismissed") {
                    toast.add_toast(libadwaita::Toast::new(&format!("Import failed: {err}")));
                }
            }
        });
    });
    window.add_action(&action_import);

    // About action
    let action_about = gtk4::gio::SimpleAction::new("about", None);
    let root_about = root_window.clone();
    action_about.connect_activate(move |_, _| {
        let about = libadwaita::AboutDialog::builder()
            .application_name("Habit")
            .application_icon(crate::APP_ID)
            .developer_name("Habit Contributors")
            .version("0.1.0")
            .license_type(gtk4::License::Gpl30)
            .comments("A native, local-only Linux habit tracker app for GNOME")
            .website("https://github.com/habit/habit")
            .issue_url("https://github.com/habit/habit/issues")
            .build();
        about.present(Some(&root_about));
    });
    window.add_action(&action_about);
}

#[cfg(feature = "ui")]
fn connect_drawing_areas(details: &DetailsWidgets, caches: &DrawCaches) {
    let curve_data_draw = Rc::clone(&caches.curve_data);
    let curve_color_draw = Rc::clone(&caches.curve_color);
    details.curve_drawing_area.set_draw_func(move |_area, cr, width, height| {
        let data = curve_data_draw.borrow();
        let color = curve_color_draw.borrow();
        draw_strength_curve(
            cr,
            f64::from(width),
            f64::from(height),
            &data,
            &color,
        );
    });

    let month_nav_draw = Rc::clone(&caches.month_nav);
    let heatmap_entries_draw = Rc::clone(&caches.heatmap_entries);
    let heatmap_color_draw = Rc::clone(&caches.heatmap_color);
    details.heatmap_drawing_area.set_draw_func(move |_area, cr, width, height| {
        let nav = month_nav_draw.borrow();
        let entries = heatmap_entries_draw.borrow();
        let color = heatmap_color_draw.borrow();
        draw_monthly_heatmap(
            cr,
            f64::from(width),
            f64::from(height),
            nav.year,
            nav.month,
            &entries,
            &color,
        );
    });
}

/// Main application window providing a 50/50 responsive split view between habits and analytics.
#[cfg(feature = "ui")]
#[derive(Debug, Clone)]
pub struct HabitWindow {
    window: libadwaita::ApplicationWindow,
    split_view: libadwaita::NavigationSplitView,
    toast_overlay: libadwaita::ToastOverlay,
}

#[cfg(feature = "ui")]
impl HabitWindow {
    /// Opens the application window using the default on-disk SQLite database path.
    ///
    /// # Panics
    /// Panics if opening an in-memory database fallback fails.
    #[must_use]
    pub fn open_default(app: &libadwaita::Application) -> Self {
        let db_path = default_database_path();
        let db = Database::open(&db_path).unwrap_or_else(|_| {
            Database::open_in_memory().expect("In-memory database fallback should always succeed")
        });
        Self::new(app, db)
    }

    /// Creates and initializes the `HabitWindow` with the provided database instance.
    #[must_use]
    pub fn new(app: &libadwaita::Application, db: Database) -> Self {
        let window = libadwaita::ApplicationWindow::builder()
            .application(app)
            .title("Habits")
            .default_width(960)
            .default_height(640)
            .build();

        let split_view = libadwaita::NavigationSplitView::new();
        split_view.set_sidebar_width_fraction(0.5);
        split_view.set_min_sidebar_width(360.0);
        split_view.set_max_sidebar_width(1200.0);

        let sidebar = build_sidebar();
        split_view.set_sidebar(Some(&sidebar.page));

        let details = build_details();
        let details_clone = details.clone();
        split_view.set_content(Some(&details.page));

        // Setup Responsive Breakpoint (< 760px collapses to single pane)
        let breakpoint = libadwaita::Breakpoint::new(libadwaita::BreakpointCondition::new_length(
            libadwaita::BreakpointConditionLengthType::MaxWidth,
            760.0,
            libadwaita::LengthUnit::Px,
        ));
        breakpoint.add_setter(&split_view, "collapsed", Some(&true.into()));
        window.add_breakpoint(breakpoint);

        let toast_overlay = libadwaita::ToastOverlay::new();
        toast_overlay.set_child(Some(&split_view));
        window.set_content(Some(&toast_overlay));

        let now = chrono::Local::now();
        let initial_nav = MonthNavigation::new(now.year(), now.month());

        let caches = DrawCaches {
            curve_data: Rc::new(RefCell::new(Vec::new())),
            curve_color: Rc::new(RefCell::new(Rgba::new(0.21, 0.52, 0.89, 1.0))),
            month_nav: Rc::new(RefCell::new(initial_nav)),
            heatmap_entries: Rc::new(RefCell::new(HashMap::new())),
            heatmap_color: Rc::new(RefCell::new(Rgba::new(0.21, 0.52, 0.89, 1.0))),
        };

        connect_drawing_areas(&details, &caches);

        let db_rc = Rc::new(RefCell::new(db));
        let inner = WindowInner {
            split_view: split_view.clone(),
            db: Rc::clone(&db_rc),
            selected_habit_id: None,
            month_nav: initial_nav,
            habits_list_box: sidebar.habits_list_box,
            detail_stack: details.detail_stack,
            title_label: details.title_label,
            subtitle_label: details.subtitle_label,
            color_badge: details.color_badge,
            edit_btn: details.edit_btn.clone(),
            strength_card_value: details.strength_card_value,
            current_streak_card_value: details.current_streak_card_value,
            best_streak_card_value: details.best_streak_card_value,
            curve_drawing_area: details.curve_drawing_area,
            month_title_label: details.month_title_label,
            heatmap_drawing_area: details.heatmap_drawing_area,
        };

        let inner_rc = Rc::new(RefCell::new(inner));
        let root_window: gtk4::Window = window.clone().upcast();

        connect_controllers(
            &inner_rc,
            &caches,
            &root_window,
            &details_clone,
            &sidebar.add_btn,
        );

        setup_primary_menu(
            &window,
            &root_window,
            &inner_rc,
            &caches,
            &sidebar.menu_btn,
            &toast_overlay,
        );

        // Initial populate and render
        refresh(&inner_rc, &caches);

        Self {
            window,
            split_view,
            toast_overlay,
        }
    }

    /// Returns a reference to the wrapped `libadwaita::ApplicationWindow`.
    #[must_use]
    pub const fn window(&self) -> &libadwaita::ApplicationWindow {
        &self.window
    }

    /// Returns a reference to the `libadwaita::NavigationSplitView`.
    #[must_use]
    pub const fn split_view(&self) -> &libadwaita::NavigationSplitView {
        &self.split_view
    }

    /// Returns a reference to the `libadwaita::ToastOverlay`.
    #[must_use]
    pub const fn toast_overlay(&self) -> &libadwaita::ToastOverlay {
        &self.toast_overlay
    }

    /// Presents the application window to the user.
    pub fn present(&self) {
        self.window.present();
    }
}

#[cfg(feature = "ui")]
impl std::ops::Deref for HabitWindow {
    type Target = libadwaita::ApplicationWindow;

    fn deref(&self) -> &Self::Target {
        &self.window
    }
}

#[cfg(feature = "ui")]
impl AsRef<gtk4::Widget> for HabitWindow {
    fn as_ref(&self) -> &gtk4::Widget {
        self.window.upcast_ref()
    }
}

/// Application entry point: initializes libadwaita and launches `HabitWindow`.
#[cfg(feature = "ui")]
pub fn run_app() {
    let app = libadwaita::Application::builder()
        .application_id(crate::APP_ID)
        .build();

    app.connect_activate(|app| {
        let window = HabitWindow::open_default(app);
        window.present();
    });

    app.run();
}
