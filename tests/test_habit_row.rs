use chrono::NaiveDate;
use habit::engine::models::{
    EntryStatus, Habit, HabitEntry, HabitType, SCHEDULE_ALL_DAYS, SCHEDULE_WORKDAYS,
};
use habit::ui::habit_row::{
    calculate_5_day_dates, calculate_7_day_dates, format_day_symbol, generate_row_css,
    next_toggle_status, DaySymbolDisplay,
};

#[test]
fn test_5_day_dates_today_on_left() {
    let today = NaiveDate::from_ymd_opt(2026, 10, 27).unwrap();
    let dates = calculate_5_day_dates(today);
    assert_eq!(dates.len(), 5);
    // Index 0 is today (on the left)
    assert_eq!(dates[0], today);
    assert_eq!(dates[1], today - chrono::Duration::days(1));
    assert_eq!(dates[2], today - chrono::Duration::days(2));
    assert_eq!(dates[3], today - chrono::Duration::days(3));
    assert_eq!(dates[4], today - chrono::Duration::days(4));
}

#[test]
fn test_5_day_dates_month_and_year_boundaries() {
    let jan_2 = NaiveDate::from_ymd_opt(2026, 1, 2).expect("Valid date");
    let dates_jan = calculate_5_day_dates(jan_2);
    assert_eq!(dates_jan[0], NaiveDate::from_ymd_opt(2026, 1, 2).unwrap());
    assert_eq!(dates_jan[1], NaiveDate::from_ymd_opt(2026, 1, 1).unwrap());
    assert_eq!(dates_jan[2], NaiveDate::from_ymd_opt(2025, 12, 31).unwrap());
    assert_eq!(dates_jan[3], NaiveDate::from_ymd_opt(2025, 12, 30).unwrap());
    assert_eq!(dates_jan[4], NaiveDate::from_ymd_opt(2025, 12, 29).unwrap());
}

#[test]
fn test_7_day_dates_chronological() {
    let today = NaiveDate::from_ymd_opt(2026, 10, 27).unwrap();
    let dates = calculate_7_day_dates(today);
    assert_eq!(dates[0], today - chrono::Duration::days(6));
    assert_eq!(dates[6], today);
}

#[test]
fn test_cycle_toggle_status() {
    assert_eq!(next_toggle_status(None), EntryStatus::Completed);
    assert_eq!(
        next_toggle_status(Some(EntryStatus::Missed)),
        EntryStatus::Completed
    );
    assert_eq!(
        next_toggle_status(Some(EntryStatus::Completed)),
        EntryStatus::Skipped
    );
    assert_eq!(
        next_toggle_status(Some(EntryStatus::Skipped)),
        EntryStatus::Missed
    );
}

#[test]
fn test_calculate_7_day_dates_standard() {
    let today = NaiveDate::from_ymd_opt(2026, 10, 26).expect("Valid date");
    let dates = calculate_7_day_dates(today);

    assert_eq!(dates.len(), 7);
    assert_eq!(dates[0], NaiveDate::from_ymd_opt(2026, 10, 20).unwrap()); // -6d
    assert_eq!(dates[1], NaiveDate::from_ymd_opt(2026, 10, 21).unwrap()); // -5d
    assert_eq!(dates[2], NaiveDate::from_ymd_opt(2026, 10, 22).unwrap()); // -4d
    assert_eq!(dates[3], NaiveDate::from_ymd_opt(2026, 10, 23).unwrap()); // -3d
    assert_eq!(dates[4], NaiveDate::from_ymd_opt(2026, 10, 24).unwrap()); // -2d
    assert_eq!(dates[5], NaiveDate::from_ymd_opt(2026, 10, 25).unwrap()); // -1d
    assert_eq!(dates[6], NaiveDate::from_ymd_opt(2026, 10, 26).unwrap()); // Today
}

