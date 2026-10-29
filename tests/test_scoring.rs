pub mod common;
use common::HabitTestExt;

use chrono::NaiveDate;
use habit::engine::models::{EntryStatus, Habit, HabitEntry, HabitType, SCHEDULE_ALL_DAYS};
use habit::engine::scoring::{
    calculate_habit_score, calculate_streaks, calculate_strength_history, DEFAULT_HALF_LIFE_DAYS,
};

#[test]
fn test_asymmetric_scoring_harder_to_gain() {
    let habit = Habit::new("Exercise", HabitType::Binary, SCHEDULE_ALL_DAYS, "#3584e4");
    let today = NaiveDate::from_ymd_opt(2026, 10, 27).unwrap();

    // 7 consecutive completed days should yield ~15.9% (with 28-day gain half-life, not 30%+)
    let mut entries = Vec::new();
    for i in (0..7).rev() {
        let d = today - chrono::Duration::days(i);
        entries.push(HabitEntry::new(0, d, EntryStatus::Completed, 1.0));
    }
    let score = calculate_habit_score(&habit, &entries, today);
    assert!((score - 0.159).abs() < 0.02, "Expected ~15.9%, got {score}");
}

#[test]
fn test_predefined_rest_days_do_not_decay_or_break_streak() {
    let gym = Habit::new_gym_example(); // Mon, Wed, Fri, Sat
                                        // Mon: Done, Tue: Pre-defined Rest, Wed: Done
    let d1 = NaiveDate::from_ymd_opt(2026, 10, 19).unwrap(); // Mon
    let d2 = NaiveDate::from_ymd_opt(2026, 10, 20).unwrap(); // Tue (rest)
    let d3 = NaiveDate::from_ymd_opt(2026, 10, 21).unwrap(); // Wed

    let entries = vec![
        HabitEntry {
            habit_id: 1,
            date: d1,
            status: EntryStatus::Completed,
            value: 1.0,
        },
        HabitEntry {
            habit_id: 1,
            date: d2,
            status: EntryStatus::Skipped,
            value: 0.0,
        },
        HabitEntry {
            habit_id: 1,
            date: d3,
            status: EntryStatus::Completed,
            value: 1.0,
        },
    ];

    let (curr, best) = calculate_streaks(&gym, &entries, d3);
    assert_eq!(curr, 2); // 2 workout sessions completed across rest day
    assert_eq!(best, 2);

    let strength_d1 = calculate_habit_score(&gym, &entries, d1);
    let strength_d2 = calculate_habit_score(&gym, &entries, d2);
    let strength_d3 = calculate_habit_score(&gym, &entries, d3);

    // Tuesday (rest day) preserves strength without decay
    assert!((strength_d1 - strength_d2).abs() < f64::EPSILON);
    // Wednesday increases strength
    assert!(strength_d3 > strength_d2);
}

#[test]
fn test_predefined_rest_days_without_logged_entries() {
    let gym = Habit::new_gym_example(); // Mon, Wed, Fri, Sat
    let d1 = NaiveDate::from_ymd_opt(2026, 10, 19).unwrap(); // Mon
    let d2 = NaiveDate::from_ymd_opt(2026, 10, 20).unwrap(); // Tue (rest, unlogged)
    let d3 = NaiveDate::from_ymd_opt(2026, 10, 21).unwrap(); // Wed

    // Notice no entry for d2 (Tue)
    let entries = vec![
        HabitEntry::new(1, d1, EntryStatus::Completed, 1.0),
        HabitEntry::new(1, d3, EntryStatus::Completed, 1.0),
    ];

    let (curr, best) = calculate_streaks(&gym, &entries, d3);
    assert_eq!(curr, 2);
    assert_eq!(best, 2);

    let s1 = calculate_habit_score(&gym, &entries, d1);
    let s2 = calculate_habit_score(&gym, &entries, d2);
    assert!((s1 - s2).abs() < f64::EPSILON);
}

