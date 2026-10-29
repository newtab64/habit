use crate::engine::db::Database;
use crate::engine::models::{EntryStatus, Habit, HabitEntry};
use crate::engine::scoring::{calculate_habit_score, calculate_strength_history};
use crate::ui::charts::{Rgba, DEFAULT_ACCENT};
use crate::ui::habit_row::HabitRowWidget;
use crate::ui::metrics::{
    calculate_habit_metrics, format_schedule_summary, format_target_summary, toggle_entry_state,
    HabitMetrics,
};
use chrono::{Datelike, NaiveDate};
use gtk4::prelude::*;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::PathBuf;
use std::rc::Rc;

/// Shared drawing caches across the UI components, holding RGBA colors, score curves, and heatmap entries.
#[derive(Clone)]
pub struct DrawCaches {
    pub curve_data: Rc<RefCell<Vec<f64>>>,
    pub curve_color: Rc<Cell<Rgba>>,
    pub heatmap_entries: Rc<RefCell<HashMap<NaiveDate, EntryStatus>>>,
    pub heatmap_color: Rc<Cell<Rgba>>,
    pub heatmap_week_offset: Rc<Cell<usize>>,
    pub heatmap_today: Rc<Cell<NaiveDate>>,
    pub overview_score: Rc<Cell<f64>>,
    pub overview_color: Rc<Cell<Rgba>>,
    pub editor_entries: Rc<RefCell<HashMap<NaiveDate, EntryStatus>>>,
    pub editor_week_offset: Rc<Cell<usize>>,
    pub editor_color: Rc<Cell<Rgba>>,
    pub editor_habit_id: Rc<Cell<Option<i64>>>,
    pub reorder_mode: Rc<Cell<bool>>,
    pub staged_habit_order: Rc<RefCell<Vec<i64>>>,
}

impl DrawCaches {
    /// Creates a new `DrawCaches` instance initialized for the given date.
    #[must_use]
    pub fn new(today: NaiveDate) -> Self {
        Self {
            curve_data: Rc::new(RefCell::new(Vec::new())),
            curve_color: Rc::new(Cell::new(DEFAULT_ACCENT)),
            heatmap_entries: Rc::new(RefCell::new(HashMap::new())),
            heatmap_color: Rc::new(Cell::new(DEFAULT_ACCENT)),
            heatmap_week_offset: Rc::new(Cell::new(0)),
            heatmap_today: Rc::new(Cell::new(today)),
            overview_score: Rc::new(Cell::new(0.0)),
            overview_color: Rc::new(Cell::new(DEFAULT_ACCENT)),
            editor_entries: Rc::new(RefCell::new(HashMap::new())),
            editor_week_offset: Rc::new(Cell::new(0)),
            editor_color: Rc::new(Cell::new(DEFAULT_ACCENT)),
            editor_habit_id: Rc::new(Cell::new(None)),
            reorder_mode: Rc::new(Cell::new(false)),
            staged_habit_order: Rc::new(RefCell::new(Vec::new())),
        }
    }
}

/// Internal mutable state and widget references for the habit window.
#[derive(Clone)]
pub struct WindowInner {
    pub nav_view: libadwaita::NavigationView,
    pub details_page: libadwaita::NavigationPage,
    pub toast_overlay: libadwaita::ToastOverlay,
    pub db: Rc<Database>,
    pub selected_habit_id: Option<i64>,
    pub lock_btn: gtk4::Button,
    pub add_btn: gtk4::Button,
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
        let write_res = {
            let inner = inner_rc.borrow();
            inner.db.log_entry(habit_id, date, status, val)
        };
        match write_res {
            Ok(()) => refresh(inner_rc, caches),
            Err(e) => {
                eprintln!("Failed to log entry: {e}");
                let inner = inner_rc.borrow();
                inner
                    .toast_overlay
                    .add_toast(libadwaita::Toast::new(&format!(
                        "Failed to record entry: {e}"
                    )));
            }
        }
    }
}

