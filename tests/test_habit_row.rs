use std::cell::RefCell;
use std::rc::Rc;

use chrono::NaiveDate;
use habit::engine::models::{
    EntryStatus, Habit, HabitEntry, HabitType, SCHEDULE_ALL_DAYS, SCHEDULE_WORKDAYS,
};
use habit::ui::habit_row::{
    calculate_5_day_dates, format_day_symbol, generate_row_css, next_toggle_status,
    DaySymbolDisplay,
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

    // Explicitly Skipped binary -> "—" (dash, not "⏸")
    let entry_skipped = HabitEntry::new(1, date, EntryStatus::Skipped, 0.0);
    let sym_skipped = format_day_symbol(&habit, Some(&entry_skipped), date);
    assert_eq!(
        sym_skipped,
        DaySymbolDisplay {
            text: "—".to_string(),
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

    // Skipped measurable day -> "—" (dash, not "⏸")
    let entry_skipped = HabitEntry::new(2, date, EntryStatus::Skipped, 0.0);
    let sym_skipped = format_day_symbol(&habit, Some(&entry_skipped), date);
    assert_eq!(
        sym_skipped,
        DaySymbolDisplay {
            text: "—".to_string(),
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

    // Day button width is 42px and obsolete progress bar rules are absent
    assert!(css.contains("min-width: 42px;"));
    assert!(!css.contains(".habit-progress"));

    // CSS syntax purity for GTK4 (no !important, no width/height properties)
    assert!(
        !css.contains("!important"),
        "GTK4 CSS parser rejects !important as junk at end of value"
    );
    assert!(
        !css.contains(" height:"),
        "GTK4 CSS has no 'height' property; must use min-height"
    );
    assert!(
        !css.contains(" width:"),
        "GTK4 CSS has no 'width' property; must use min-width"
    );

    // Test normalization when passed without leading '#'
    let css_no_hash = generate_row_css(10, "FF7800");
    assert!(css_no_hash.contains(".habit-row-10"));
    assert!(css_no_hash.contains("#FF7800"));
}

#[cfg(feature = "ui")]
#[test]
fn test_css_parsing_cleanliness() {
    match gtk4::init() {
        Ok(()) => {
            println!("GTK initialized successfully!");
            let provider = gtk4::CssProvider::new();
            let errors = Rc::new(RefCell::new(Vec::new()));
            let errors_clone = Rc::clone(&errors);
            provider.connect_parsing_error(move |_, _section, error| {
                errors_clone.borrow_mut().push(format!("{error}"));
            });
            let css = generate_row_css(1, "#3584E4");
            provider.load_from_data(&css);
            println!("Errors captured for row CSS: {:?}", errors.borrow());
            assert!(
                errors.borrow().is_empty(),
                "Row CSS had parser errors: {:?}",
                errors.borrow()
            );

            // Also verify the bottom-sheet dimming & seamless docking CSS
            let sheet_css = ".editor-dimming-backdrop {\n\
                  background-color: rgba(0, 0, 0, 0.45);\n\
                }\n\
                .editor-bottom-sheet {\n\
                  background-color: var(--window-bg-color);\n\
                  border-top-left-radius: 16px;\n\
                  border-top-right-radius: 16px;\n\
                  border-bottom-left-radius: 0;\n\
                  border-bottom-right-radius: 0;\n\
                  border: 1px solid rgba(0, 0, 0, 0.12);\n\
                  border-bottom: none;\n\
                  box-shadow: 0 -2px 8px rgba(0, 0, 0, 0.18);\n\
                  margin-left: 20px;\n\
                  margin-right: 20px;\n\
                  margin-bottom: 0;\n\
                }\n\
                button.destructive-action.flat,\n\
                button.destructive-action.flat image {\n\
                  color: #e01b24;\n\
                }\n";
            provider.load_from_data(sheet_css);
            println!("Errors captured for sheet CSS: {:?}", errors.borrow());
            assert!(
                errors.borrow().is_empty(),
                "Sheet CSS had parser errors: {:?}",
                errors.borrow()
            );
        }
        Err(e) => {
            println!("GTK init failed: {e}");
        }
    }
}

#[test]
fn test_three_states_only_and_no_pause_symbol() {
    let habit = Habit::new("Exercise", HabitType::Binary, SCHEDULE_ALL_DAYS, "#3584E4").with_id(1);
    let date = NaiveDate::from_ymd_opt(2026, 10, 27).unwrap();

    // 1. Completed state: (v) -> "✓"
    let completed_entry = HabitEntry::new(1, date, EntryStatus::Completed, 1.0);
    let completed_sym = format_day_symbol(&habit, Some(&completed_entry), date);
    assert_eq!(completed_sym.text, "✓");
    assert_ne!(completed_sym.text, "⏸");

    // 2. Missed state: (X) -> "✕"
    let missed_entry = HabitEntry::new(1, date, EntryStatus::Missed, 0.0);
    let missed_sym = format_day_symbol(&habit, Some(&missed_entry), date);
    assert_eq!(missed_sym.text, "✕");
    assert_ne!(missed_sym.text, "⏸");

    // 3. Skipped state: (-) -> "—"
    let skipped_entry = HabitEntry::new(1, date, EntryStatus::Skipped, 0.0);
    let skipped_sym = format_day_symbol(&habit, Some(&skipped_entry), date);
    assert_eq!(skipped_sym.text, "—");
    assert_ne!(skipped_sym.text, "⏸");

    // Cycle transitions: Missed -> Completed -> Skipped -> Missed
    let next_1 = next_toggle_status(None);
    assert_eq!(next_1, EntryStatus::Completed);
    let next_2 = next_toggle_status(Some(next_1));
    assert_eq!(next_2, EntryStatus::Skipped);
    let next_3 = next_toggle_status(Some(next_2));
    assert_eq!(next_3, EntryStatus::Missed);
    let next_4 = next_toggle_status(Some(next_3));
    assert_eq!(next_4, EntryStatus::Completed);
}

#[test]
fn test_move_habit_in_order() {
    let mut order = vec![10, 20, 30, 40];

    // Move 40 to position of 20 (upwards)
    assert!(habit::ui::habit_row::move_habit_in_order(
        &mut order, 40, 20
    ));
    assert_eq!(order, vec![10, 40, 20, 30]);

    // Move 10 to position of 30 (downwards)
    assert!(habit::ui::habit_row::move_habit_in_order(
        &mut order, 10, 30
    ));
    assert_eq!(order, vec![40, 20, 30, 10]);

    // Dragging onto self is a no-op
    assert!(!habit::ui::habit_row::move_habit_in_order(
        &mut order, 20, 20
    ));
    assert_eq!(order, vec![40, 20, 30, 10]);

    // Unknown ID is a no-op
    assert!(!habit::ui::habit_row::move_habit_in_order(
        &mut order, 99, 20
    ));
    assert_eq!(order, vec![40, 20, 30, 10]);
}

#[test]
fn test_target_index_for_drag_y_midpoint_crossing() {
    // 4 rows of height 46px with 6px spacing: (top, height)
    let spans = vec![
        (0.0, 46.0),   // idx 0: midpoint 23.0
        (52.0, 46.0),  // idx 1: midpoint 75.0
        (104.0, 46.0), // idx 2: midpoint 127.0
        (156.0, 46.0), // idx 3: midpoint 179.0
    ];

    // Dragging row 1 downward: before row 2 midpoint (127.0) stays at 1; past 127.0 moves to 2
    assert_eq!(
        habit::ui::habit_row::target_index_for_drag_y(&spans, 1, 120.0),
        1
    );
    assert_eq!(
        habit::ui::habit_row::target_index_for_drag_y(&spans, 1, 130.0),
        2
    );

    // Dragging row 2 upward: above row 1 midpoint (75.0) moves to 1; above row 0 midpoint (23.0) moves to 0
    assert_eq!(
        habit::ui::habit_row::target_index_for_drag_y(&spans, 2, 70.0),
        1
    );
    assert_eq!(
        habit::ui::habit_row::target_index_for_drag_y(&spans, 2, -15.0),
        0
    );

    // Dragging past bottom edge clamps to last index (3)
    assert_eq!(
        habit::ui::habit_row::target_index_for_drag_y(&spans, 0, 500.0),
        3
    );
}
