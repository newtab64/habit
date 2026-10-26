use std::collections::HashMap;
use chrono::{Datelike, NaiveDate};
use crate::engine::models::{EntryStatus, Habit, HabitEntry, HabitType};

/// Default half-life in days for exponential habit strength decay ($T_{1/2} = 14.0$).
pub const DEFAULT_HALF_LIFE_DAYS: f64 = 14.0;

/// Computes the exponential moving average decay factor $\alpha$ from half-life in days.
///
/// Formula: $\alpha = 1.0 - 0.5^{1.0 / T_{1/2}}$.
/// Clamps half-life to a positive minimum to avoid division by zero or NaN.
#[must_use]
pub fn decay_alpha(half_life_days: f64) -> f64 {
    let t = if half_life_days <= 0.0 || half_life_days.is_nan() {
        DEFAULT_HALF_LIFE_DAYS
    } else {
        half_life_days
    };
    1.0 - 0.5_f64.powf(1.0 / t)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DayEvaluation {
    Completed,
    Missed,
    Neutral,
}

fn is_entry_completed(habit: &Habit, entry: &HabitEntry) -> bool {
    if entry.status == EntryStatus::Skipped || entry.status == EntryStatus::Missed {
        return false;
    }
    match &habit.habit_type {
        HabitType::Binary => entry.status == EntryStatus::Completed,
        HabitType::Measurable { target, .. } => {
            entry.status == EntryStatus::Completed && entry.value >= *target
        }
    }
}

fn evaluate_day(
    habit: &Habit,
    entry: Option<&HabitEntry>,
    date: NaiveDate,
    as_of: NaiveDate,
) -> DayEvaluation {
    if !habit.is_scheduled_on(date.weekday()) {
        // Pre-defined rest day
        if let Some(e) = entry {
            if is_entry_completed(habit, e) {
                return DayEvaluation::Completed;
            }
        }
        return DayEvaluation::Neutral;
    }

    // Active scheduled day:
    entry.map_or_else(
        || {
            if date == as_of {
                // Pending today (does not penalize as missed yet)
                DayEvaluation::Neutral
            } else {
                // Past unlogged scheduled day
                DayEvaluation::Missed
            }
        },
        |e| {
            if e.status == EntryStatus::Skipped {
                DayEvaluation::Neutral
            } else if is_entry_completed(habit, e) {
                DayEvaluation::Completed
            } else {
                DayEvaluation::Missed
            }
        },
    )
}

fn build_entry_map<'a>(
    habit: &Habit,
    entries: &'a [HabitEntry],
) -> HashMap<NaiveDate, &'a HabitEntry> {
    let mut map = HashMap::with_capacity(entries.len());
    for entry in entries {
        if habit.id.is_none() || habit.id == Some(entry.habit_id) {
            map.insert(entry.date, entry);
        }
    }
    map
}

fn get_start_date(habit: &Habit, entry_map: &HashMap<NaiveDate, &HabitEntry>) -> Option<NaiveDate> {
    let earliest_entry = entry_map.keys().copied().min();
    let creation_date = if habit.created_at > 0 {
        chrono::DateTime::from_timestamp(habit.created_at, 0).map(|dt| dt.naive_utc().date())
    } else {
        None
    };

    match (creation_date, earliest_entry) {
        (Some(c), Some(e)) => Some(c.min(e)),
        (Some(c), None) => Some(c),
        (None, Some(e)) => Some(e),
        (None, None) => None,
    }
}

/// Calculates the habit strength score as of the given date, in the range [0.0, 1.0].
///
/// Implements Android Loop Habit Tracker's exponential moving average decay formula:
/// - Completed day: `S_t = S_{t-1} + (1.0 - S_{t-1}) * alpha`
/// - Missed day: `S_t = S_{t-1} * (1.0 - alpha)`
/// - Pre-defined rest day / Skipped day: `S_t = S_{t-1}` (zero decay penalty)
///
/// The calculation starts from habit creation date or earliest entry date up to `as_of`.
/// If `as_of` has no entry yet, it is treated neutrally (evaluated up to yesterday / preserved).
#[must_use]
pub fn calculate_habit_strength(
    habit: &Habit,
    entries: &[HabitEntry],
    as_of: NaiveDate,
) -> f64 {
    let entry_map = build_entry_map(habit, entries);
    if entry_map.is_empty() {
        return 0.0;
    }

    let Some(start_date) = get_start_date(habit, &entry_map) else {
        return 0.0;
    };

    if start_date > as_of {
        return 0.0;
    }

    let alpha = decay_alpha(DEFAULT_HALF_LIFE_DAYS);
    let mut strength = 0.0_f64;
    let mut current_date = start_date;

    while current_date <= as_of {
        let entry = entry_map.get(&current_date).copied();
        let eval = evaluate_day(habit, entry, current_date, as_of);

        match eval {
            DayEvaluation::Completed => {
                strength = (1.0 - strength).mul_add(alpha, strength);
            }
            DayEvaluation::Missed => {
                strength *= 1.0 - alpha;
            }
            DayEvaluation::Neutral => {}
        }
        strength = strength.clamp(0.0, 1.0);

        match current_date.succ_opt() {
            Some(next) => current_date = next,
            None => break,
        }
    }

    strength
}