#[test]
fn test_measurable_partial_progress_does_not_add_strength_or_keep_streak() {
    let reading = Habit::new_measurable("Read", 30.0, "pages");
    let d = NaiveDate::from_ymd_opt(2026, 10, 26).unwrap();
    let partial_entries = vec![HabitEntry {
        habit_id: 2,
        date: d,
        status: EntryStatus::Missed,
        value: 15.0,
    }];
    let (curr, _) = calculate_streaks(&reading, &partial_entries, d);
    assert_eq!(curr, 0); // Partial progress breaks streak
    let strength = calculate_habit_score(&reading, &partial_entries, d);
    assert_eq!(strength, 0.0); // Does not add to strength
}

#[test]
fn test_measurable_completion_meets_or_exceeds_target() {
    let reading = Habit::new_measurable("Read", 30.0, "pages");
    let d1 = NaiveDate::from_ymd_opt(2026, 10, 25).unwrap();
    let d2 = NaiveDate::from_ymd_opt(2026, 10, 26).unwrap();

    let entries = vec![
        HabitEntry::new(2, d1, EntryStatus::Completed, 30.0),
        HabitEntry::new(2, d2, EntryStatus::Completed, 45.0),
    ];

    let (curr, best) = calculate_streaks(&reading, &entries, d2);
    assert_eq!(curr, 2);
    assert_eq!(best, 2);

    let s1 = calculate_habit_score(&reading, &entries, d1);
    let s2 = calculate_habit_score(&reading, &entries, d2);
    assert!(s1 > 0.0);
    assert!(s2 > s1);
}

#[test]
fn test_exponential_decay_half_life() {
    let habit = Habit::new_binary("Daily Walk");
    let d0 = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
    let mut entries = vec![HabitEntry::new(1, d0, EntryStatus::Completed, 1.0)];

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let half_life_days = DEFAULT_HALF_LIFE_DAYS as u64;
    // 14 consecutive missed days
    for i in 1..=half_life_days {
        let date = d0 + chrono::Days::new(i);
        entries.push(HabitEntry::new(1, date, EntryStatus::Missed, 0.0));
    }

    let initial_strength = calculate_habit_score(&habit, &entries[..1], d0);
    let d14 = d0 + chrono::Days::new(half_life_days);
    let half_strength = calculate_habit_score(&habit, &entries, d14);

    // After DEFAULT_HALF_LIFE_DAYS (14 days), strength should decay to half: S_14 ≈ S_0 * 0.5
    let ratio = half_strength / initial_strength;
    assert!(
        (ratio - 0.5).abs() < 1e-4,
        "Expected decay ratio ~0.5, got {ratio}"
    );
}

#[test]
fn test_unlogged_past_scheduled_day_breaks_streak_and_decays() {
    let habit = Habit::new_binary("Meditate");
    let d1 = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
    // d2 is unlogged
    let d3 = NaiveDate::from_ymd_opt(2026, 10, 3).unwrap();

    let entries = vec![
        HabitEntry::new(1, d1, EntryStatus::Completed, 1.0),
        HabitEntry::new(1, d3, EntryStatus::Completed, 1.0),
    ];

    let (curr, best) = calculate_streaks(&habit, &entries, d3);
    // Day 2 was unlogged on a daily habit, breaking the streak. Day 3 restarted streak at 1.
    assert_eq!(curr, 1);
    assert_eq!(best, 1);

    // In history from d3, d2 was an unlogged past day, so d2 decayed compared to d1
    let history = calculate_strength_history(&habit, &entries, 3, d3);
    assert_eq!(history.len(), 3);
    assert!(
        history[1] < history[0],
        "Expected d2 ({}) to have decayed from d1 ({})",
        history[1],
        history[0]
    );
}

