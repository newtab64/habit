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
use crate::engine::scoring::{calculate_habit_score, calculate_streaks};
use crate::ui::habit_row::{format_measurable_value, next_toggle_status};

#[cfg(feature = "ui")]
use crate::engine::db::Database;
#[cfg(feature = "ui")]
use crate::engine::scoring::calculate_strength_history;
#[cfg(feature = "ui")]
use crate::ui::charts::{draw_loop_heatmap, draw_strength_curve, loop_heatmap_hit_test, Rgba};
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
}

/// Calculates score and streak analytics for a habit given its entries and evaluation date.
#[must_use]
pub fn calculate_habit_metrics(
    habit: &Habit,
    entries: &[HabitEntry],
    as_of: NaiveDate,
) -> HabitMetrics {
    let score = calculate_habit_score(habit, entries, as_of);
    let pct = format_score_percentage(score);
    let (current_streak, best_streak) = calculate_streaks(habit, entries, as_of);
    HabitMetrics {
        score,
        score_percent: pct.clone(),
        strength: score,
        strength_percent: pct,
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
    subtitle_label: gtk4::Label,
    color_badge: gtk4::Label,
    edit_btn: gtk4::Button,
    delete_btn: gtk4::Button,
    score_card_value: gtk4::Label,
    current_streak_card_value: gtk4::Label,
    best_streak_card_value: gtk4::Label,
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
    subtitle_label: gtk4::Label,
    color_badge: gtk4::Label,
    edit_btn: gtk4::Button,
    delete_btn: gtk4::Button,
    score_card_value: gtk4::Label,
    current_streak_card_value: gtk4::Label,
    best_streak_card_value: gtk4::Label,
    curve_drawing_area: gtk4::DrawingArea,
    prev_week_btn: gtk4::Button,
    today_week_btn: gtk4::Button,
    next_week_btn: gtk4::Button,
    edit_mode_btn: gtk4::Button,
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
                .subtitle_label
                .set_text(&format!("Schedule: {sched_text} • Target: {target_text}"));
            inner.color_badge.set_text(&selected_habit.accent_color);

            let start_bound = NaiveDate::MIN;
            let all_entries = inner
                .db
                .borrow()
                .get_entries(sel_id, start_bound, today)
                .unwrap_or_default();
            let metrics = calculate_habit_metrics(selected_habit, &all_entries, today);

            inner.score_card_value.set_text(&metrics.score_percent);
            inner
                .current_streak_card_value
                .set_text(&metrics.current_streak_text);
            inner
                .best_streak_card_value
                .set_text(&metrics.best_streak_text);

            let curve_pts = calculate_strength_history(selected_habit, &all_entries, 30, today);
            let rgba = Rgba::from_hex(&selected_habit.accent_color)
                .unwrap_or(Rgba::new(0.21, 0.52, 0.89, 1.0));

            *caches.curve_data.borrow_mut() = curve_pts;
            *caches.curve_color.borrow_mut() = rgba;
            inner.curve_drawing_area.queue_draw();

            // Continuous rolling-weeks heatmap data (Loop Habit Tracker style)
            inner
                .next_week_btn
                .set_sensitive(inner.heatmap_week_offset > 0);
            inner
                .today_week_btn
                .set_sensitive(inner.heatmap_week_offset > 0);

            let weeks_back = (inner.heatmap_week_offset + 32) as i64;
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
fn build_metrics_section() -> (gtk4::Box, gtk4::Label, gtk4::Label, gtk4::Label) {
    let metrics_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 10);
    metrics_box.set_homogeneous(true);

    let score_card_value = gtk4::Label::new(Some("0%"));
    let current_streak_card_value = gtk4::Label::new(Some("0d"));
    let best_streak_card_value = gtk4::Label::new(Some("0d"));

    let card_score = create_metric_card("Score", &score_card_value);
    let card_current = create_metric_card("Current Streak", &current_streak_card_value);
    let card_best = create_metric_card("Best Streak", &best_streak_card_value);

    metrics_box.append(&card_score);
    metrics_box.append(&card_current);
    metrics_box.append(&card_best);

    (
        metrics_box,
        score_card_value,
        current_streak_card_value,
        best_streak_card_value,
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

    let (metrics_box, score_card_value, current_streak_card_value, best_streak_card_value) =
        build_metrics_section();
    details_vbox.append(&metrics_box);

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
        subtitle_label,
        color_badge,
        edit_btn,
        delete_btn,
        score_card_value,
        current_streak_card_value,
        best_streak_card_value,
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
fn show_heatmap_date_popover(
    parent_area: &gtk4::DrawingArea,
    x: f64,
    y: f64,
    inner_rc: &Rc<RefCell<WindowInner>>,
    caches: &DrawCaches,
    habit_id: i64,
    date: NaiveDate,
) {
    let (habit, current_entry) = {
        let inner = inner_rc.borrow();
        let db = inner.db.borrow();
        let h = db.get_habit(habit_id).ok().flatten();
        let e = db.get_entry(habit_id, date).ok().flatten();
        (h, e)
    };
    let Some(habit) = habit else { return };

    let popover = gtk4::Popover::new();
    popover.set_parent(parent_area);
    let rect = gtk4::gdk::Rectangle::new(x as i32 - 4, y as i32 - 4, 8, 8);
    popover.set_pointing_to(Some(&rect));

    let pop_box = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
    pop_box.set_margin_start(8);
    pop_box.set_margin_end(8);
    pop_box.set_margin_top(6);
    pop_box.set_margin_bottom(6);

    // Date header (Loop Habit Tracker compact style)
    let date_str = date.format("%a, %b %-d, %Y").to_string();
    let header_text = if habit.habit_type.is_measurable() {
        let target = habit.habit_type.target().unwrap_or(1.0);
        let unit = habit.habit_type.unit().unwrap_or("");
        format!(
            "{date_str} (Target: {} {})",
            crate::ui::habit_row::format_measurable_value(target),
            unit
        )
    } else {
        date_str
    };
    let title_lbl = gtk4::Label::new(Some(&header_text));
    title_lbl.add_css_class("caption");
    title_lbl.add_css_class("dim-label");
    title_lbl.set_halign(gtk4::Align::Center);
    title_lbl.set_margin_bottom(4);
    pop_box.append(&title_lbl);

    let sep = gtk4::Separator::new(gtk4::Orientation::Horizontal);
    pop_box.append(&sep);

    let cur_status = current_entry.as_ref().map(|e| e.status);

    if habit.habit_type.is_measurable() {
        let btn_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        btn_box.add_css_class("linked");
        btn_box.add_css_class("loop-popup-box");
        btn_box.set_halign(gtk4::Align::Center);

        let current_val = current_entry.as_ref().map_or(0.0, |e| e.value);
        let adj = gtk4::Adjustment::new(current_val, 0.0, 100_000.0, 1.0, 5.0, 0.0);
        let spin = gtk4::SpinButton::new(Some(&adj), 1.0, 1);
        spin.set_size_request(70, -1);
        btn_box.append(&spin);

        let save_btn = gtk4::Button::with_label("SAVE");
        save_btn.add_css_class("suggested-action");
        save_btn.set_tooltip_text(Some("Save Value"));
        let inner_save = Rc::clone(inner_rc);
        let caches_save = caches.clone();
        let pop_save = popover.clone();
        let spin_save = spin;
        save_btn.connect_clicked(move |_| {
            let val = spin_save.value();
            pop_save.popdown();
            handle_toggle(&inner_save, &caches_save, habit_id, date, Some(val));
        });
        btn_box.append(&save_btn);

        let skip_btn = gtk4::Button::with_label("SKIP");
        skip_btn.set_tooltip_text(Some("Rest Day / Skip"));
        let inner_skip = Rc::clone(inner_rc);
        let caches_skip = caches.clone();
        let pop_skip = popover.clone();
        skip_btn.connect_clicked(move |_| {
            pop_skip.popdown();
            handle_toggle(&inner_skip, &caches_skip, habit_id, date, Some(-1.0));
        });
        btn_box.append(&skip_btn);

        let clear_btn = gtk4::Button::with_label("?");
        clear_btn.add_css_class("flat");
        clear_btn.set_tooltip_text(Some("Clear / Unrecorded"));
        let inner_clear = Rc::clone(inner_rc);
        let caches_clear = caches.clone();
        let pop_clear = popover.clone();
        clear_btn.connect_clicked(move |_| {
            pop_clear.popdown();
            {
                let inner = inner_clear.borrow();
                let _ = inner.db.borrow().delete_entry(habit_id, date);
            }
            refresh(&inner_clear, &caches_clear);
        });
        btn_box.append(&clear_btn);

        pop_box.append(&btn_box);
    } else {
        let btn_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        btn_box.add_css_class("linked");
        btn_box.add_css_class("loop-popup-box");
        btn_box.set_halign(gtk4::Align::Center);

        // 1. Yes / Completed button (fa_check in habit color)
        let yes_btn = gtk4::Button::with_label("✓");
        yes_btn.add_css_class("flat");
        yes_btn.add_css_class("loop-popup-btn");
        yes_btn.add_css_class(&format!("loop-popup-accent-{habit_id}"));
        yes_btn.set_tooltip_text(Some("Completed"));
        if cur_status == Some(EntryStatus::Completed) {
            yes_btn.add_css_class("suggested-action");
        }
        let inner_yes = Rc::clone(inner_rc);
        let caches_yes = caches.clone();
        let pop_yes = popover.clone();
        yes_btn.connect_clicked(move |_| {
            pop_yes.popdown();
            handle_toggle(&inner_yes, &caches_yes, habit_id, date, Some(1.0));
        });
        btn_box.append(&yes_btn);

        // 2. Skip / Rest Day button (fa_skipped in habit color)
        let skip_btn = gtk4::Button::with_label("⏸");
        skip_btn.add_css_class("flat");
        skip_btn.add_css_class("loop-popup-btn");
        skip_btn.add_css_class(&format!("loop-popup-accent-{habit_id}"));
        skip_btn.set_tooltip_text(Some("Rest Day / Skip"));
        if cur_status == Some(EntryStatus::Skipped) {
            skip_btn.add_css_class("suggested-action");
        }
        let inner_skip = Rc::clone(inner_rc);
        let caches_skip = caches.clone();
        let pop_skip = popover.clone();
        skip_btn.connect_clicked(move |_| {
            pop_skip.popdown();
            handle_toggle(&inner_skip, &caches_skip, habit_id, date, Some(-1.0));
        });
        btn_box.append(&skip_btn);

        // 3. No / Missed button (fa_times in contrast gray)
        let no_btn = gtk4::Button::with_label("✕");
        no_btn.add_css_class("flat");
        no_btn.add_css_class("loop-popup-btn");
        no_btn.add_css_class("muted");
        no_btn.set_tooltip_text(Some("Not Done"));
        if cur_status == Some(EntryStatus::Missed) {
            no_btn.add_css_class("destructive-action");
        }
        let inner_no = Rc::clone(inner_rc);
        let caches_no = caches.clone();
        let pop_no = popover.clone();
        no_btn.connect_clicked(move |_| {
            pop_no.popdown();
            handle_toggle(&inner_no, &caches_no, habit_id, date, Some(0.0));
        });
        btn_box.append(&no_btn);

        // 4. Unknown / Clear button (fa_question in contrast gray)
        let unknown_btn = gtk4::Button::with_label("?");
        unknown_btn.add_css_class("flat");
        unknown_btn.add_css_class("loop-popup-btn");
        unknown_btn.add_css_class("muted");
        unknown_btn.set_tooltip_text(Some("Clear / Unrecorded"));
        let inner_un = Rc::clone(inner_rc);
        let caches_un = caches.clone();
        let pop_un = popover.clone();
        unknown_btn.connect_clicked(move |_| {
            pop_un.popdown();
            {
                let inner = inner_un.borrow();
                let _ = inner.db.borrow().delete_entry(habit_id, date);
            }
            refresh(&inner_un, &caches_un);
        });
        btn_box.append(&unknown_btn);

        pop_box.append(&btn_box);
    }

    popover.set_child(Some(&pop_box));
    popover.popup();
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
    let area_edit_weak = details.heatmap_drawing_area.downgrade();
    details.edit_mode_btn.connect_clicked(move |_| {
        let Some(area) = area_edit_weak.upgrade() else { return };
        let selected_id = inner_edit_btn.borrow().selected_habit_id;
        if let Some(habit_id) = selected_id {
            let today = chrono::Local::now().date_naive();
            show_heatmap_date_popover(
                &area,
                f64::from(area.width()) / 2.0,
                f64::from(area.height()) / 2.0,
                &inner_edit_btn,
                &caches_edit_btn,
                habit_id,
                today,
            );
        }
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
        let (selected_id, week_offset) = {
            let inner = inner_gesture.borrow();
            (inner.selected_habit_id, inner.heatmap_week_offset)
        };
        if let Some(habit_id) = selected_id {
            let today = chrono::Local::now().date_naive();
            if let Some(clicked_date) =
                loop_heatmap_hit_test(x, y, width, height, today, week_offset)
            {
                if clicked_date <= today {
                    show_heatmap_date_popover(
                        &area,
                        x,
                        y,
                        &inner_gesture,
                        &caches_gesture,
                        habit_id,
                        clicked_date,
                    );
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
            subtitle_label: details.subtitle_label,
            color_badge: details.color_badge,
            edit_btn: details.edit_btn.clone(),
            delete_btn: details.delete_btn.clone(),
            score_card_value: details.score_card_value,
            current_streak_card_value: details.current_streak_card_value,
            best_streak_card_value: details.best_streak_card_value,
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