#[test]
fn test_calculate_7_day_dates_month_and_year_boundaries() {
    // Month boundary: Nov 3 -> crosses into Oct
    let nov_3 = NaiveDate::from_ymd_opt(2026, 11, 3).expect("Valid date");
    let dates_nov = calculate_7_day_dates(nov_3);
    assert_eq!(dates_nov[0], NaiveDate::from_ymd_opt(2026, 10, 28).unwrap());
    assert_eq!(dates_nov[1], NaiveDate::from_ymd_opt(2026, 10, 29).unwrap());
    assert_eq!(dates_nov[2], NaiveDate::from_ymd_opt(2026, 10, 30).unwrap());
    assert_eq!(dates_nov[3], NaiveDate::from_ymd_opt(2026, 10, 31).unwrap());
    assert_eq!(dates_nov[4], NaiveDate::from_ymd_opt(2026, 11, 1).unwrap());
    assert_eq!(dates_nov[5], NaiveDate::from_ymd_opt(2026, 11, 2).unwrap());
    assert_eq!(dates_nov[6], NaiveDate::from_ymd_opt(2026, 11, 3).unwrap());

    // Year boundary: Jan 3, 2026 -> crosses into Dec 2025
    let jan_3 = NaiveDate::from_ymd_opt(2026, 1, 3).expect("Valid date");
    let dates_jan = calculate_7_day_dates(jan_3);
    assert_eq!(dates_jan[0], NaiveDate::from_ymd_opt(2025, 12, 28).unwrap());
    assert_eq!(dates_jan[1], NaiveDate::from_ymd_opt(2025, 12, 29).unwrap());
    assert_eq!(dates_jan[2], NaiveDate::from_ymd_opt(2025, 12, 30).unwrap());
    assert_eq!(dates_jan[3], NaiveDate::from_ymd_opt(2025, 12, 31).unwrap());
    assert_eq!(dates_jan[4], NaiveDate::from_ymd_opt(2026, 1, 1).unwrap());
    assert_eq!(dates_jan[5], NaiveDate::from_ymd_opt(2026, 1, 2).unwrap());
    assert_eq!(dates_jan[6], NaiveDate::from_ymd_opt(2026, 1, 3).unwrap());
}

#[test]
fn test_bare_symbol_formatting_binary_habits() {
    let habit = Habit::new("Read Book", HabitType::Binary, SCHEDULE_ALL_DAYS, "#3584E4").with_id(1);

    let date = NaiveDate::from_ymd_opt(2026, 10, 26).unwrap(); // Monday

    // Completed binary
    let entry_completed = HabitEntry::new(1, date, EntryStatus::Completed, 1.0);
    let sym_completed = format_day_symbol(&habit, Some(&entry_completed), date);
    assert_eq!(
        sym_completed,
        DaySymbolDisplay {
            text: "✓".to_string(),
            css_class: "symbol-completed",
            is_accent: true,
        }
    );

    // Explicitly Missed binary
    let entry_missed = HabitEntry::new(1, date, EntryStatus::Missed, 0.0);
    let sym_missed = format_day_symbol(&habit, Some(&entry_missed), date);
    assert_eq!(
        sym_missed,
        DaySymbolDisplay {
            text: "✕".to_string(),
            css_class: "symbol-missed",
            is_accent: false,
        }
    );

    // Unlogged scheduled binary
    let sym_unlogged = format_day_symbol(&habit, None, date);
    assert_eq!(
        sym_unlogged,
        DaySymbolDisplay {
            text: "✕".to_string(),
            css_class: "symbol-missed",
            is_accent: false,
        }
    );

    // Explicitly Skipped binary
    let entry_skipped = HabitEntry::new(1, date, EntryStatus::Skipped, 0.0);
    let sym_skipped = format_day_symbol(&habit, Some(&entry_skipped), date);
    assert_eq!(
        sym_skipped,
        DaySymbolDisplay {
            text: "⏸".to_string(),
            css_class: "symbol-rest",
            is_accent: true,
        }
    );
}

