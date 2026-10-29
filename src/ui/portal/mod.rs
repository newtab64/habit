use chrono::NaiveDate;

#[cfg(feature = "ui")]
mod dialogs;
#[cfg(feature = "ui")]
pub use dialogs::{trigger_export_dialog, trigger_import_dialog, DialogOutcome};

/// Generates the default backup filename based on the specified date.
///
/// Format: `habit-backup-YYYY-MM-DD.db` (e.g. `habit-backup-2026-10-26.db`).
#[must_use]
pub fn default_backup_filename(date: NaiveDate) -> String {
    format!("habit-backup-{}.db", date.format("%Y-%m-%d"))
}
