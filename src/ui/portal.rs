use chrono::NaiveDate;

#[cfg(feature = "ui")]
use std::cell::RefCell;
#[cfg(feature = "ui")]
use std::path::PathBuf;
#[cfg(feature = "ui")]
use std::rc::Rc;

#[cfg(feature = "ui")]
use gtk4::prelude::*;
#[cfg(feature = "ui")]
use libadwaita::prelude::*;

#[cfg(feature = "ui")]
use crate::engine::db::Database;

/// Generates the default backup filename based on the specified date.
///
/// Format: `habit-backup-YYYY-MM-DD.db` (e.g. `habit-backup-2026-10-26.db`).
#[must_use]
pub fn default_backup_filename(date: NaiveDate) -> String {
    format!("habit-backup-{}.db", date.format("%Y-%m-%d"))
}

/// Prompts the user via XDG `FileDialog` to select a destination path, then exports
/// an atomic SQLite backup of the database to that path.
#[cfg(feature = "ui")]
pub fn trigger_export_dialog<W: IsA<gtk4::Window>>(
    parent: &W,
    db: Rc<RefCell<Database>>,
    on_complete: impl Fn(Result<PathBuf, String>) + 'static,
) {
    let filter = gtk4::FileFilter::new();
    filter.set_name(Some("SQLite Database (*.db)"));
    filter.add_pattern("*.db");

    let filters = gtk4::gio::ListStore::new::<gtk4::FileFilter>();
    filters.append(&filter);

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
                    match db.borrow().export_backup(&path) {
                        Ok(()) => on_complete(Ok(path)),
                        Err(e) => on_complete(Err(e.to_string())),
                    }
                } else {
                    on_complete(Err("Failed to resolve destination file path".to_string()));
                }
            }
            Err(e) => {
                on_complete(Err(e.to_string()));
            }
        },
    );
}

/// Prompts the user via XDG `FileDialog` to select a `.db` backup file, then restores
/// the database from that file after verifying integrity and schema.
#[cfg(feature = "ui")]
pub fn trigger_import_dialog<W: IsA<gtk4::Window>>(
    parent: &W,
    db: Rc<RefCell<Database>>,
    on_complete: impl Fn(Result<PathBuf, String>) + 'static,
) {
    let filter = gtk4::FileFilter::new();
    filter.set_name(Some("SQLite Database (*.db)"));
    filter.add_pattern("*.db");

    let filters = gtk4::gio::ListStore::new::<gtk4::FileFilter>();
    filters.append(&filter);

    let dialog = gtk4::FileDialog::builder()
        .title("Import Database Backup")
        .accept_label("Import")
        .default_filter(&filter)
        .filters(&filters)
        .build();

    let parent_clone = parent.clone();
    dialog.open(
        Some(parent),
        None::<&gtk4::gio::Cancellable>,
        move |result| match result {
            Ok(file) => {
                if let Some(path) = file.path() {
                    let alert = libadwaita::AlertDialog::builder()
                        .heading("Restore Database Backup?")
                        .body("Restoring a backup will overwrite all current habits and history. This action cannot be undone.")
                        .build();
                    alert.add_response("cancel", "Cancel");
                    alert.add_response("restore", "Restore");
                    alert.set_response_appearance("restore", libadwaita::ResponseAppearance::Destructive);
                    alert.set_default_response(Some("cancel"));
                    alert.set_close_response("cancel");

                    let db_clone = Rc::clone(&db);
                    let on_complete_rc = Rc::new(on_complete);
                    let on_complete_cb = Rc::clone(&on_complete_rc);
                    alert.choose(
                        parent_clone.as_ref().upcast_ref::<gtk4::Widget>(),
                        None::<&gtk4::gio::Cancellable>,
                        move |response| {
                            if response == "restore" {
                                let restore_res = db_clone.borrow_mut().restore_backup(&path);
                                gtk4::glib::idle_add_local_once(move || match restore_res {
                                    Ok(()) => on_complete_cb(Ok(path)),
                                    Err(e) => on_complete_cb(Err(e.to_string())),
                                });
                            } else {
                                gtk4::glib::idle_add_local_once(move || {
                                    on_complete_cb(Err("Backup restoration cancelled".to_string()));
                                });
                            }
                        },
                    );
                } else {
                    on_complete(Err("Failed to resolve source file path".to_string()));
                }
            }
            Err(e) => {
                on_complete(Err(e.to_string()));
            }
        },
    );
}
