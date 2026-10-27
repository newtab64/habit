use crate::engine::db::Database;
use crate::engine::models::EntryStatus;
use crate::engine::scoring::{calculate_habit_score, calculate_strength_history};
use crate::ui::charts::Rgba;
use crate::ui::habit_row::HabitRowWidget;
use crate::ui::metrics::{
    calculate_habit_metrics, format_schedule_summary, format_target_summary, toggle_entry_state,
};
use chrono::{Datelike, NaiveDate};
use gtk4::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::PathBuf;
use std::rc::Rc;

/// Shared drawing caches across the UI components, holding RGBA colors, score curves, and heatmap entries.
#[derive(Clone)]
pub struct DrawCaches {
    pub curve_data: Rc<RefCell<Vec<f64>>>,
    pub curve_color: Rc<RefCell<Rgba>>,
    pub heatmap_entries: Rc<RefCell<HashMap<NaiveDate, EntryStatus>>>,
    pub heatmap_color: Rc<RefCell<Rgba>>,
    pub heatmap_week_offset: Rc<RefCell<usize>>,
    pub heatmap_today: Rc<RefCell<NaiveDate>>,
    pub overview_score: Rc<RefCell<f64>>,
    pub overview_color: Rc<RefCell<Rgba>>,
    pub editor_entries: Rc<RefCell<HashMap<NaiveDate, EntryStatus>>>,
    pub editor_week_offset: Rc<RefCell<usize>>,
    pub editor_color: Rc<RefCell<Rgba>>,
    pub editor_habit_id: Rc<RefCell<Option<i64>>>,
}

impl DrawCaches {
    /// Creates a new `DrawCaches` instance initialized for the given date.
    #[must_use]
    pub fn new(today: NaiveDate) -> Self {
        Self {
            curve_data: Rc::new(RefCell::new(Vec::new())),
            curve_color: Rc::new(RefCell::new(Rgba::new(0.21, 0.52, 0.89, 1.0))),
            heatmap_entries: Rc::new(RefCell::new(HashMap::new())),
            heatmap_color: Rc::new(RefCell::new(Rgba::new(0.21, 0.52, 0.89, 1.0))),
            heatmap_week_offset: Rc::new(RefCell::new(0)),
            heatmap_today: Rc::new(RefCell::new(today)),
            overview_score: Rc::new(RefCell::new(0.0)),
            overview_color: Rc::new(RefCell::new(Rgba::new(0.21, 0.52, 0.89, 1.0))),
            editor_entries: Rc::new(RefCell::new(HashMap::new())),
            editor_week_offset: Rc::new(RefCell::new(0)),
            editor_color: Rc::new(RefCell::new(Rgba::new(0.21, 0.52, 0.89, 1.0))),
            editor_habit_id: Rc::new(RefCell::new(None)),
        }
    }
}

/// Internal mutable state and widget references for the habit window.
pub struct WindowInner {
    pub nav_view: libadwaita::NavigationView,
    pub details_page: libadwaita::NavigationPage,
    pub db: Rc<Database>,
    pub selected_habit_id: Option<i64>,
    pub heatmap_week_offset: usize,
    pub habits_list_box: gtk4::Box,
    pub habit_score_label: gtk4::Label,
    pub header_day_labels: Vec<gtk4::Label>,
    pub habit_css_provider: gtk4::CssProvider,
    pub title_label: gtk4::Label,
    pub schedule_label: gtk4::Label,
    pub target_label: gtk4::Label,
    pub edit_btn: gtk4::Button,
    pub delete_btn: gtk4::Button,
    pub ring_drawing_area: gtk4::DrawingArea,
    pub score_card_value: gtk4::Label,
    pub month_diff_card_value: gtk4::Label,
    pub year_diff_card_value: gtk4::Label,
    pub total_count_card_value: gtk4::Label,
    pub curve_drawing_area: gtk4::DrawingArea,
    pub today_week_btn: gtk4::Button,
    pub next_week_btn: gtk4::Button,
    pub heatmap_drawing_area: gtk4::DrawingArea,
}

/// Returns the default SQLite database path in `XDG_DATA_HOME` or `~/.local/share/habit`.
#[must_use]
pub fn default_database_path() -> PathBuf {
    let data_dir = std::env::var_os("XDG_DATA_HOME").map_or_else(
        || {
            let home = std::env::var_os("HOME").unwrap_or_default();
            PathBuf::from(home)
                .join(".local")
                .join("share")
                .join("habit")
        },
        |val| PathBuf::from(val).join("habit"),
    );
    let _ = std::fs::create_dir_all(&data_dir);
    data_dir.join("habits.db")
}

