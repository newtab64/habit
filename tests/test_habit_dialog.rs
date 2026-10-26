use habit::engine::models::{
    ColorError, HabitType, SCHEDULE_ALL_DAYS, SCHEDULE_FRI, SCHEDULE_MON, SCHEDULE_SAT,
    SCHEDULE_SUN, SCHEDULE_THU, SCHEDULE_TUE, SCHEDULE_WED, SCHEDULE_WEEKENDS, SCHEDULE_WORKDAYS,
};
use habit::ui::habit_dialog::{validate_habit_form, HabitValidationError};

#[test]
fn test_validate_valid_binary_habit() {
    let result = validate_habit_form(
        "  Morning Run  ",
        false,
        0.0,
        "",
        SCHEDULE_ALL_DAYS,
        "#3584e4",
    );
    assert!(result.is_ok(), "Expected valid binary habit: {result:?}");
    let habit = result.unwrap();
    assert_eq!(habit.name, "Morning Run");
    assert_eq!(habit.habit_type, HabitType::Binary);
    assert_eq!(habit.scheduled_days, SCHEDULE_ALL_DAYS);
    assert_eq!(habit.accent_color, "#3584E4");
    assert!(habit.id.is_none());
    assert!(!habit.archived);
}

#[test]
fn test_validate_valid_measurable_habit() {
    let result = validate_habit_form(
        "Read Book",
        true,
        25.5,
        "  pages  ",
        SCHEDULE_WORKDAYS,
        "ff7800",
    );
    assert!(result.is_ok(), "Expected valid measurable habit: {result:?}");
    let habit = result.unwrap();
    assert_eq!(habit.name, "Read Book");
    assert_eq!(
        habit.habit_type,
        HabitType::Measurable {
            target: 25.5,
            unit: "pages".to_string()
        }
    );
    assert_eq!(habit.scheduled_days, SCHEDULE_WORKDAYS);
    assert_eq!(habit.accent_color, "#FF7800");
}

#[test]
fn test_validate_empty_name() {
    assert_eq!(
        validate_habit_form("", false, 1.0, "", SCHEDULE_ALL_DAYS, "#3584E4"),
        Err(HabitValidationError::EmptyName)
    );
    assert_eq!(
        validate_habit_form("   ", true, 10.0, "km", SCHEDULE_ALL_DAYS, "#3584E4"),
        Err(HabitValidationError::EmptyName)
    );
}

#[test]
fn test_validate_invalid_target_for_measurable() {
    // Measurable with target <= 0.0 or non-finite
    assert_eq!(
        validate_habit_form("Drink Water", true, 0.0, "glasses", SCHEDULE_ALL_DAYS, "#3584E4"),
        Err(HabitValidationError::InvalidTarget)
    );
    assert_eq!(
        validate_habit_form("Drink Water", true, -1.0, "glasses", SCHEDULE_ALL_DAYS, "#3584E4"),
        Err(HabitValidationError::InvalidTarget)
    );
    assert_eq!(
        validate_habit_form("Drink Water", true, f64::NAN, "glasses", SCHEDULE_ALL_DAYS, "#3584E4"),
        Err(HabitValidationError::InvalidTarget)
    );
    assert_eq!(
        validate_habit_form("Drink Water", true, f64::INFINITY, "glasses", SCHEDULE_ALL_DAYS, "#3584E4"),
        Err(HabitValidationError::InvalidTarget)
    );
    assert_eq!(
        validate_habit_form("Drink Water", true, f64::NEG_INFINITY, "glasses", SCHEDULE_ALL_DAYS, "#3584E4"),
        Err(HabitValidationError::InvalidTarget)
    );

    // Binary habits do not fail on target <= 0.0
    assert!(validate_habit_form("Drink Water", false, 0.0, "", SCHEDULE_ALL_DAYS, "#3584E4").is_ok());
}

#[test]
fn test_validate_no_scheduled_days() {
    assert_eq!(
        validate_habit_form("Gym", false, 1.0, "", 0, "#3584E4"),
        Err(HabitValidationError::NoScheduledDays)
    );
    // At least 1 day should be accepted
    assert!(validate_habit_form("Gym", false, 1.0, "", SCHEDULE_MON, "#3584E4").is_ok());
}

#[test]
fn test_validate_schedule_bitmasks() {
    for bit in [
        SCHEDULE_MON,
        SCHEDULE_TUE,
        SCHEDULE_WED,
        SCHEDULE_THU,
        SCHEDULE_FRI,
        SCHEDULE_SAT,
        SCHEDULE_SUN,
        SCHEDULE_WORKDAYS,
        SCHEDULE_WEEKENDS,
        SCHEDULE_ALL_DAYS,
    ] {
        let habit = validate_habit_form("Test", false, 1.0, "", bit, "#3584E4").unwrap();
        assert_eq!(habit.scheduled_days, bit);
    }
}

#[test]
fn test_validate_invalid_hex_color() {
    match validate_habit_form("Gym", false, 1.0, "", SCHEDULE_ALL_DAYS, "") {
        Err(HabitValidationError::InvalidColor(_)) => {}
        other => panic!("Expected InvalidColor error, got {other:?}"),
    }

    match validate_habit_form("Gym", false, 1.0, "", SCHEDULE_ALL_DAYS, "ZZZZZZ") {
        Err(HabitValidationError::InvalidColor(_)) => {}
        other => panic!("Expected InvalidColor error, got {other:?}"),
    }

    match validate_habit_form("Gym", false, 1.0, "", SCHEDULE_ALL_DAYS, "#12345") {
        Err(HabitValidationError::InvalidColor(_)) => {}
        other => panic!("Expected InvalidColor error, got {other:?}"),
    }
}

#[test]
fn test_validate_hex_color_normalization() {
    let habit1 = validate_habit_form("Gym", false, 1.0, "", SCHEDULE_ALL_DAYS, "33d17a").unwrap();
    assert_eq!(habit1.accent_color, "#33D17A");

    let habit2 = validate_habit_form("Gym", false, 1.0, "", SCHEDULE_ALL_DAYS, "   #ff7800   ").unwrap();
    assert_eq!(habit2.accent_color, "#FF7800");
}

#[test]
fn test_error_display_and_trait_impls() {
    let err = HabitValidationError::EmptyName;
    assert_eq!(format!("{err}"), "habit name cannot be empty");

    let err_target = HabitValidationError::InvalidTarget;
    assert_eq!(format!("{err_target}"), "measurable target must be greater than zero");

    let err_days = HabitValidationError::NoScheduledDays;
    assert_eq!(format!("{err_days}"), "at least one scheduled day must be selected");

    let err_color = HabitValidationError::InvalidColor("bad hex".to_string());
    assert_eq!(format!("{err_color}"), "invalid accent color: bad hex");

    let boxed: Box<dyn std::error::Error> = Box::new(HabitValidationError::EmptyName);
    assert_eq!(boxed.to_string(), "habit name cannot be empty");

    // From<ColorError>
    let from_color_err: HabitValidationError = ColorError::Empty.into();
    assert_eq!(
        from_color_err,
        HabitValidationError::InvalidColor("color string cannot be empty".to_string())
    );

    // Clone & PartialEq
    let cloned = err.clone();
    assert_eq!(err, cloned);
}