#[test]
fn test_today_unlogged_preserves_streak_and_strength() {
    let habit = Habit::new_binary("Meditate");
    let d1 = NaiveDate::from_ymd_opt(2026, 10, 25).unwrap();
    let d2 = NaiveDate::from_ymd_opt(2026, 10, 26).unwrap(); // as_of (today)

    let entries = vec![HabitEntry::new(1, d1, EntryStatus::Completed, 1.0)];

    let (curr, best) = calculate_streaks(&habit, &entries, d2);
    // Pending today preserves yesterday's active streak
    assert_eq!(curr, 1);
    assert_eq!(best, 1);

    let s1 = calculate_habit_score(&habit, &entries, d1);
    let s2 = calculate_habit_score(&habit, &entries, d2);
    // Does not penalize today yet
    assert!((s1 - s2).abs() < f64::EPSILON);
}

#[test]
fn test_today_explicitly_missed_resets_streak() {
    let habit = Habit::new_binary("Meditate");
    let d1 = NaiveDate::from_ymd_opt(2026, 10, 25).unwrap();
    let d2 = NaiveDate::from_ymd_opt(2026, 10, 26).unwrap(); // as_of (today)

    let entries = vec![
        HabitEntry::new(1, d1, EntryStatus::Completed, 1.0),
        HabitEntry::new(1, d2, EntryStatus::Missed, 0.0),
    ];

    let (curr, best) = calculate_streaks(&habit, &entries, d2);
    assert_eq!(curr, 0);
    assert_eq!(best, 1);

    let s1 = calculate_habit_score(&habit, &entries, d1);
    let s2 = calculate_habit_score(&habit, &entries, d2);
    assert!(s2 < s1);
}

#[test]
fn test_manual_skip_on_scheduled_day_acts_as_neutral_bridge() {
    let habit = Habit::new_binary("Daily Workout");
    let d1 = NaiveDate::from_ymd_opt(2026, 10, 20).unwrap();
    let d2 = NaiveDate::from_ymd_opt(2026, 10, 21).unwrap();
    let d3 = NaiveDate::from_ymd_opt(2026, 10, 22).unwrap();

    let entries = vec![
        HabitEntry::new(1, d1, EntryStatus::Completed, 1.0),
        HabitEntry::new(1, d2, EntryStatus::Skipped, 0.0), // sick day
        HabitEntry::new(1, d3, EntryStatus::Completed, 1.0),
    ];

    let (curr, best) = calculate_streaks(&habit, &entries, d3);
    assert_eq!(curr, 2);
    assert_eq!(best, 2);

    let s1 = calculate_habit_score(&habit, &entries, d1);
    let s2 = calculate_habit_score(&habit, &entries, d2);
    assert!((s1 - s2).abs() < f64::EPSILON);
}

#[test]
fn test_strength_history() {
    let habit = Habit::new_binary("Daily Floss");
    let d1 = NaiveDate::from_ymd_opt(2026, 10, 20).unwrap();
    let d2 = NaiveDate::from_ymd_opt(2026, 10, 21).unwrap();
    let d3 = NaiveDate::from_ymd_opt(2026, 10, 22).unwrap();

    let entries = vec![
        HabitEntry::new(1, d1, EntryStatus::Completed, 1.0),
        HabitEntry::new(1, d2, EntryStatus::Completed, 1.0),
        HabitEntry::new(1, d3, EntryStatus::Completed, 1.0),
    ];

    // Request 5 days history ending at d3 (d-1, d0, d1, d2, d3)
    let history = calculate_strength_history(&habit, &entries, 5, d3);
    assert_eq!(history.len(), 5);

    // Days before d1 should be 0.0
    assert_eq!(history[0], 0.0);
    assert_eq!(history[1], 0.0);

    // Days d1..d3 should match calculate_habit_score
    let s1 = calculate_habit_score(&habit, &entries, d1);
    let s2 = calculate_habit_score(&habit, &entries, d2);
    let s3 = calculate_habit_score(&habit, &entries, d3);

    assert!((history[2] - s1).abs() < 1e-9);
    assert!((history[3] - s2).abs() < 1e-9);
    assert!((history[4] - s3).abs() < 1e-9);
    assert_eq!(history[4], s3);
}

