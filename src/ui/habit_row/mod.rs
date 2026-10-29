use crate::engine::models::{EntryStatus, Habit, HabitEntry, HabitType};
use chrono::{Datelike, NaiveDate};

#[cfg(feature = "ui")]
mod widget;
#[cfg(feature = "ui")]
pub use widget::HabitRowWidget;

/// Number of days displayed in the quick-check row (5 days: today down to today - 4).
pub const QUICK_CHECK_DAYS: usize = 5;

/// Calculates the array of `QUICK_CHECK_DAYS` consecutive dates starting from `today` on the left (`today` down to `today - 4`).
#[must_use]
pub fn calculate_quick_check_dates(today: NaiveDate) -> [NaiveDate; QUICK_CHECK_DAYS] {
    std::array::from_fn(|i| {
        let offset = i64::try_from(i).unwrap_or(0);
        today
            .checked_sub_signed(chrono::Duration::days(offset))
            .unwrap_or(today)
    })
}

/// Visual display attributes for a single day column in a habit row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaySymbolDisplay {
    /// Character or formatted string to display (e.g. "✓", "—", "✕", or "30").
    pub text: String,
    /// Scoped CSS class to apply for color and styling.
    pub css_class: &'static str,
    /// Whether the symbol should be highlighted with the habit's accent color.
    pub is_accent: bool,
}

/// Cycles to the next entry status in the sequence:
/// `None / Missed` -> `Completed` -> `Skipped` -> `Missed`.
#[must_use]
pub const fn next_toggle_status(current: Option<EntryStatus>) -> EntryStatus {
    match current {
        None | Some(EntryStatus::Missed) => EntryStatus::Completed,
        Some(EntryStatus::Completed) => EntryStatus::Skipped,
        Some(EntryStatus::Skipped) => EntryStatus::Missed,
    }
}

/// Formats a floating-point measurable value without unnecessary trailing `.0` or decimals.
///
/// For example, `30.0` becomes `"30"`, and `1.5` becomes `"1.5"`.
#[must_use]
pub fn format_measurable_value(val: f64) -> String {
    if val.is_nan() || val.is_infinite() {
        return "0".to_string();
    }
    if (val - val.round()).abs() < 1e-6 {
        format!("{:.0}", val.round())
    } else {
        let s = format!("{val:.2}");
        let trimmed = s.trim_end_matches('0').trim_end_matches('.');
        trimmed.to_string()
    }
}

/// Formats the bare symbol display attributes for a habit on a specific date.
///
/// Rules:
/// - Explicit manual skip or pre-defined rest day without completion: `"—"`, `symbol-rest`, accent.
/// - Binary completed: `"✓"`, `symbol-completed`, accent.
/// - Measurable completed ($v \ge \text{target}$): formatted number string, `symbol-completed`, accent.
/// - Measurable partial ($0 < v < \text{target}$): formatted number string, `symbol-partial`, muted.
/// - Missed or unlogged past scheduled day: `"✕"`, `symbol-missed`, muted.
#[must_use]
pub fn format_day_symbol(
    habit: &Habit,
    entry: Option<&HabitEntry>,
    date: NaiveDate,
) -> DaySymbolDisplay {
    let is_scheduled = habit.is_scheduled_on(date.weekday());

    // Explicit manual skip displays as rest dash symbol ("—")
    if let Some(e) = entry {
        if e.status == EntryStatus::Skipped {
            return DaySymbolDisplay {
                text: "—".to_string(),
                css_class: "symbol-rest",
                is_accent: true,
            };
        }
    }

    match &habit.habit_type {
        HabitType::Binary => {
            if let Some(e) = entry {
                if e.status == EntryStatus::Completed {
                    return DaySymbolDisplay {
                        text: "✓".to_string(),
                        css_class: "symbol-completed",
                        is_accent: true,
                    };
                }
            }
        }
        HabitType::Measurable { target, .. } => {
            if let Some(e) = entry {
                if e.value >= *target && (e.status == EntryStatus::Completed || e.value > 0.0) {
                    return DaySymbolDisplay {
                        text: format_measurable_value(e.value),
                        css_class: "symbol-completed",
                        is_accent: true,
                    };
                }
                if e.value > 0.0 {
                    return DaySymbolDisplay {
                        text: format_measurable_value(e.value),
                        css_class: "symbol-partial",
                        is_accent: false,
                    };
                }
            }
        }
    }

    if is_scheduled {
        DaySymbolDisplay {
            text: "✕".to_string(),
            css_class: "symbol-missed",
            is_accent: false,
        }
    } else {
        DaySymbolDisplay {
            text: "—".to_string(),
            css_class: "symbol-rest",
            is_accent: true,
        }
    }
}