/// Handles selecting a habit and optionally navigating to its details view.
pub fn handle_select(inner_rc: &Rc<RefCell<WindowInner>>, caches: &DrawCaches, habit_id: i64) {
    if caches.reorder_mode.get() {
        return;
    }
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

fn update_header_dates(labels: &[gtk4::Label], today: NaiveDate) {
    let dates = crate::ui::habit_row::calculate_quick_check_dates(today);
    for (i, d) in dates.iter().enumerate() {
        if let Some(lbl) = labels.get(i) {
            lbl.set_text(&d.format("%m/%d").to_string());
        }
    }
}

/// Static application stylesheet loaded once at startup.
pub const BASE_CSS: &str = "\
.habit-title {\n  font-weight: 700;\n  padding: 0;\n  line-height: 1;\n}\n\
.day-symbol {\n  font-size: 20px;\n  font-weight: bold;\n  padding: 0;\n  line-height: 1;\n}\n\
.dragging-row {\n  background-color: rgba(53, 132, 228, 0.18);\n  border-radius: 8px;\n}\n\
.loop-popup-box {\n  min-width: 200px;\n}\n\
.loop-popup-btn {\n  min-width: 48px;\n  min-height: 42px;\n  font-size: 19px;\n  font-weight: bold;\n}\n\
.loop-popup-btn.muted {\n  color: #888888;\n}\n\
.editor-dimming-backdrop {\n  background-color: rgba(0, 0, 0, 0.45);\n}\n\
.editor-bottom-sheet {\n  background-color: var(--window-bg-color);\n  border-top-left-radius: 16px;\n  border-top-right-radius: 16px;\n  border-bottom-left-radius: 0;\n  border-bottom-right-radius: 0;\n  border: 1px solid rgba(0, 0, 0, 0.12);\n  border-bottom: none;\n  box-shadow: 0 -2px 8px rgba(0, 0, 0, 0.18);\n  margin-left: 10px;\n  margin-right: 10px;\n  margin-bottom: 0;\n}\n\
button.quick-preset-btn {\n  font-size: 13px;\n  font-weight: 600;\n  padding-left: 10px;\n  padding-right: 10px;\n  padding-top: 4px;\n  padding-bottom: 4px;\n  min-height: 28px;\n  border-radius: 8px;\n}\n\
button.destructive-action.flat,\n\
button.destructive-action.flat image {\n  color: #e01b24;\n}\n";

fn update_css(provider: &gtk4::CssProvider, habits: &[Habit], selected_habit_id: Option<i64>) {
    let mut dynamic_css = String::new();
    for h in habits {
        if let Some(hid) = h.id {
            dynamic_css.push_str(&crate::ui::habit_row::generate_row_css(
                hid,
                &h.accent_color,
            ));
        }
    }
    if let Some(hid) = selected_habit_id {
        if let Some(selected_habit) = habits.iter().find(|h| h.id == Some(hid)) {
            let color = if selected_habit.accent_color.starts_with('#') {
                selected_habit.accent_color.clone()
            } else {
                format!("#{}", selected_habit.accent_color)
            };
            let _ = write!(
                dynamic_css,
                ".overview-accent {{\n  color: {color};\n  font-weight: 700;\n}}\n"
            );
        }
    }
    provider.load_from_data(&dynamic_css);
}

fn ordered_habits_for_render(
    habits: &[Habit],
    is_reorder: bool,
    staged_order: &[i64],
) -> Vec<Habit> {
    if !is_reorder {
        return habits.to_vec();
    }
    let mut list = Vec::with_capacity(habits.len());
    for id in staged_order {
        if let Some(h) = habits.iter().find(|h| h.id == Some(*id)) {
            list.push(h.clone());
        }
    }
    for h in habits {
        if let Some(id) = h.id {
            if !staged_order.contains(&id) {
                list.push(h.clone());
            }
        }
    }
    list
}

fn rebuild_habit_list(
    inner_rc: &Rc<RefCell<WindowInner>>,
    inner: &WindowInner,
    habits: &[Habit],
    caches: &DrawCaches,
    today: NaiveDate,
) {
    let left_group = gtk4::SizeGroup::new(gtk4::SizeGroupMode::Horizontal);
    left_group.add_widget(&inner.habit_score_label);

    let col_groups: Vec<gtk4::SizeGroup> = (0..crate::ui::habit_row::QUICK_CHECK_DAYS)
        .map(|i| {
            let sg = gtk4::SizeGroup::new(gtk4::SizeGroupMode::Horizontal);
            if let Some(lbl) = inner.header_day_labels.get(i) {
                sg.add_widget(lbl);
            }
            sg
        })
        .collect();

    while let Some(child) = inner.habits_list_box.first_child() {
        inner.habits_list_box.remove(&child);
    }

    let is_reorder = caches.reorder_mode.get();
    inner.lock_btn.set_icon_name(if is_reorder {
        "changes-allow-symbolic"
    } else {
        "changes-prevent-symbolic"
    });
    inner.lock_btn.set_tooltip_text(Some(if is_reorder {
        "Lock and Save Habit Order"
    } else {
        "Unlock to Reorder Habits"
    }));
    inner.add_btn.set_sensitive(!is_reorder);

    let ordered_habits =
        ordered_habits_for_render(habits, is_reorder, &caches.staged_habit_order.borrow());

    for habit in &ordered_habits {
        let Some(habit_id) = habit.id else { continue };
        let entries = inner
            .db
            .get_entries(habit_id, NaiveDate::MIN, today)
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

        let Some(row_widget) =
            HabitRowWidget::new(habit, score, &entries, today, on_toggle, on_select)
        else {
            continue;
        };
        if is_reorder {
            row_widget.enable_reorder_drag(
                habit_id,
                &inner.habits_list_box,
                &caches.staged_habit_order,
            );
        }
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
}

fn update_charts(
    inner: &WindowInner,
    selected_habit: &Habit,
    all_entries: &[HabitEntry],
    metrics: &HabitMetrics,
    caches: &DrawCaches,
    today: NaiveDate,
) {
    let curve_pts = calculate_strength_history(selected_habit, all_entries, 30, today);
    let rgba = selected_habit.accent_rgba();

    *caches.curve_data.borrow_mut() = curve_pts;
    caches.curve_color.set(rgba);
    inner.curve_drawing_area.queue_draw();

    caches.overview_score.set(metrics.score);
    caches.overview_color.set(rgba);
    inner.ring_drawing_area.queue_draw();

    // Continuous rolling-weeks heatmap data (Loop Habit Tracker style)
    let week_offset = caches.heatmap_week_offset.get();
    inner.next_week_btn.set_sensitive(week_offset > 0);
    inner.today_week_btn.set_sensitive(week_offset > 0);

    let offset_weeks = i64::try_from(week_offset).unwrap_or(0);
    let weeks_back = offset_weeks + 104;
    let heatmap_start = today - chrono::Duration::weeks(weeks_back);
    let Some(sel_id) = selected_habit.id else {
        return;
    };
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
    caches.heatmap_color.set(rgba);
    caches.heatmap_today.set(today);
    inner.heatmap_drawing_area.queue_draw();
}

fn set_diff_style(label: &gtk4::Label, value: f64) {
    if value >= 0.0 {
        label.add_css_class("overview-accent");
        label.remove_css_class("dim-label");
    } else {
        label.remove_css_class("overview-accent");
        label.add_css_class("dim-label");
    }
}

fn update_details(inner: &WindowInner, habits: &[Habit], caches: &DrawCaches, today: NaiveDate) {
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

            set_diff_style(&inner.month_diff_card_value, metrics.month_diff);
            set_diff_style(&inner.year_diff_card_value, metrics.year_diff);

            update_charts(inner, selected_habit, &all_entries, &metrics, caches, today);
            return;
        }
    }

    inner.edit_btn.set_visible(false);
    inner.delete_btn.set_visible(false);
}

