use chrono::NaiveDate;

#[cfg(feature = "ui")]
use std::cell::RefCell;
#[cfg(feature = "ui")]
use std::collections::HashMap;
#[cfg(feature = "ui")]
use std::fmt::Write as _;
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
use crate::engine::scoring::{calculate_habit_score, calculate_streaks};
use crate::ui::habit_row::{format_measurable_value, next_toggle_status};

#[cfg(feature = "ui")]
use crate::engine::db::Database;
#[cfg(feature = "ui")]
use crate::engine::scoring::calculate_strength_history;
#[cfg(feature = "ui")]
use crate::ui::charts::{
    draw_loop_heatmap, draw_progress_ring, draw_strength_curve, loop_heatmap_hit_test, Rgba,
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

/// Formats habit score in `[0.0, 1.0]` into a rounded percentage string (e.g. `"88%"`).
#[must_use]
pub fn format_score_percentage(score: f64) -> String {
    let clamped = score.clamp(0.0, 1.0);
    let pct = (clamped * 100.0).round();
    format!("{pct:.0}%")
}

/// Backwards-compatibility alias for `format_score_percentage`.
#[must_use]
pub fn format_strength_percentage(strength: f64) -> String {
    format_score_percentage(strength)
}

/// Formats a streak count into a badge string (e.g. `"4d"`).
#[must_use]
pub fn format_streak_badge(streak: u32) -> String {
    format!("{streak}d")
}

/// Formats a score percentage difference (e.g. `"+5%"`, `"-2%"`, `"+0%"`).
#[must_use]
pub fn format_percentage_diff(diff: f64) -> String {
    if !diff.is_finite() {
        return "+0%".to_string();
    }
    let pct = (diff.abs() * 100.0).round();
    if diff >= 0.0 {
        format!("+{pct:.0}%")
    } else {
        format!("-{pct:.0}%")
    }
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
    /// Habit score in `[0.0, 1.0]`.
    pub score: f64,
    /// Formatted score percentage (e.g. `"88%"`).
    pub score_percent: String,
    /// Habit strength score in `[0.0, 1.0]` (alias for score).
    pub strength: f64,
    /// Formatted strength percentage (alias for `score_percent`).
    pub strength_percent: String,
    /// Current consecutive active day streak count.
    pub current_streak: u32,
    /// Formatted current streak badge (e.g. `"4d"`).
    pub current_streak_text: String,
    /// All-time best consecutive active day streak count.
    pub best_streak: u32,
    /// Formatted best streak badge (e.g. `"18d"`).
    pub best_streak_text: String,
    /// Score difference compared to 30 days ago.
    pub month_diff: f64,
    /// Formatted 30-day difference (e.g. `"+5%"`).
    pub month_diff_text: String,
    /// Score difference compared to 365 days ago.
    pub year_diff: f64,
    /// Formatted 365-day difference (e.g. `"+12%"`).
    pub year_diff_text: String,
    /// All-time completed entries count up to evaluation date.
    pub total_count: usize,
    /// Formatted completed count string (e.g. `"42"`).
    pub total_count_text: String,
}

/// Calculates score, overview differences, and streak analytics for a habit given its entries and evaluation date.
#[must_use]
pub fn calculate_habit_metrics(
    habit: &Habit,
    entries: &[HabitEntry],
    as_of: NaiveDate,
) -> HabitMetrics {
    let score = calculate_habit_score(habit, entries, as_of);
    let pct = format_score_percentage(score);
    let (current_streak, best_streak) = calculate_streaks(habit, entries, as_of);

    let date_month = as_of
        .checked_sub_signed(chrono::Duration::days(30))
        .unwrap_or(as_of);
    let score_month = calculate_habit_score(habit, entries, date_month);
    let month_diff = score - score_month;
    let month_diff_text = format_percentage_diff(month_diff);

    let date_year = as_of
        .checked_sub_signed(chrono::Duration::days(365))
        .unwrap_or(as_of);
    let score_year = calculate_habit_score(habit, entries, date_year);
    let year_diff = score - score_year;
    let year_diff_text = format_percentage_diff(year_diff);

    let total_count = entries
        .iter()
        .filter(|e| e.status == EntryStatus::Completed && e.date <= as_of)
        .count();
    let total_count_text = total_count.to_string();

    HabitMetrics {
        score,
        score_percent: pct.clone(),
        strength: score,
        strength_percent: pct,
        current_streak,
        current_streak_text: format_streak_badge(current_streak),
        best_streak,
        best_streak_text: format_streak_badge(best_streak),
        month_diff,
        month_diff_text,
        year_diff,
        year_diff_text,
        total_count,
        total_count_text,
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
            HabitType::Binary => {
                let next = next_toggle_status(current_entry.map(|e| e.status));
                match next {
                    EntryStatus::Completed => (EntryStatus::Completed, 1.0),
                    EntryStatus::Skipped | EntryStatus::Missed => (next, 0.0),
                }
            }
            HabitType::Measurable { target, .. } => {
                let next = next_toggle_status(current_entry.map(|e| e.status));
                match next {
                    EntryStatus::Completed => (EntryStatus::Completed, *target),
                    EntryStatus::Skipped | EntryStatus::Missed => (next, 0.0),
                }
            }
        },
        |val| {
            if (val - (-1.0)).abs() < 1e-6 {
                if current_entry.is_some_and(|e| e.status == EntryStatus::Skipped) {
                    (EntryStatus::Missed, 0.0)
                } else {
                    (EntryStatus::Skipped, 0.0)
                }
            } else {
                match &habit.habit_type {
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
    heatmap_entries: Rc<RefCell<HashMap<NaiveDate, EntryStatus>>>,
    heatmap_color: Rc<RefCell<Rgba>>,
    heatmap_week_offset: Rc<RefCell<usize>>,
    heatmap_today: Rc<RefCell<NaiveDate>>,
    overview_score: Rc<RefCell<f64>>,
    overview_color: Rc<RefCell<Rgba>>,
}

#[cfg(feature = "ui")]
struct WindowInner {
    nav_view: libadwaita::NavigationView,
    details_page: libadwaita::NavigationPage,
    db: Rc<RefCell<Database>>,
    selected_habit_id: Option<i64>,
    heatmap_week_offset: usize,
    habits_list_box: gtk4::Box,
    habit_score_label: gtk4::Label,
    header_day_labels: Vec<gtk4::Label>,
    habit_css_provider: gtk4::CssProvider,
    title_label: gtk4::Label,
    schedule_label: gtk4::Label,
    target_label: gtk4::Label,
    edit_btn: gtk4::Button,
    delete_btn: gtk4::Button,
    ring_drawing_area: gtk4::DrawingArea,
    score_card_value: gtk4::Label,
    month_diff_card_value: gtk4::Label,
    year_diff_card_value: gtk4::Label,
    total_count_card_value: gtk4::Label,
    curve_drawing_area: gtk4::DrawingArea,
    today_week_btn: gtk4::Button,
    next_week_btn: gtk4::Button,
    heatmap_drawing_area: gtk4::DrawingArea,
}

#[cfg(feature = "ui")]
struct SidebarWidgets {
    page: libadwaita::NavigationPage,
    habits_list_box: gtk4::Box,
    habit_score_label: gtk4::Label,
    header_day_labels: Vec<gtk4::Label>,
    add_btn: gtk4::Button,
    menu_btn: gtk4::MenuButton,
}

#[cfg(feature = "ui")]
#[derive(Clone)]
struct DetailsWidgets {
    page: libadwaita::NavigationPage,
    title_label: gtk4::Label,
    schedule_label: gtk4::Label,
    target_label: gtk4::Label,
    edit_btn: gtk4::Button,
    delete_btn: gtk4::Button,
    ring_drawing_area: gtk4::DrawingArea,
    score_card_value: gtk4::Label,
    month_diff_card_value: gtk4::Label,
    year_diff_card_value: gtk4::Label,
    total_count_card_value: gtk4::Label,
    curve_drawing_area: gtk4::DrawingArea,
    prev_week_btn: gtk4::Button,
    today_week_btn: gtk4::Button,
    next_week_btn: gtk4::Button,
    edit_mode_btn: gtk4::Button,
    heatmap_drawing_area: gtk4::DrawingArea,
}

#[cfg(feature = "ui")]
fn create_stat_column(value_label: &gtk4::Label, title: &str) -> gtk4::Box {
    let col = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
    col.set_hexpand(true);
    col.set_halign(gtk4::Align::Center);

    value_label.add_css_class("title-1");
    value_label.set_halign(gtk4::Align::Center);
    col.append(value_label);

    let title_lbl = gtk4::Label::new(Some(title));
    title_lbl.add_css_class("dim-label");
    title_lbl.add_css_class("caption");
    title_lbl.set_halign(gtk4::Align::Center);
    col.append(&title_lbl);

    col
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
fn handle_select(inner_rc: &Rc<RefCell<WindowInner>>, caches: &DrawCaches, habit_id: i64) {
    let (nav_view, details_page, should_push) = {
        let mut inner = inner_rc.borrow_mut();
        inner.selected_habit_id = Some(habit_id);
        let should_push = inner.nav_view.find_page("details").is_none();
        (
            inner.nav_view.clone(),
            inner.details_page.clone(),
            should_push,
        )
    };
    refresh(inner_rc, caches);
    if should_push {
        nav_view.push(&details_page);
    }
}

#[cfg(feature = "ui")]
#[allow(clippy::too_many_lines, clippy::cast_possible_wrap)]
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

    // 0. Update header date labels (5 days: today on the left)
    let dates = crate::ui::habit_row::calculate_5_day_dates(today);
    for (i, d) in dates.iter().enumerate() {
        if let Some(lbl) = inner.header_day_labels.get(i) {
            lbl.set_text(&d.format("%m/%d").to_string());
        }
    }

    // Build and apply global dynamic CSS for habit row styling (accent color on symbols, progress bars, bold titles)
    let mut full_css = String::from(
        ".habit-title {\n  font-weight: 700;\n}\n\
         .day-symbol {\n  font-size: 20px;\n  font-weight: bold;\n}\n\
         .loop-popup-box {\n  min-width: 200px;\n}\n\
         .loop-popup-btn {\n  min-width: 48px;\n  min-height: 42px;\n  font-size: 19px;\n  font-weight: bold;\n}\n\
         .loop-popup-btn.muted {\n  color: #888888;\n}\n",
    );
    for h in &habits {
        if let Some(hid) = h.id {
            full_css.push_str(&crate::ui::habit_row::generate_row_css(hid, &h.accent_color));
        }
    }
    if let Some(hid) = inner.selected_habit_id {
        if let Some(selected_habit) = habits.iter().find(|h| h.id == Some(hid)) {
            let color = if selected_habit.accent_color.starts_with('#') {
                selected_habit.accent_color.clone()
            } else {
                format!("#{}", selected_habit.accent_color)
            };
            let _ = write!(
                full_css,
                ".overview-accent {{\n  color: {color};\n  font-weight: 700;\n}}\n"
            );
        }
    }
    inner.habit_css_provider.load_from_data(&full_css);

    let left_group = gtk4::SizeGroup::new(gtk4::SizeGroupMode::Horizontal);
    left_group.add_widget(&inner.habit_score_label);

    let col_groups: Vec<gtk4::SizeGroup> = (0..5)
        .map(|i| {
            let sg = gtk4::SizeGroup::new(gtk4::SizeGroupMode::Horizontal);
            if let Some(lbl) = inner.header_day_labels.get(i) {
                sg.add_widget(lbl);
            }
            sg
        })
        .collect();

    // 1. Rebuild habits list
    while let Some(child) = inner.habits_list_box.first_child() {
        inner.habits_list_box.remove(&child);
    }

    for habit in &habits {
        let habit_id = habit.id.unwrap_or(0);
        let start_bound = NaiveDate::MIN;
        let entries = inner
            .db
            .borrow()
            .get_entries(habit_id, start_bound, today)
            .unwrap_or_default();
        let score = calculate_habit_score(habit, &entries, today);

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

        let row_widget = HabitRowWidget::new(habit, score, &entries, today, on_toggle, on_select);
        left_group.add_widget(&row_widget.left_box);
        for (i, btn) in row_widget.day_buttons.iter().enumerate() {
            if let Some(sg) = col_groups.get(i) {
                sg.add_widget(btn);
            }
        }
        if habit.id == inner.selected_habit_id {
            row_widget.widget().add_css_class("selected");
        }
        inner.habits_list_box.append(row_widget.widget());
    }

    // 2. Update details
    if let Some(sel_id) = inner.selected_habit_id {
        if let Some(selected_habit) = habits.iter().find(|h| h.id == Some(sel_id)) {
            inner.edit_btn.set_visible(true);
            inner.delete_btn.set_visible(true);
            inner.title_label.set_text(&selected_habit.name);

            let sched_text = format_schedule_summary(selected_habit.scheduled_days);
            let target_text = format_target_summary(selected_habit);
            inner
                .schedule_label
                .set_text(&format!("Schedule: {sched_text}"));
            inner
                .target_label
                .set_text(&format!("Target: {target_text}"));

            let start_bound = NaiveDate::MIN;
            let all_entries = inner
                .db
                .borrow()
                .get_entries(sel_id, start_bound, today)
                .unwrap_or_default();
            let metrics = calculate_habit_metrics(selected_habit, &all_entries, today);

            inner.score_card_value.set_text(&metrics.score_percent);
            inner.month_diff_card_value.set_text(&metrics.month_diff_text);
            inner.year_diff_card_value.set_text(&metrics.year_diff_text);
            inner.total_count_card_value.set_text(&metrics.total_count_text);

            if metrics.month_diff >= 0.0 {
                inner.month_diff_card_value.add_css_class("overview-accent");
                inner.month_diff_card_value.remove_css_class("dim-label");
            } else {
                inner.month_diff_card_value.remove_css_class("overview-accent");
                inner.month_diff_card_value.add_css_class("dim-label");
            }

            if metrics.year_diff >= 0.0 {
                inner.year_diff_card_value.add_css_class("overview-accent");
                inner.year_diff_card_value.remove_css_class("dim-label");
            } else {
                inner.year_diff_card_value.remove_css_class("overview-accent");
                inner.year_diff_card_value.add_css_class("dim-label");
            }

            let curve_pts = calculate_strength_history(selected_habit, &all_entries, 30, today);
            let rgba = Rgba::from_hex(&selected_habit.accent_color)
                .unwrap_or(Rgba::new(0.21, 0.52, 0.89, 1.0));

            *caches.curve_data.borrow_mut() = curve_pts;
            *caches.curve_color.borrow_mut() = rgba;
            inner.curve_drawing_area.queue_draw();

            *caches.overview_score.borrow_mut() = metrics.score;
            *caches.overview_color.borrow_mut() = rgba;
            inner.ring_drawing_area.queue_draw();

            // Continuous rolling-weeks heatmap data (Loop Habit Tracker style)
            inner
                .next_week_btn
                .set_sensitive(inner.heatmap_week_offset > 0);
            inner
                .today_week_btn
                .set_sensitive(inner.heatmap_week_offset > 0);

            let weeks_back = (inner.heatmap_week_offset + 104) as i64;
            let heatmap_start = today - chrono::Duration::weeks(weeks_back);
            let h_entries = inner
                .db
                .borrow()
                .get_entries(sel_id, heatmap_start, today)
                .unwrap_or_default();

            let mut map = HashMap::with_capacity(h_entries.len() + 100);
            for e in h_entries {
                map.insert(e.date, e.status);
            }

            let mut cur = heatmap_start;
            while cur <= today {
                if !map.contains_key(&cur) && !selected_habit.is_scheduled_on(cur.weekday()) {
                    map.insert(cur, EntryStatus::Skipped);
                }
                cur += chrono::Duration::days(1);
            }

            *caches.heatmap_entries.borrow_mut() = map;
            *caches.heatmap_color.borrow_mut() = rgba;
            *caches.heatmap_week_offset.borrow_mut() = inner.heatmap_week_offset;
            *caches.heatmap_today.borrow_mut() = today;
            inner.heatmap_drawing_area.queue_draw();
            return;
        }
    }

    inner.edit_btn.set_visible(false);
    inner.delete_btn.set_visible(false);
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

    let sidebar_scrolled = gtk4::ScrolledWindow::new();
    sidebar_scrolled.set_vexpand(true);
    sidebar_scrolled.set_hscrollbar_policy(gtk4::PolicyType::Never);

    let content_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);

    // Quick-check column headers with compact gap 6
    let quick_check_header = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    quick_check_header.set_margin_start(12);
    quick_check_header.set_margin_end(12);
    quick_check_header.set_margin_top(8);
    quick_check_header.set_margin_bottom(8);

    let habit_score_label = gtk4::Label::new(Some("HABIT & SCORE"));
    habit_score_label.set_halign(gtk4::Align::Start);
    habit_score_label.set_hexpand(true);
    habit_score_label.add_css_class("dim-label");
    habit_score_label.add_css_class("caption");
    quick_check_header.append(&habit_score_label);

    let days_header_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    days_header_box.set_halign(gtk4::Align::End);
    let today = chrono::Local::now().date_naive();
    let dates = crate::ui::habit_row::calculate_5_day_dates(today);
    let mut header_day_labels = Vec::with_capacity(5);
    for date in dates {
        let date_str = date.format("%m/%d").to_string();
        let lbl = gtk4::Label::new(Some(&date_str));
        lbl.add_css_class("dim-label");
        lbl.add_css_class("caption");
        lbl.set_size_request(50, -1);
        lbl.set_halign(gtk4::Align::Center);
        days_header_box.append(&lbl);
        header_day_labels.push(lbl);
    }
    quick_check_header.append(&days_header_box);
    content_box.append(&quick_check_header);

    let sep1 = gtk4::Separator::new(gtk4::Orientation::Horizontal);
    content_box.append(&sep1);

    let habits_list_box = gtk4::Box::new(gtk4::Orientation::Vertical, 6);
    habits_list_box.set_margin_start(12);
    habits_list_box.set_margin_end(12);
    habits_list_box.set_margin_top(8);
    habits_list_box.set_margin_bottom(8);
    content_box.append(&habits_list_box);

    sidebar_scrolled.set_child(Some(&content_box));
    sidebar_box.append(&sidebar_scrolled);

    let page = libadwaita::NavigationPage::new(&sidebar_box, "Habits");
    page.set_tag(Some("habits"));

    SidebarWidgets {
        page,
        habits_list_box,
        habit_score_label,
        header_day_labels,
        add_btn,
        menu_btn,
    }
}

#[cfg(feature = "ui")]
fn build_overview_section() -> (
    gtk4::Box,
    gtk4::DrawingArea,
    gtk4::Label,
    gtk4::Label,
    gtk4::Label,
    gtk4::Label,
) {
    let card = gtk4::Box::new(gtk4::Orientation::Vertical, 10);
    card.add_css_class("card");
    card.set_margin_start(2);
    card.set_margin_end(2);
    card.set_margin_top(2);
    card.set_margin_bottom(2);

    let overview_title = gtk4::Label::new(Some("Overview"));
    overview_title.add_css_class("heading");
    overview_title.add_css_class("overview-accent");
    overview_title.set_halign(gtk4::Align::Start);
    overview_title.set_margin_start(14);
    overview_title.set_margin_top(10);
    card.append(&overview_title);

    let stats_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 4);
    stats_box.set_margin_start(8);
    stats_box.set_margin_end(8);
    stats_box.set_margin_bottom(12);
    stats_box.set_margin_top(2);

    let ring_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    ring_box.set_valign(gtk4::Align::Center);
    ring_box.set_halign(gtk4::Align::Center);
    ring_box.set_hexpand(true);

    let ring_drawing_area = gtk4::DrawingArea::new();
    ring_drawing_area.set_content_width(38);
    ring_drawing_area.set_content_height(38);
    ring_box.append(&ring_drawing_area);
    stats_box.append(&ring_box);

    let score_card_value = gtk4::Label::new(Some("0%"));
    score_card_value.add_css_class("overview-accent");

    let month_diff_card_value = gtk4::Label::new(Some("+0%"));
    let year_diff_card_value = gtk4::Label::new(Some("+0%"));

    let total_count_card_value = gtk4::Label::new(Some("0"));
    total_count_card_value.add_css_class("overview-accent");

    let col_score = create_stat_column(&score_card_value, "Score");
    let col_month = create_stat_column(&month_diff_card_value, "Month");
    let col_year = create_stat_column(&year_diff_card_value, "Year");
    let col_total = create_stat_column(&total_count_card_value, "Total");

    stats_box.append(&col_score);
    stats_box.append(&col_month);
    stats_box.append(&col_year);
    stats_box.append(&col_total);

    card.append(&stats_box);

    (
        card,
        ring_drawing_area,
        score_card_value,
        month_diff_card_value,
        year_diff_card_value,
        total_count_card_value,
    )
}

#[cfg(feature = "ui")]
fn build_heatmap_section() -> (
    gtk4::Box,
    gtk4::Button,
    gtk4::Button,
    gtk4::Button,
    gtk4::Button,
    gtk4::DrawingArea,
) {
    let heatmap_nav_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    heatmap_nav_box.set_margin_top(4);
    heatmap_nav_box.set_margin_bottom(4);

    let heatmap_title = gtk4::Label::new(Some("History Heatmap"));
    heatmap_title.add_css_class("heading");
    heatmap_title.set_halign(gtk4::Align::Start);
    heatmap_title.set_hexpand(true);
    heatmap_nav_box.append(&heatmap_title);

    let edit_mode_btn = gtk4::Button::from_icon_name("document-edit-symbolic");
    edit_mode_btn.set_tooltip_text(Some("Edit Today's Status"));
    heatmap_nav_box.append(&edit_mode_btn);

    let nav_buttons = gtk4::Box::new(gtk4::Orientation::Horizontal, 4);
    nav_buttons.add_css_class("linked");

    let prev_week_btn = gtk4::Button::from_icon_name("go-previous-symbolic");
    prev_week_btn.set_tooltip_text(Some("Older Weeks"));
    nav_buttons.append(&prev_week_btn);

    let today_week_btn = gtk4::Button::with_label("Today");
    today_week_btn.set_tooltip_text(Some("Jump to Current"));
    nav_buttons.append(&today_week_btn);

    let next_week_btn = gtk4::Button::from_icon_name("go-next-symbolic");
    next_week_btn.set_tooltip_text(Some("Newer Weeks"));
    nav_buttons.append(&next_week_btn);

    heatmap_nav_box.append(&nav_buttons);

    let heatmap_drawing_area = gtk4::DrawingArea::new();
    heatmap_drawing_area.set_content_height(220);
    heatmap_drawing_area.set_hexpand(true);

    (
        heatmap_nav_box,
        prev_week_btn,
        today_week_btn,
        next_week_btn,
        edit_mode_btn,
        heatmap_drawing_area,
    )
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

    let delete_btn = gtk4::Button::from_icon_name("user-trash-symbolic");
    delete_btn.set_tooltip_text(Some("Delete Habit"));
    delete_btn.add_css_class("flat");
    content_header.pack_end(&delete_btn);

    content_box.append(&content_header);

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

    let schedule_label = gtk4::Label::new(None);
    schedule_label.add_css_class("dim-label");
    schedule_label.set_halign(gtk4::Align::Start);
    info_box.append(&schedule_label);

    let target_label = gtk4::Label::new(None);
    target_label.add_css_class("dim-label");
    target_label.set_halign(gtk4::Align::Start);
    info_box.append(&target_label);

    details_vbox.append(&info_box);

    let sep2 = gtk4::Separator::new(gtk4::Orientation::Horizontal);
    details_vbox.append(&sep2);

    let (
        overview_card,
        ring_drawing_area,
        score_card_value,
        month_diff_card_value,
        year_diff_card_value,
        total_count_card_value,
    ) = build_overview_section();
    details_vbox.append(&overview_card);

    let sep3 = gtk4::Separator::new(gtk4::Orientation::Horizontal);
    details_vbox.append(&sep3);

    let curve_header = gtk4::Label::new(Some("Score History (Last 30 Days)"));
    curve_header.add_css_class("heading");
    curve_header.set_halign(gtk4::Align::Start);
    details_vbox.append(&curve_header);

    let curve_drawing_area = gtk4::DrawingArea::new();
    curve_drawing_area.set_content_height(140);
    curve_drawing_area.set_hexpand(true);
    details_vbox.append(&curve_drawing_area);

    let sep4 = gtk4::Separator::new(gtk4::Orientation::Horizontal);
    details_vbox.append(&sep4);

    let (
        heatmap_nav_box,
        prev_week_btn,
        today_week_btn,
        next_week_btn,
        edit_mode_btn,
        heatmap_drawing_area,
    ) = build_heatmap_section();
    details_vbox.append(&heatmap_nav_box);
    details_vbox.append(&heatmap_drawing_area);

    let details_scrolled = gtk4::ScrolledWindow::new();
    details_scrolled.set_vexpand(true);
    details_scrolled.set_hscrollbar_policy(gtk4::PolicyType::Never);
    details_scrolled.set_child(Some(&details_vbox));
    content_box.append(&details_scrolled);

    let page = libadwaita::NavigationPage::new(&content_box, "Details");
    page.set_tag(Some("details"));

    DetailsWidgets {
        page,
        title_label,
        schedule_label,
        target_label,
        edit_btn,
        delete_btn,
        ring_drawing_area,
        score_card_value,
        month_diff_card_value,
        year_diff_card_value,
        total_count_card_value,
        curve_drawing_area,
        prev_week_btn,
        today_week_btn,
        next_week_btn,
        edit_mode_btn,
        heatmap_drawing_area,
    }
}

#[cfg(feature = "ui")]
#[allow(clippy::too_many_lines, clippy::cast_possible_truncation)]
fn show_history_editor_dialog(
    parent: &gtk4::Window,
    inner_rc: &Rc<RefCell<WindowInner>>,
    caches: &DrawCaches,
    habit_id: i64,
) {
    let (habit, initial_entries) = {
        let inner = inner_rc.borrow();
        let db = inner.db.borrow();
        let h = db.get_habit(habit_id).ok().flatten();
        let today = chrono::Local::now().date_naive();
        let start_date = today - chrono::Duration::weeks(104);
        let e = db.get_entries(habit_id, start_date, today).unwrap_or_default();
        (h, e)
    };
    let Some(habit) = habit else { return };

    let toolbar_view = libadwaita::ToolbarView::new();

    let header = libadwaita::HeaderBar::new();
    let window_title = libadwaita::WindowTitle::new(&habit.name, "");
    header.set_title_widget(Some(&window_title));
    toolbar_view.add_top_bar(&header);

    let content_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    content_box.set_margin_start(12);
    content_box.set_margin_end(12);
    content_box.set_margin_top(8);
    content_box.set_margin_bottom(12);

    // Large heatmap drawing area
    let drawing_area = gtk4::DrawingArea::new();
    drawing_area.set_size_request(-1, 280);
    drawing_area.set_hexpand(true);
    drawing_area.set_vexpand(true);
    content_box.append(&drawing_area);

    toolbar_view.set_content(Some(&content_box));

    let dialog = libadwaita::Dialog::builder()
        .title(&habit.name)
        .content_width(500)
        .content_height(340)
        .presentation_mode(libadwaita::DialogPresentationMode::BottomSheet)
        .child(&toolbar_view)
        .build();

    let week_offset = Rc::new(RefCell::new(0usize));
    let entries_map = Rc::new(RefCell::new({
        let today = chrono::Local::now().date_naive();
        let start_date = today - chrono::Duration::weeks(104);
        let mut map = HashMap::with_capacity(initial_entries.len() + 104 * 7);
        for e in initial_entries {
            map.insert(e.date, e.status);
        }
        let mut cur = start_date;
        while cur <= today {
            if !map.contains_key(&cur) && !habit.is_scheduled_on(cur.weekday()) {
                map.insert(cur, EntryStatus::Skipped);
            }
            cur += chrono::Duration::days(1);
        }
        map
    }));

    let habit_color = Rgba::from_hex(&habit.accent_color)
        .unwrap_or(Rgba::new(0.21, 0.52, 0.89, 1.0));

    // Drawing func
    {
        let week_offset_draw = Rc::clone(&week_offset);
        let entries_map_draw = Rc::clone(&entries_map);
        let color_draw = habit_color;
        drawing_area.set_draw_func(move |_, cr, w, h| {
            let today = chrono::Local::now().date_naive();
            let offset = *week_offset_draw.borrow();
            let entries = entries_map_draw.borrow();
            draw_loop_heatmap(
                cr,
                f64::from(w),
                f64::from(h),
                today,
                offset,
                &*entries,
                &color_draw,
            );
        });
    }

    // Scroll controller for mouse wheel / touchpad week scrolling
    let scroll = gtk4::EventControllerScroll::new(
        gtk4::EventControllerScrollFlags::VERTICAL | gtk4::EventControllerScrollFlags::HORIZONTAL,
    );
    {
        let offset_scroll = Rc::clone(&week_offset);
        let area_scroll = drawing_area.clone();
        scroll.connect_scroll(move |_, dx, dy| {
            let delta = if dy.abs() > dx.abs() { dy } else { dx };
            let mut off = offset_scroll.borrow_mut();
            if delta > 0.0 {
                *off = (*off + 2).min(104);
            } else if delta < 0.0 {
                *off = off.saturating_sub(2);
            }
            drop(off);
            area_scroll.queue_draw();
            gtk4::glib::Propagation::Stop
        });
    }
    drawing_area.add_controller(scroll);

    // Click gesture to directly toggle entry status
    let gesture = gtk4::GestureClick::new();
    {
        let offset_click = Rc::clone(&week_offset);
        let entries_click = Rc::clone(&entries_map);
        let area_click = drawing_area.clone();
        let inner_click = Rc::clone(inner_rc);
        let caches_click = caches.clone();
        let target_val = habit.habit_type.target().unwrap_or(1.0);

        gesture.connect_pressed(move |_, _, x, y| {
            let width = f64::from(area_click.width());
            let height = f64::from(area_click.height());
            let today = chrono::Local::now().date_naive();
            let offset = *offset_click.borrow();
            if let Some(clicked_date) =
                loop_heatmap_hit_test(x, y, width, height, today, offset)
            {
                if clicked_date <= today {
                    let is_scheduled = habit.is_scheduled_on(clicked_date.weekday());
                    let cur_status = entries_click.borrow().get(&clicked_date).copied();

                    let (new_status, new_val, delete_db) = if is_scheduled {
                        // Workday: default is unrecorded / Missed
                        match cur_status {
                            None | Some(EntryStatus::Missed) => {
                                (Some(EntryStatus::Completed), target_val, false)
                            }
                            Some(EntryStatus::Completed) => {
                                (Some(EntryStatus::Skipped), 0.0, false)
                            }
                            Some(EntryStatus::Skipped) => {
                                (None, 0.0, true)
                            }
                        }
                    } else {
                        // Inherent Rest Day: default is Skipped
                        match cur_status {
                            Some(EntryStatus::Skipped) | None => {
                                // Transition to Completed (worked out on rest day)
                                (Some(EntryStatus::Completed), target_val, false)
                            }
                            Some(EntryStatus::Completed) => {
                                // Transition to Missed (explicitly missed)
                                (Some(EntryStatus::Missed), 0.0, false)
                            }
                            Some(EntryStatus::Missed) => {
                                // Reset back to natural rest day (delete explicit DB entry)
                                (Some(EntryStatus::Skipped), 0.0, true)
                            }
                        }
                    };

                    {
                        let inner = inner_click.borrow();
                        let db = inner.db.borrow();
                        if delete_db {
                            let _ = db.delete_entry(habit_id, clicked_date);
                        } else if let Some(status) = new_status {
                            let _ = db.log_entry(habit_id, clicked_date, status, new_val);
                        }

                        if let Some(status) = new_status {
                            entries_click.borrow_mut().insert(clicked_date, status);
                        } else {
                            entries_click.borrow_mut().remove(&clicked_date);
                        }
                    }

                    area_click.queue_draw();
                    refresh(&inner_click, &caches_click);
                }
            }
        });
    }
    drawing_area.add_controller(gesture);

    // When dialog closes, ensure main window is completely refreshed
    {
        let inner_close = Rc::clone(inner_rc);
        let caches_close = caches.clone();
        dialog.connect_closed(move |_| {
            refresh(&inner_close, &caches_close);
        });
    }

    dialog.present(Some(parent));
}

#[cfg(feature = "ui")]
#[allow(clippy::too_many_lines)]
fn connect_controllers(
    inner_rc: &Rc<RefCell<WindowInner>>,
    caches: &DrawCaches,
    root_window: &gtk4::Window,
    details: &DetailsWidgets,
    add_btn: &gtk4::Button,
) {
    let inner_prev = Rc::clone(inner_rc);
    let caches_prev = caches.clone();
    details.prev_week_btn.connect_clicked(move |_| {
        {
            let mut inner = inner_prev.borrow_mut();
            inner.heatmap_week_offset += 12;
        }
        refresh(&inner_prev, &caches_prev);
    });

    let inner_today_btn = Rc::clone(inner_rc);
    let caches_today_btn = caches.clone();
    details.today_week_btn.connect_clicked(move |_| {
        {
            let mut inner = inner_today_btn.borrow_mut();
            inner.heatmap_week_offset = 0;
        }
        refresh(&inner_today_btn, &caches_today_btn);
    });

    let inner_next = Rc::clone(inner_rc);
    let caches_next = caches.clone();
    details.next_week_btn.connect_clicked(move |_| {
        {
            let mut inner = inner_next.borrow_mut();
            inner.heatmap_week_offset = inner.heatmap_week_offset.saturating_sub(12);
        }
        refresh(&inner_next, &caches_next);
    });

    let inner_edit_btn = Rc::clone(inner_rc);
    let caches_edit_btn = caches.clone();
    let win_edit = root_window.clone();
    details.edit_mode_btn.connect_clicked(move |_| {
        let selected_id = inner_edit_btn.borrow().selected_habit_id;
        if let Some(habit_id) = selected_id {
            show_history_editor_dialog(&win_edit, &inner_edit_btn, &caches_edit_btn, habit_id);
        }
    });

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
                let (nav_view, details_page) = {
                    let mut inner = inner_saved.borrow_mut();
                    inner.selected_habit_id = Some(id);
                    (inner.nav_view.clone(), inner.details_page.clone())
                };
                refresh(&inner_saved, &caches_saved);
                nav_view.push(&details_page);
            } else {
                refresh(&inner_saved, &caches_saved);
            }
        });
    });

    connect_detail_actions(inner_rc, caches, root_window, details);
}

