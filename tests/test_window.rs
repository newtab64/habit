use chrono::NaiveDate;
use habit::engine::models::{
    EntryStatus, Habit, HabitEntry, HabitType, SCHEDULE_ALL_DAYS, SCHEDULE_FRI,
    SCHEDULE_MON, SCHEDULE_SAT, SCHEDULE_WED, SCHEDULE_WEEKENDS, SCHEDULE_WORKDAYS,
};
use habit::ui::window::{
    calculate_habit_metrics, format_schedule_summary, format_streak_badge,
    format_strength_percentage, format_target_summary, toggle_entry_state, MonthNavigation,
};

#[test]
fn test_month_navigation_rollover_next_and_prev() {
    let oct_2026 = MonthNavigation::new(2026, 10);
    assert_eq!(oct_2026.year, 2026);
    assert_eq!(oct_2026.month, 10);
    assert_eq!(oct_2026.format_title(), "October 2026");

    // Rollover: Dec 2026 -> Jan 2027
    let dec_2026 = MonthNavigation::new(2026, 12);
    assert_eq!(dec_2026.format_title(), "December 2026");
    let jan_2027 = dec_2026.next();
    assert_eq!(jan_2027.year, 2027);
    assert_eq!(jan_2027.month, 1);
    assert_eq!(jan_2027.format_title(), "January 2027");

    // Rollover: Jan 2026 -> Dec 2025
    let jan_2026 = MonthNavigation::new(2026, 1);
    assert_eq!(jan_2026.format_title(), "January 2026");
    let dec_2025 = jan_2026.prev();
    assert_eq!(dec_2025.year, 2025);
    assert_eq!(dec_2025.month, 12);
    assert_eq!(dec_2025.format_title(), "December 2025");

    // Sequential transitions
    let mid_year = MonthNavigation::new(2026, 6);
    assert_eq!(mid_year.format_title(), "June 2026");
    assert_eq!(mid_year.prev().format_title(), "May 2026");
    assert_eq!(mid_year.next().format_title(), "July 2026");

    let chain = MonthNavigation::new(2026, 11).next().next();
    assert_eq!(chain.year, 2027);
    assert_eq!(chain.month, 1);
    assert_eq!(chain.format_title(), "January 2027");
}

#[test]
fn test_month_navigation_clamping_and_bounds() {
    let clamped_low = MonthNavigation::new(2026, 0);
    assert_eq!(clamped_low.month, 1);
    assert_eq!(clamped_low.format_title(), "January 2026");

    let clamped_high = MonthNavigation::new(2026, 15);
    assert_eq!(clamped_high.month, 12);
    assert_eq!(clamped_high.format_title(), "December 2026");
}

#[test]
fn test_schedule_and_metrics_formatting() {
    // Schedule formatting
    assert_eq!(format_schedule_summary(SCHEDULE_ALL_DAYS), "Daily");
    assert_eq!(format_schedule_summary(SCHEDULE_WORKDAYS), "Weekdays (Mon-Fri)");
    assert_eq!(format_schedule_summary(SCHEDULE_WEEKENDS), "Weekends (Sat-Sun)");

    let gym_mask = SCHEDULE_MON | SCHEDULE_WED | SCHEDULE_FRI | SCHEDULE_SAT;
    assert_eq!(format_schedule_summary(gym_mask), "Mon, Wed, Fri, Sat");
    assert_eq!(format_schedule_summary(0), "None");

    // Strength percentage formatting
    assert_eq!(format_strength_percentage(0.88), "88%");
    assert_eq!(format_strength_percentage(0.884), "88%");
    assert_eq!(format_strength_percentage(0.886), "89%");
    assert_eq!(format_strength_percentage(0.0), "0%");
    assert_eq!(format_strength_percentage(1.0), "100%");

    // Streak badge formatting
    assert_eq!(format_streak_badge(0), "0d");
    assert_eq!(format_streak_badge(4), "4d");
    assert_eq!(format_streak_badge(18), "18d");

    // Target summary
    let binary_habit = Habit::new_binary("Gym");
    assert_eq!(format_target_summary(&binary_habit), "1 session");

    let measurable_habit = Habit::new_measurable("Read", 30.0, "pages");
    assert_eq!(format_target_summary(&measurable_habit), "30 pages");
}

#[test]
fn test_calculate_habit_metrics_from_entries() {
    let habit = Habit::new(
        "Workout",
        HabitType::Binary,
        SCHEDULE_ALL_DAYS,
        "#FF7800",
    ).with_id(1);

    let today = NaiveDate::from_ymd_opt(2026, 10, 26).unwrap();
    let entries = vec![
        HabitEntry::new(1, today - chrono::Duration::days(3), EntryStatus::Completed, 1.0),
        HabitEntry::new(1, today - chrono::Duration::days(2), EntryStatus::Completed, 1.0),
        HabitEntry::new(1, today - chrono::Duration::days(1), EntryStatus::Completed, 1.0),
        HabitEntry::new(1, today, EntryStatus::Completed, 1.0),
    ];

    let metrics = calculate_habit_metrics(&habit, &entries, today);
    assert_eq!(metrics.current_streak, 4);
    assert_eq!(metrics.current_streak_text, "4d");
    assert_eq!(metrics.best_streak, 4);
    assert_eq!(metrics.best_streak_text, "4d");
    assert!(metrics.strength > 0.08);
    assert!(metrics.strength_percent.ends_with('%'));
}

#[test]
fn test_toggle_entry_state_binary_and_measurable() {
    let binary = Habit::new_binary("Gym").with_id(1);
    let today = NaiveDate::from_ymd_opt(2026, 10, 26).unwrap();

    // 1. Unlogged binary -> toggles to Completed (1.0)
    let (status, val) = toggle_entry_state(&binary, None, None);
    assert_eq!(status, EntryStatus::Completed);
    assert!((val - 1.0).abs() < 1e-6);

    // 2. Completed binary -> toggles to Missed (0.0)
    let comp_entry = HabitEntry::new(1, today, EntryStatus::Completed, 1.0);
    let (status, val) = toggle_entry_state(&binary, Some(&comp_entry), None);
    assert_eq!(status, EntryStatus::Missed);
    assert!((val - 0.0).abs() < 1e-6);

    // 3. Missed binary -> toggles to Completed (1.0)
    let miss_entry = HabitEntry::new(1, today, EntryStatus::Missed, 0.0);
    let (status, val) = toggle_entry_state(&binary, Some(&miss_entry), None);
    assert_eq!(status, EntryStatus::Completed);
    assert!((val - 1.0).abs() < 1e-6);

    // 4. Measurable with explicit value >= target -> Completed
    let measurable = Habit::new_measurable("Read", 30.0, "pages").with_id(2);
    let (status, val) = toggle_entry_state(&measurable, None, Some(35.0));
    assert_eq!(status, EntryStatus::Completed);
    assert!((val - 35.0).abs() < 1e-6);

    // 5. Measurable with explicit value < target -> Missed
    let (status, val) = toggle_entry_state(&measurable, None, Some(15.0));
    assert_eq!(status, EntryStatus::Missed);
    assert!((val - 15.0).abs() < 1e-6);

    // 6. Measurable toggled without value when unlogged -> Completed with target value
    let (status, val) = toggle_entry_state(&measurable, None, None);
    assert_eq!(status, EntryStatus::Completed);
    assert!((val - 30.0).abs() < 1e-6);
}