pub use default_database_path as get_database_path;

/// Handles toggling a habit's status for a given date.
pub fn handle_toggle(
    inner_rc: &Rc<RefCell<WindowInner>>,
    caches: &DrawCaches,
    habit_id: i64,
    date: NaiveDate,
    val_opt: Option<f64>,
) {
    let (habit, current_entry) = {
        let inner = inner_rc.borrow();
        let habit = inner.db.get_habit(habit_id).ok().flatten();
        let entry = inner.db.get_entry(habit_id, date).ok().flatten();
        (habit, entry)
    };

    if let Some(habit) = habit {
        let (status, val) = toggle_entry_state(&habit, current_entry.as_ref(), val_opt);
        {
            let inner = inner_rc.borrow();
            let _ = inner.db.log_entry(habit_id, date, status, val);
        }
        refresh(inner_rc, caches);
    }
}

/// Handles selecting a habit and optionally navigating to its details view.
pub fn handle_select(inner_rc: &Rc<RefCell<WindowInner>>, caches: &DrawCaches, habit_id: i64) {
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

/// Refreshes the UI state, habits list, metrics, charts, and heatmaps.
#[allow(clippy::too_many_lines, clippy::cast_possible_wrap)]
pub fn refresh(inner_rc: &Rc<RefCell<WindowInner>>, caches: &DrawCaches) {
    let today = chrono::Local::now().date_naive();
    let mut inner = inner_rc.borrow_mut();

    let habits = inner.db.get_habits().unwrap_or_default();

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

    // Build and apply global dynamic CSS for habit row styling (accent color on symbols, progress bars, bold titles, and seamless bottom-sheet dimming)
    let mut full_css = String::from(
        ".habit-title {\n  font-weight: 700;\n}\n\
         .day-symbol {\n  font-size: 20px;\n  font-weight: bold;\n}\n\
         .loop-popup-box {\n  min-width: 200px;\n}\n\
         .loop-popup-btn {\n  min-width: 48px;\n  min-height: 42px;\n  font-size: 19px;\n  font-weight: bold;\n}\n\
         .loop-popup-btn.muted {\n  color: #888888;\n}\n\
         .editor-dimming-backdrop {\n  background-color: rgba(0, 0, 0, 0.45);\n}\n\
         .editor-bottom-sheet {\n  background-color: var(--window-bg-color);\n  border-top-left-radius: 16px;\n  border-top-right-radius: 16px;\n  border-bottom-left-radius: 0;\n  border-bottom-right-radius: 0;\n  border: 1px solid rgba(0, 0, 0, 0.12);\n  border-bottom: none;\n  box-shadow: 0 -2px 8px rgba(0, 0, 0, 0.18);\n  margin-left: 20px;\n  margin-right: 20px;\n  margin-bottom: 0;\n}\n\
         button.destructive-action.flat,\n\
         button.destructive-action.flat image {\n  color: #e01b24;\n}\n",
    );
    for h in &habits {
        if let Some(hid) = h.id {
            full_css.push_str(&crate::ui::habit_row::generate_row_css(
                hid,
                &h.accent_color,
            ));
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
                .get_entries(sel_id, start_bound, today)
                .unwrap_or_default();
            let metrics = calculate_habit_metrics(selected_habit, &all_entries, today);

            inner.score_card_value.set_text(&metrics.score_percent);
            inner
                .month_diff_card_value
                .set_text(&metrics.month_diff_text);
            inner.year_diff_card_value.set_text(&metrics.year_diff_text);
            inner
                .total_count_card_value
                .set_text(&metrics.total_count_text);

            if metrics.month_diff >= 0.0 {
                inner.month_diff_card_value.add_css_class("overview-accent");
                inner.month_diff_card_value.remove_css_class("dim-label");
            } else {
                inner
                    .month_diff_card_value
                    .remove_css_class("overview-accent");
                inner.month_diff_card_value.add_css_class("dim-label");
            }

            if metrics.year_diff >= 0.0 {
                inner.year_diff_card_value.add_css_class("overview-accent");
                inner.year_diff_card_value.remove_css_class("dim-label");
            } else {
                inner
                    .year_diff_card_value
                    .remove_css_class("overview-accent");
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
