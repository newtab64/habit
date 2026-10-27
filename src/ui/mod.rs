pub mod charts;
#[cfg(feature = "ui")]
pub mod details;
pub mod habit_dialog;
pub mod habit_row;
#[cfg(feature = "ui")]
pub mod history_editor;
pub mod metrics;
pub mod portal;
#[cfg(feature = "ui")]
pub mod sidebar;
#[cfg(feature = "ui")]
pub mod state;
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
pub use details::{build_details, DetailsWidgets};
#[cfg(feature = "ui")]
pub use habit_dialog::show_habit_dialog;
#[cfg(feature = "ui")]
pub use habit_row::HabitRowWidget;
#[cfg(feature = "ui")]
pub use history_editor::{build_history_editor, HistoryEditorWidgets};
#[cfg(feature = "ui")]
pub use portal::{trigger_export_dialog, trigger_import_dialog};
#[cfg(feature = "ui")]
pub use sidebar::{build_sidebar, SidebarWidgets};
#[cfg(feature = "ui")]
pub use state::{
    default_database_path, get_database_path, handle_select, handle_toggle, refresh, DrawCaches,
    WindowInner,
};
#[cfg(feature = "ui")]
pub use window::{run_app, HabitWindow};