/// Calculates the current and best streaks as of the given date.
///
/// Returns `(current_streak, best_streak)`:
/// - Current streak: Number of consecutive active scheduled days completed up to `as_of`.
/// - Best streak: All-time maximum consecutive active scheduled days completed.
///
/// Pre-defined rest days (`!habit.is_scheduled_on(...)`) and `EntryStatus::Skipped`
/// act as neutral bridges that do not break the streak and do not increment it.
/// If `as_of` is an active scheduled day with no entry yet, the current streak is preserved
/// from the previous active scheduled day (pending user action today).
#[must_use]
pub fn calculate_streaks(
    habit: &Habit,
    entries: &[HabitEntry],
    as_of: NaiveDate,
) -> (u32, u32) {
    let entry_map = build_entry_map(habit, entries);
    if entry_map.is_empty() {
        return (0, 0);
    }

    let Some(start_date) = get_start_date(habit, &entry_map) else {
        return (0, 0);
    };

    if start_date > as_of {
        return (0, 0);
    }

    let mut current_streak: u32 = 0;
    let mut best_streak: u32 = 0;
    let mut current_date = start_date;

    while current_date <= as_of {
        let entry = entry_map.get(&current_date).copied();
        let eval = evaluate_day(habit, entry, current_date, as_of);

        match eval {
            DayEvaluation::Completed => {
                current_streak = current_streak.saturating_add(1);
                if current_streak > best_streak {
                    best_streak = current_streak;
                }
            }
            DayEvaluation::Missed => {
                current_streak = 0;
            }
            DayEvaluation::Neutral => {
                // Neutral bridge: streak preserved
            }
        }

        match current_date.succ_opt() {
            Some(next) => current_date = next,
            None => break,
        }
    }

    (current_streak, best_streak)
}

/// Calculates historical habit strength for each of the last `days` days ending at `as_of`.
///
/// Returns a vector of length `days`, ordered oldest first, with the last element
/// corresponding to `as_of`.
#[must_use]
pub fn calculate_strength_history(
    habit: &Habit,
    entries: &[HabitEntry],
    days: usize,
    as_of: NaiveDate,
) -> Vec<f64> {
    if days == 0 {
        return Vec::new();
    }

    let entry_map = build_entry_map(habit, entries);
    let days_u64 = u64::try_from(days.saturating_sub(1)).unwrap_or(0);
    let window_start = as_of
        .checked_sub_days(chrono::Days::new(days_u64))
        .unwrap_or(as_of);

    if entry_map.is_empty() {
        return vec![0.0; days];
    }

    let Some(start_date) = get_start_date(habit, &entry_map) else {
        return vec![0.0; days];
    };

    if start_date > as_of {
        return vec![0.0; days];
    }

    let alpha = decay_alpha(DEFAULT_HALF_LIFE_DAYS);
    let mut strength = 0.0_f64;
    let mut strength_by_date = HashMap::with_capacity(days);
    let mut current_date = start_date;

    while current_date <= as_of {
        let entry = entry_map.get(&current_date).copied();
        let eval = evaluate_day(habit, entry, current_date, as_of);

        match eval {
            DayEvaluation::Completed => {
                strength = (1.0 - strength).mul_add(alpha, strength);
            }
            DayEvaluation::Missed => {
                strength *= 1.0 - alpha;
            }
            DayEvaluation::Neutral => {}
        }
        strength = strength.clamp(0.0, 1.0);
        if current_date >= window_start {
            strength_by_date.insert(current_date, strength);
        }

        match current_date.succ_opt() {
            Some(next) => current_date = next,
            None => break,
        }
    }

    let mut history = Vec::with_capacity(days);
    let mut d = window_start;
    while d <= as_of {
        let s = strength_by_date.get(&d).copied().unwrap_or(0.0);
        history.push(s);
        match d.succ_opt() {
            Some(next) => d = next,
            None => break,
        }
    }

    // In case of any date boundary mismatch, guarantee length == days
    while history.len() < days {
        history.insert(0, 0.0);
    }
    if history.len() > days {
        history.truncate(days);
    }

    history
}