/// Generates scoped CSS for `.habit-row-{habit_id}` tinting day symbols.
#[must_use]
pub fn generate_row_css(habit_id: i64, hex_color: &str) -> String {
    let color = if hex_color.starts_with('#') {
        hex_color.to_string()
    } else {
        format!("#{hex_color}")
    };

    format!(
        ".habit-row-{habit_id} .symbol-completed,\n\
         .habit-row-{habit_id} .symbol-rest,\n\
         .habit-row-{habit_id} button.day-symbol-button label.symbol-completed,\n\
         .habit-row-{habit_id} button.day-symbol-button label.symbol-rest,\n\
         .habit-row-{habit_id} .day-symbol-button .symbol-completed,\n\
         .habit-row-{habit_id} .day-symbol-button .symbol-rest {{\n  \
             color: {color};\n\
         }}\n\
         .habit-row-{habit_id} .symbol-missed,\n\
         .habit-row-{habit_id} .symbol-partial,\n\
         .habit-row-{habit_id} button.day-symbol-button label.symbol-missed,\n\
         .habit-row-{habit_id} button.day-symbol-button label.symbol-partial,\n\
         .habit-row-{habit_id} .day-symbol-button .symbol-missed,\n\
         .habit-row-{habit_id} .day-symbol-button .symbol-partial {{\n  \
             color: #77767b;\n  \
             opacity: 0.55;\n\
         }}\n\
         .habit-row-{habit_id} button.day-symbol-button,\n\
         .habit-row-{habit_id} .day-symbol-button {{\n  \
             border: none;\n  \
             background: none;\n  \
             box-shadow: none;\n  \
             min-width: 42px;\n  \
             min-height: 46px;\n  \
             padding: 0;\n  \
             margin: 0;\n\
         }}\n\
         .habit-row-{habit_id} button.day-symbol-button label {{\n  \
             padding: 0;\n  \
             line-height: 1;\n\
         }}\n\
         .loop-popup-accent-{habit_id} {{\n  \
             color: {color};\n\
         }}\n"
    )
}

/// Computes the target row index for a dragged row currently at `current_idx` when the pointer is at `pointer_y`.
///
/// `row_spans` is a slice of `(top_y, height)` for each row in visual order.
#[must_use]
pub fn target_index_for_drag_y(
    row_spans: &[(f64, f64)],
    current_idx: usize,
    pointer_y: f64,
) -> usize {
    if row_spans.is_empty() {
        return 0;
    }
    let clamped_current = current_idx.min(row_spans.len() - 1);
    let mut target = clamped_current;
    for (idx, &(top, height)) in row_spans.iter().enumerate() {
        let midpoint = height.mul_add(0.5, top);
        if idx > clamped_current && pointer_y > midpoint {
            target = idx;
        }
    }
    for (idx, &(top, height)) in row_spans.iter().enumerate().rev() {
        let midpoint = height.mul_add(0.5, top);
        if idx < clamped_current && pointer_y < midpoint {
            target = idx;
        }
    }
    target
}

/// Moves `dragged_id` to the position occupied by `target_id` inside `order`.
///
/// Returns `true` if `order` was modified, or `false` if `dragged_id == target_id`
/// or either ID is not present in `order`.
pub fn move_habit_in_order(order: &mut Vec<i64>, dragged_id: i64, target_id: i64) -> bool {
    if dragged_id == target_id {
        return false;
    }
    let Some(from_idx) = order.iter().position(|&id| id == dragged_id) else {
        return false;
    };
    let Some(to_idx) = order.iter().position(|&id| id == target_id) else {
        return false;
    };
    let item = order.remove(from_idx);
    order.insert(to_idx, item);
    true
}
