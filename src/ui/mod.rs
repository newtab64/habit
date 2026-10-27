pub mod charts;
pub mod habit_dialog;
pub mod habit_row;
pub mod metrics;
pub mod portal;
pub mod window;

pub use habit_dialog::{validate_habit_form, HabitValidationError};
pub use habit_row::{
    calculate_7_day_dates, format_day_symbol, format_measurable_value, generate_row_css,
    DaySymbolDisplay, QUICK_CHECK_DAYS,
};
pub use metrics::{
    calculate_habit_metrics, format_percentage_diff, format_schedule_summary,
    format_score_percentage, format_streak_badge, format_strength_percentage,
    format_target_summary, toggle_entry_state, HabitMetrics, MonthNavigation,
};
pub use portal::default_backup_filename;

#[cfg(feature = "ui")]
pub use habit_dialog::show_habit_dialog;
#[cfg(feature = "ui")]
pub use habit_row::HabitRowWidget;
#[cfg(feature = "ui")]
pub use portal::{trigger_export_dialog, trigger_import_dialog};
#[cfg(feature = "ui")]
pub use window::{run_app, HabitWindow};
