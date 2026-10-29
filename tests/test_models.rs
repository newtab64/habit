use chrono::{NaiveDate, Weekday};
use habit::engine::models::{
    normalize_hex_color, ColorError, EntryStatus, Habit, HabitEntry, HabitType, SCHEDULE_ALL_DAYS,
    SCHEDULE_FRI, SCHEDULE_MON, SCHEDULE_SAT, SCHEDULE_SUN, SCHEDULE_THU, SCHEDULE_TUE,
    SCHEDULE_WED, SCHEDULE_WEEKENDS, SCHEDULE_WORKDAYS,
};

#[test]
fn test_habit_schedule_mask() {
    let gym = Habit::new_gym_example(); // scheduled Mon, Wed, Fri, Sat
    assert!(gym.is_scheduled_on(Weekday::Mon));
    assert!(!gym.is_scheduled_on(Weekday::Tue)); // pre-defined rest day
    assert!(gym.is_scheduled_on(Weekday::Wed));
    assert!(!gym.is_scheduled_on(Weekday::Sun)); // pre-defined rest day
}

#[test]
fn test_individual_weekday_bits() {
    assert_eq!(Habit::weekday_bit(Weekday::Mon), SCHEDULE_MON);
    assert_eq!(Habit::weekday_bit(Weekday::Tue), SCHEDULE_TUE);
    assert_eq!(Habit::weekday_bit(Weekday::Wed), SCHEDULE_WED);
    assert_eq!(Habit::weekday_bit(Weekday::Thu), SCHEDULE_THU);
    assert_eq!(Habit::weekday_bit(Weekday::Fri), SCHEDULE_FRI);
    assert_eq!(Habit::weekday_bit(Weekday::Sat), SCHEDULE_SAT);
    assert_eq!(Habit::weekday_bit(Weekday::Sun), SCHEDULE_SUN);

    let all_days = Habit::new_binary("Daily").with_schedule(SCHEDULE_ALL_DAYS);
    for wd in [
        Weekday::Mon,
        Weekday::Tue,
        Weekday::Wed,
        Weekday::Thu,
        Weekday::Fri,
        Weekday::Sat,
        Weekday::Sun,
    ] {
        assert!(all_days.is_scheduled_on(wd));
    }

    let workdays = Habit::new_binary("Work").with_schedule(SCHEDULE_WORKDAYS);
    assert!(workdays.is_scheduled_on(Weekday::Mon));
    assert!(workdays.is_scheduled_on(Weekday::Fri));
    assert!(!workdays.is_scheduled_on(Weekday::Sat));
    assert!(!workdays.is_scheduled_on(Weekday::Sun));

    let weekends = Habit::new_binary("Weekend").with_schedule(SCHEDULE_WEEKENDS);
    assert!(!weekends.is_scheduled_on(Weekday::Fri));
    assert!(weekends.is_scheduled_on(Weekday::Sat));
    assert!(weekends.is_scheduled_on(Weekday::Sun));
}

#[test]
fn test_habit_constructors() {
    let binary = Habit::new_binary("Hydrate");
    assert_eq!(binary.name, "Hydrate");
    assert_eq!(binary.habit_type, HabitType::Binary);
    assert_eq!(binary.scheduled_days, SCHEDULE_ALL_DAYS);
    assert!(!binary.archived);
    assert_eq!(binary.id, None);

    let measurable = Habit::new_measurable("Read", 30.0, "pages");
    assert_eq!(measurable.name, "Read");
    assert_eq!(
        measurable.habit_type,
        HabitType::Measurable {
            target: 30.0,
            unit: "pages".to_string()
        }
    );
    assert_eq!(measurable.scheduled_days, SCHEDULE_ALL_DAYS);

    let fixture = Habit::test_fixture();
    assert_ne!(fixture.name, "");
}

#[test]
fn test_hex_color_validation_and_normalization() {
    assert_eq!(normalize_hex_color("#3584e4").unwrap(), "#3584E4");
    assert_eq!(normalize_hex_color("3584e4").unwrap(), "#3584E4");
    assert_eq!(normalize_hex_color("#FF7800").unwrap(), "#FF7800");
    assert_eq!(normalize_hex_color("   #e01b24   ").unwrap(), "#E01B24");

    assert_eq!(normalize_hex_color(""), Err(ColorError::Empty));
    assert_eq!(normalize_hex_color("#123"), Err(ColorError::InvalidFormat));
    assert_eq!(
        normalize_hex_color("#1234567"),
        Err(ColorError::InvalidFormat)
    );
    assert_eq!(
        normalize_hex_color("#12345G"),
        Err(ColorError::InvalidHexDigit('G'))
    );
}

#[test]
fn test_entry_status_and_habit_entry() {
    let d = NaiveDate::from_ymd_opt(2026, 10, 26).unwrap();
    let entry = HabitEntry {
        habit_id: 42,
        date: d,
        status: EntryStatus::Completed,
        value: 1.0,
    };

    assert_eq!(entry.habit_id, 42);
    assert_eq!(entry.date, d);
    assert_eq!(entry.status, EntryStatus::Completed);
    assert_eq!(entry.value, 1.0);

    assert_eq!(EntryStatus::Completed.as_str(), "completed");
    assert_eq!(EntryStatus::Skipped.as_str(), "skipped");
    assert_eq!(EntryStatus::Missed.as_str(), "missed");

    assert_eq!(
        EntryStatus::parse_status("completed"),
        Some(EntryStatus::Completed)
    );
    assert_eq!(
        EntryStatus::parse_status("skipped"),
        Some(EntryStatus::Skipped)
    );
    assert_eq!(
        EntryStatus::parse_status("missed"),
        Some(EntryStatus::Missed)
    );
    assert_eq!(EntryStatus::parse_status("unknown"), None);
}
