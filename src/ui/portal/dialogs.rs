use std::path::PathBuf;
use std::rc::Rc;

use gtk4::prelude::*;
use libadwaita::prelude::*;

use crate::engine::db::Database;

use super::default_backup_filename;

/// Outcome of a database import or export dialog operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportOutcome {
    /// Operation completed successfully at the selected path.
    Done(PathBuf),
    /// The user dismissed the file chooser or cancelled the confirmation dialog.
    Cancelled,
    /// Selecting, exporting, or restoring the backup failed with an error message.
    Failed(String),
}

fn is_dialog_cancelled(err: &gtk4::glib::Error) -> bool {
    err.matches(gtk4::DialogError::Dismissed)
        || err.matches(gtk4::DialogError::Cancelled)
        || err.matches(gtk4::gio::IOErrorEnum::Cancelled)
}

fn sqlite_file_filters() -> (gtk4::FileFilter, gtk4::gio::ListStore) {
    let filter = gtk4::FileFilter::new();
    filter.set_name(Some("SQLite Database (*.db)"));
    filter.add_pattern("*.db");

    let filters = gtk4::gio::ListStore::new::<gtk4::FileFilter>();
    filters.append(&filter);
    (filter, filters)
}

/// Prompts the user via XDG `FileDialog` to select a destination path, then exports
/// an atomic SQLite backup of the database to that path.
pub fn trigger_export_dialog<W: IsA<gtk4::Window>>(
    parent: &W,
    db: Rc<Database>,
    on_complete: impl Fn(ImportOutcome) + 'static,
) {
    let (filter, filters) = sqlite_file_filters();
    let today = chrono::Local::now().date_naive();
    let initial_name = default_backup_filename(today);

    let dialog = gtk4::FileDialog::builder()
        .title("Export Database Backup")
        .accept_label("Export")
        .default_filter(&filter)
        .filters(&filters)
        .initial_name(&initial_name)
        .build();

    dialog.save(
        Some(parent),
        None::<&gtk4::gio::Cancellable>,
        move |result| match result {
            Ok(file) => {
                if let Some(path) = file.path() {
                    match db.export_backup(&path) {
                        Ok(()) => on_complete(ImportOutcome::Done(path)),
                        Err(e) => on_complete(ImportOutcome::Failed(e.to_string())),
                    }
                } else {
                    on_complete(ImportOutcome::Failed(
                        "Failed to resolve destination file path".to_string(),
                    ));
                }
            }
            Err(e) => {
                if is_dialog_cancelled(&e) {
                    on_complete(ImportOutcome::Cancelled);
                } else {
                    on_complete(ImportOutcome::Failed(e.to_string()));
                }
            }
        },
    );
}

fn confirm_and_restore<W: IsA<gtk4::Window>>(
    parent: &W,
    path: PathBuf,
    db: &Rc<Database>,
    on_complete: Rc<impl Fn(ImportOutcome) + 'static>,
) {
    let alert = libadwaita::AlertDialog::builder()
        .heading("Restore Database Backup?")
        .body("Restoring a backup will overwrite all current habits and history. This action cannot be undone.")
        .build();
    alert.add_response("cancel", "Cancel");
    alert.add_response("restore", "Restore");
    alert.set_response_appearance("restore", libadwaita::ResponseAppearance::Destructive);
    alert.set_default_response(Some("cancel"));
    alert.set_close_response("cancel");

    let db_clone = Rc::clone(db);
    alert.choose(
        parent.as_ref().upcast_ref::<gtk4::Widget>(),
        None::<&gtk4::gio::Cancellable>,
        move |response| {
            if response == "restore" {
                let restore_res = db_clone.restore_backup(&path);
                gtk4::glib::idle_add_local_once(move || match restore_res {
                    Ok(()) => on_complete(ImportOutcome::Done(path)),
                    Err(e) => on_complete(ImportOutcome::Failed(e.to_string())),
                });
            } else {
                gtk4::glib::idle_add_local_once(move || {
                    on_complete(ImportOutcome::Cancelled);
                });
            }
        },
    );
}

/// Prompts the user via XDG `FileDialog` to select a `.db` backup file, then restores
/// the database from that file after verifying integrity and schema.
pub fn trigger_import_dialog<W: IsA<gtk4::Window>>(
    parent: &W,
    db: Rc<Database>,
    on_complete: impl Fn(ImportOutcome) + 'static,
) {
    let (filter, filters) = sqlite_file_filters();

    let dialog = gtk4::FileDialog::builder()
        .title("Import Database Backup")
        .accept_label("Import")
        .default_filter(&filter)
        .filters(&filters)
        .build();

    let parent_clone = parent.clone();
    let on_complete_rc = Rc::new(on_complete);
    dialog.open(
        Some(parent),
        None::<&gtk4::gio::Cancellable>,
        move |result| match result {
            Ok(file) => {
                if let Some(path) = file.path() {
                    confirm_and_restore(&parent_clone, path, &db, Rc::clone(&on_complete_rc));
                } else {
                    on_complete_rc(ImportOutcome::Failed(
                        "Failed to resolve source file path".to_string(),
                    ));
                }
            }
            Err(e) => {
                if is_dialog_cancelled(&e) {
                    on_complete_rc(ImportOutcome::Cancelled);
                } else {
                    on_complete_rc(ImportOutcome::Failed(e.to_string()));
                }
            }
        },
    );
}
