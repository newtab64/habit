//! Domain metrics calculation, navigation, and formatting utilities for habit tracking.
//!
//! This module contains pure logic for monthly navigation, schedule summaries,
//! badge and percentage formatting, habit metric calculations, and entry toggle state transitions.
//! It has zero dependencies on UI toolkits (GTK, Libadwaita, Cairo).

use chrono::NaiveDate;

use crate::engine::models::{
    EntryStatus, Habit, HabitEntry, HabitType, SCHEDULE_ALL_DAYS, SCHEDULE_FRI, SCHEDULE_MON,
    SCHEDULE_SAT, SCHEDULE_SUN, SCHEDULE_THU, SCHEDULE_TUE, SCHEDULE_WED, SCHEDULE_WEEKENDS,
    SCHEDULE_WORKDAYS,
};
use crate::engine::scoring::{calculate_habit_score, calculate_streaks};
use crate::ui::habit_row::{format_measurable_value, next_toggle_status};

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