#[test]
fn test_bare_symbol_formatting_measurable_habits() {
    let habit = Habit::new(
        "Read Pages",
        HabitType::Measurable {
            target: 30.0,
            unit: "pages".to_string(),
        },
        SCHEDULE_ALL_DAYS,
        "#FF7800",
    )
    .with_id(2);

    let date = NaiveDate::from_ymd_opt(2026, 10, 26).unwrap();

    // Exact target reached (v == target)
    let entry_target = HabitEntry::new(2, date, EntryStatus::Completed, 30.0);
    let sym_target = format_day_symbol(&habit, Some(&entry_target), date);
    assert_eq!(
        sym_target,
        DaySymbolDisplay {
            text: "30".to_string(),
            css_class: "symbol-completed",
            is_accent: true,
        }
    );

    // Exceeded target (v > target)
    let entry_exceeded = HabitEntry::new(2, date, EntryStatus::Completed, 45.0);
    let sym_exceeded = format_day_symbol(&habit, Some(&entry_exceeded), date);
    assert_eq!(
        sym_exceeded,
        DaySymbolDisplay {
            text: "45".to_string(),
            css_class: "symbol-completed",
            is_accent: true,
        }
    );

    // Partial progress (0 < v < target)
    let entry_partial = HabitEntry::new(2, date, EntryStatus::Missed, 15.0);
    let sym_partial = format_day_symbol(&habit, Some(&entry_partial), date);
    assert_eq!(
        sym_partial,
        DaySymbolDisplay {
            text: "15".to_string(),
            css_class: "symbol-partial",
            is_accent: false,
        }
    );

    // Fractional partial progress: target 2.0, value 1.5 -> "1.5"
    let habit_float = Habit::new(
        "Water Intake",
        HabitType::Measurable {
            target: 2.0,
            unit: "L".to_string(),
        },
        SCHEDULE_ALL_DAYS,
        "#3584E4",
    )
    .with_id(3);
    let entry_float = HabitEntry::new(3, date, EntryStatus::Missed, 1.5);
    let sym_float = format_day_symbol(&habit_float, Some(&entry_float), date);
    assert_eq!(
        sym_float,
        DaySymbolDisplay {
            text: "1.5".to_string(),
            css_class: "symbol-partial",
            is_accent: false,
        }
    );

    // Zero logged on scheduled day -> "✕"
    let entry_zero = HabitEntry::new(2, date, EntryStatus::Missed, 0.0);
    let sym_zero = format_day_symbol(&habit, Some(&entry_zero), date);
    assert_eq!(
        sym_zero,
        DaySymbolDisplay {
            text: "✕".to_string(),
            css_class: "symbol-missed",
            is_accent: false,
        }
    );

    // Unlogged scheduled day -> "✕"
    let sym_unlogged = format_day_symbol(&habit, None, date);
    assert_eq!(
        sym_unlogged,
        DaySymbolDisplay {
            text: "✕".to_string(),
            css_class: "symbol-missed",
            is_accent: false,
        }
    );

    // Skipped measurable day -> "⏸"
    let entry_skipped = HabitEntry::new(2, date, EntryStatus::Skipped, 0.0);
    let sym_skipped = format_day_symbol(&habit, Some(&entry_skipped), date);
    assert_eq!(
        sym_skipped,
        DaySymbolDisplay {
            text: "⏸".to_string(),
            css_class: "symbol-rest",
            is_accent: true,
        }
    );
}

#[test]
fn test_predefined_rest_days_formatting() {
    // Habit scheduled on Monday through Friday only
    let weekday_habit = Habit::new(
        "Weekday Gym",
        HabitType::Binary,
        SCHEDULE_WORKDAYS,
        "#E01B24",
    )
    .with_id(4);

    let sat = NaiveDate::from_ymd_opt(2026, 10, 24).unwrap(); // Saturday (Rest day)
    let sun = NaiveDate::from_ymd_opt(2026, 10, 25).unwrap(); // Sunday (Rest day)
    let mon = NaiveDate::from_ymd_opt(2026, 10, 26).unwrap(); // Monday (Scheduled)

    // Unlogged rest day -> accent dash "—"
    let sym_sat = format_day_symbol(&weekday_habit, None, sat);
    assert_eq!(
        sym_sat,
        DaySymbolDisplay {
            text: "—".to_string(),
            css_class: "symbol-rest",
            is_accent: true,
        }
    );

    let sym_sun = format_day_symbol(&weekday_habit, None, sun);
    assert_eq!(
        sym_sun,
        DaySymbolDisplay {
            text: "—".to_string(),
            css_class: "symbol-rest",
            is_accent: true,
        }
    );

    // Active scheduled day without log -> "✕"
    let sym_mon = format_day_symbol(&weekday_habit, None, mon);
    assert_eq!(
        sym_mon,
        DaySymbolDisplay {
            text: "✕".to_string(),
            css_class: "symbol-missed",
            is_accent: false,
        }
    );

    // User completed a session on a pre-defined rest day (bonus day!)
    let entry_sat_done = HabitEntry::new(4, sat, EntryStatus::Completed, 1.0);
    let sym_sat_done = format_day_symbol(&weekday_habit, Some(&entry_sat_done), sat);
    assert_eq!(
        sym_sat_done,
        DaySymbolDisplay {
            text: "✓".to_string(),
            css_class: "symbol-completed",
            is_accent: true,
        }
    );
}

#[test]
fn test_generate_row_css_structure_and_styling() {
    let css = generate_row_css(42, "#3584E4");

    // Scoped row selector
    assert!(
        css.contains(".habit-row-42"),
        "Must scope styles to habit row"
    );

    // Accent color applied to symbols
    assert!(css.contains(".symbol-completed"));
    assert!(css.contains(".symbol-rest"));
    assert!(css.contains("#3584E4"));

    // Muted color applied to missed and partial symbols
    assert!(css.contains(".symbol-missed"));
    assert!(css.contains(".symbol-partial"));

    // Progress bar fill and height
    assert!(css.contains(".habit-progress progress"));
    assert!(css.contains("5px"), "Progress bar height must be 5px");

    // Test normalization when passed without leading '#'
    let css_no_hash = generate_row_css(10, "FF7800");
    assert!(css_no_hash.contains(".habit-row-10"));
    assert!(css_no_hash.contains("#FF7800"));
}