/// Refreshes the UI state, habits list, metrics, charts, and heatmaps.
///
/// NOTE: No `RefCell` borrow (shared or mutable) on `inner_rc` is held while
/// invoking GTK widget methods, preventing reentrant dynamic borrow panics
/// if widget calls trigger signal emission or reentrant callbacks.
pub fn refresh(inner_rc: &Rc<RefCell<WindowInner>>, caches: &DrawCaches) {
    let today = chrono::Local::now().date_naive();
    let (inner, habits) = {
        let mut inner_guard = inner_rc.borrow_mut();
        let habits = inner_guard.db.get_habits().unwrap_or_default();
        if inner_guard.selected_habit_id.is_none() && !habits.is_empty() {
            inner_guard.selected_habit_id = habits.first().and_then(|h| h.id);
        } else if let Some(sel_id) = inner_guard.selected_habit_id {
            if !habits.iter().any(|h| h.id == Some(sel_id)) {
                inner_guard.selected_habit_id = habits.first().and_then(|h| h.id);
            }
        }
        (inner_guard.clone(), habits)
    };

    update_header_dates(&inner.header_day_labels, today);
    update_css(&inner.habit_css_provider, &habits, inner.selected_habit_id);
    rebuild_habit_list(inner_rc, &inner, &habits, caches, today);
    update_details(&inner, &habits, caches, today);
}