#[cfg(feature = "ui")]
fn connect_detail_actions(
    inner_rc: &Rc<RefCell<WindowInner>>,
    caches: &DrawCaches,
    root_window: &gtk4::Window,
    details: &DetailsWidgets,
) {
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

    let inner_delete = Rc::clone(inner_rc);
    let caches_delete = caches.clone();
    let win_delete = root_window.clone();
    details.delete_btn.connect_clicked(move |_| {
        let (selected_id, db) = {
            let inner = inner_delete.borrow();
            (inner.selected_habit_id, Rc::clone(&inner.db))
        };
        if let Some(id) = selected_id {
            let habit_name = db.borrow().get_habit(id).ok().flatten().map_or_else(|| "this habit".to_string(), |h| h.name);
            let alert = libadwaita::AlertDialog::new(
                Some(&format!("Delete \"{habit_name}\"?")),
                Some("All completion history and progress data for this habit will be permanently deleted. This action cannot be undone."),
            );
            alert.add_response("cancel", "Cancel");
            alert.add_response("delete", "Delete");
            alert.set_response_appearance("delete", libadwaita::ResponseAppearance::Destructive);
            alert.set_default_response(Some("cancel"));

            let inner_cb = Rc::clone(&inner_delete);
            let caches_cb = caches_delete.clone();
            alert.choose(&win_delete, gtk4::gio::Cancellable::NONE, move |response| {
                if response == "delete" {
                    let db = {
                        let inner = inner_cb.borrow();
                        Rc::clone(&inner.db)
                    };
                    if let Err(e) = db.borrow().delete_habit(id) {
                        eprintln!("Failed to delete habit: {e}");
                    }
                    {
                        let mut inner = inner_cb.borrow_mut();
                        inner.selected_habit_id = None;
                        inner.nav_view.pop();
                    }
                    refresh(&inner_cb, &caches_cb);
                }
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
                toast.add_toast(libadwaita::Toast::new(&format!(
                    "Backup exported to {name}"
                )));
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
    let overview_score_draw = Rc::clone(&caches.overview_score);
    let overview_color_draw = Rc::clone(&caches.overview_color);
    details
        .ring_drawing_area
        .set_draw_func(move |_area, cr, width, height| {
            let score = *overview_score_draw.borrow();
            let color = overview_color_draw.borrow();
            draw_progress_ring(cr, f64::from(width), f64::from(height), score, &color);
        });

    let curve_data_draw = Rc::clone(&caches.curve_data);
    let curve_color_draw = Rc::clone(&caches.curve_color);
    details
        .curve_drawing_area
        .set_draw_func(move |_area, cr, width, height| {
            let data = curve_data_draw.borrow();
            let color = curve_color_draw.borrow();
            draw_strength_curve(cr, f64::from(width), f64::from(height), &data, &color);
        });

    let heatmap_entries_draw = Rc::clone(&caches.heatmap_entries);
    let heatmap_color_draw = Rc::clone(&caches.heatmap_color);
    let heatmap_offset_draw = Rc::clone(&caches.heatmap_week_offset);
    let heatmap_today_draw = Rc::clone(&caches.heatmap_today);
    details
        .heatmap_drawing_area
        .set_draw_func(move |_area, cr, width, height| {
            let entries = heatmap_entries_draw.borrow();
            let color = heatmap_color_draw.borrow();
            let week_offset = *heatmap_offset_draw.borrow();
            let today = *heatmap_today_draw.borrow();
            draw_loop_heatmap(
                cr,
                f64::from(width),
                f64::from(height),
                today,
                week_offset,
                &entries,
                &color,
            );
        });
}

/// Main application window providing Fast-Check List and push-navigation for details.
#[cfg(feature = "ui")]
#[derive(Debug, Clone)]
pub struct HabitWindow {
    window: libadwaita::ApplicationWindow,
    nav_view: libadwaita::NavigationView,
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
            .default_width(520)
            .default_height(700)
            .resizable(false)
            .build();

        let nav_view = libadwaita::NavigationView::new();

        let sidebar = build_sidebar();
        nav_view.add(&sidebar.page);

        let details = build_details();
        let details_clone = details.clone();

        let toast_overlay = libadwaita::ToastOverlay::new();
        toast_overlay.set_child(Some(&nav_view));
        window.set_content(Some(&toast_overlay));

        let now = chrono::Local::now();
        let today = now.date_naive();

        let caches = DrawCaches {
            curve_data: Rc::new(RefCell::new(Vec::new())),
            curve_color: Rc::new(RefCell::new(Rgba::new(0.21, 0.52, 0.89, 1.0))),
            heatmap_entries: Rc::new(RefCell::new(HashMap::new())),
            heatmap_color: Rc::new(RefCell::new(Rgba::new(0.21, 0.52, 0.89, 1.0))),
            heatmap_week_offset: Rc::new(RefCell::new(0)),
            heatmap_today: Rc::new(RefCell::new(today)),
            overview_score: Rc::new(RefCell::new(0.0)),
            overview_color: Rc::new(RefCell::new(Rgba::new(0.21, 0.52, 0.89, 1.0))),
        };

        connect_drawing_areas(&details, &caches);

        let habit_css_provider = gtk4::CssProvider::new();
        if let Some(display) = gtk4::gdk::Display::default() {
            gtk4::style_context_add_provider_for_display(
                &display,
                &habit_css_provider,
                gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }

        let db_rc = Rc::new(RefCell::new(db));
        let inner = WindowInner {
            nav_view: nav_view.clone(),
            details_page: details.page.clone(),
            db: Rc::clone(&db_rc),
            selected_habit_id: None,
            heatmap_week_offset: 0,
            habits_list_box: sidebar.habits_list_box,
            habit_score_label: sidebar.habit_score_label,
            header_day_labels: sidebar.header_day_labels,
            habit_css_provider,
            title_label: details.title_label,
            schedule_label: details.schedule_label,
            target_label: details.target_label,
            edit_btn: details.edit_btn.clone(),
            delete_btn: details.delete_btn.clone(),
            ring_drawing_area: details.ring_drawing_area,
            score_card_value: details.score_card_value,
            month_diff_card_value: details.month_diff_card_value,
            year_diff_card_value: details.year_diff_card_value,
            total_count_card_value: details.total_count_card_value,
            curve_drawing_area: details.curve_drawing_area,
            today_week_btn: details.today_week_btn,
            next_week_btn: details.next_week_btn,
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
            nav_view,
            toast_overlay,
        }
    }

    /// Returns a reference to the wrapped `libadwaita::ApplicationWindow`.
    #[must_use]
    pub const fn window(&self) -> &libadwaita::ApplicationWindow {
        &self.window
    }

    /// Returns a reference to the `libadwaita::NavigationView`.
    #[must_use]
    pub const fn nav_view(&self) -> &libadwaita::NavigationView {
        &self.nav_view
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