#[test]
fn test_empty_entries_handling() {
    let habit = Habit::new_binary("Floss");
    let d = NaiveDate::from_ymd_opt(2026, 10, 26).unwrap();
    let entries: Vec<HabitEntry> = Vec::new();

    let (curr, best) = calculate_streaks(&habit, &entries, d);
    assert_eq!(curr, 0);
    assert_eq!(best, 0);

    let strength = calculate_habit_score(&habit, &entries, d);
    assert_eq!(strength, 0.0);

    let history = calculate_strength_history(&habit, &entries, 7, d);
    assert_eq!(history.len(), 7);
    assert!(history.iter().all(|&v| v == 0.0));
}

#[test]
fn test_best_streak_tracks_all_time_maximum() {
    let habit = Habit::new_binary("Gym");
    let base = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
    let mut entries = Vec::new();

    // 5 completed days
    for i in 0..5 {
        entries.push(HabitEntry::new(
            1,
            base + chrono::Days::new(i),
            EntryStatus::Completed,
            1.0,
        ));
    }
    // 1 missed day
    entries.push(HabitEntry::new(
        1,
        base + chrono::Days::new(5),
        EntryStatus::Missed,
        0.0,
    ));
    // 2 completed days
    for i in 6..8 {
        entries.push(HabitEntry::new(
            1,
            base + chrono::Days::new(i),
            EntryStatus::Completed,
            1.0,
        ));
    }

    let as_of = base + chrono::Days::new(7);
    let (curr, best) = calculate_streaks(&habit, &entries, as_of);
    assert_eq!(curr, 2);
    assert_eq!(best, 5);
}

#[test]
fn test_weekend_only_habit_streak_and_strength() {
    let weekend_habit = Habit::new(
        "Weekend Hike",
        habit::engine::models::HabitType::Binary,
        habit::engine::models::SCHEDULE_WEEKENDS,
        "#33D17A",
    );

    // Sat Oct 17, 2026 and Sun Oct 18, 2026
    let sat1 = NaiveDate::from_ymd_opt(2026, 10, 17).unwrap();
    let sun1 = NaiveDate::from_ymd_opt(2026, 10, 18).unwrap();
    // Sat Oct 24, 2026 and Sun Oct 25, 2026
    let sat2 = NaiveDate::from_ymd_opt(2026, 10, 24).unwrap();
    let sun2 = NaiveDate::from_ymd_opt(2026, 10, 25).unwrap();

    let entries = vec![
        HabitEntry::new(1, sat1, EntryStatus::Completed, 1.0),
        HabitEntry::new(1, sun1, EntryStatus::Completed, 1.0),
        // Mon-Fri are rest days, no entries needed
        HabitEntry::new(1, sat2, EntryStatus::Completed, 1.0),
        HabitEntry::new(1, sun2, EntryStatus::Completed, 1.0),
    ];

    let (curr, best) = calculate_streaks(&weekend_habit, &entries, sun2);
    assert_eq!(curr, 4); // 4 weekend sessions completed across 5 rest days
    assert_eq!(best, 4);

    let strength = calculate_habit_score(&weekend_habit, &entries, sun2);
    assert!(strength > 0.0);
}

#[test]
fn test_entries_after_as_of_are_ignored() {
    let habit = Habit::new_binary("Journal");
    let d1 = NaiveDate::from_ymd_opt(2026, 10, 20).unwrap();
    let d2 = NaiveDate::from_ymd_opt(2026, 10, 21).unwrap();
    let d3 = NaiveDate::from_ymd_opt(2026, 10, 22).unwrap(); // Future

    let entries = vec![
        HabitEntry::new(1, d1, EntryStatus::Completed, 1.0),
        HabitEntry::new(1, d2, EntryStatus::Completed, 1.0),
        HabitEntry::new(1, d3, EntryStatus::Completed, 1.0),
    ];

    // As of d2, d3 should not be counted
    let (curr, best) = calculate_streaks(&habit, &entries, d2);
    assert_eq!(curr, 2);
    assert_eq!(best, 2);

    let s2 = calculate_habit_score(&habit, &entries, d2);
    let s3 = calculate_habit_score(&habit, &entries, d3);
    assert!(s3 > s2);
}
