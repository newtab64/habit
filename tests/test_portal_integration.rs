use chrono::NaiveDate;
use habit::engine::db::{Database, DbError};
use habit::engine::models::{EntryStatus, Habit};
use habit::ui::default_backup_filename;

#[test]
fn test_default_backup_filename() {
    let date = NaiveDate::from_ymd_opt(2026, 10, 26).unwrap();
    assert_eq!(default_backup_filename(date), "habit-backup-2026-10-26.db");

    let new_year = NaiveDate::from_ymd_opt(2027, 1, 1).unwrap();
    assert_eq!(
        default_backup_filename(new_year),
        "habit-backup-2027-01-01.db"
    );
}

#[test]
fn test_backup_export_and_atomic_restore() {
    let temp_dir = std::env::temp_dir().join(format!("portal_test_export_{}", std::process::id()));
    std::fs::create_dir_all(&temp_dir).unwrap();
    let backup_path = temp_dir.join("export.db");

    let db_src = Database::open_in_memory().expect("open source db");
    let habit1 = Habit::new_binary("Morning Meditation");
    let habit2 = Habit::new_measurable("Read Pages", 20.0, "pages");
    let id1 = db_src.create_habit(&habit1).unwrap();
    let id2 = db_src.create_habit(&habit2).unwrap();

    let today = NaiveDate::from_ymd_opt(2026, 10, 26).unwrap();
    db_src
        .log_entry(id1, today, EntryStatus::Completed, 1.0)
        .unwrap();
    db_src
        .log_entry(id2, today, EntryStatus::Completed, 25.0)
        .unwrap();

    // Export backup to disk
    db_src
        .export_backup(&backup_path)
        .expect("export backup should succeed");
    assert!(backup_path.exists());

    // Restore into empty target DB
    let db_dst = Database::open_in_memory().expect("open target db");
    assert_eq!(db_dst.get_habits().unwrap().len(), 0);

    db_dst
        .restore_backup(&backup_path)
        .expect("restore backup should succeed");

    let restored = db_dst.get_habits().unwrap();
    assert_eq!(restored.len(), 2);
    assert_eq!(restored[0].name, "Morning Meditation");
    assert_eq!(restored[1].name, "Read Pages");

    let entries1 = db_dst.get_entries(id1, today, today).unwrap();
    assert_eq!(entries1.len(), 1);
    assert_eq!(entries1[0].status, EntryStatus::Completed);

    let entries2 = db_dst.get_entries(id2, today, today).unwrap();
    assert_eq!(entries2.len(), 1);
    assert!((entries2[0].value - 25.0).abs() < f64::EPSILON);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_restore_rejects_corrupted_file() {
    let temp_dir = std::env::temp_dir().join(format!("portal_test_corrupt_{}", std::process::id()));
    std::fs::create_dir_all(&temp_dir).unwrap();
    let corrupt_file = temp_dir.join("corrupted.db");
    std::fs::write(&corrupt_file, b"this is not a valid sqlite database file").unwrap();

    let db = Database::open_in_memory().expect("open in-memory db");
    let res = db.restore_backup(&corrupt_file);
    assert!(res.is_err(), "restoring corrupt file must fail");

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_restore_rejects_missing_tables() {
    let temp_dir = std::env::temp_dir().join(format!("portal_test_schema_{}", std::process::id()));
    std::fs::create_dir_all(&temp_dir).unwrap();
    let invalid_schema_file = temp_dir.join("missing_tables.db");

    {
        let conn = rusqlite::Connection::open(&invalid_schema_file).unwrap();
        conn.execute("CREATE TABLE dummy (id INTEGER PRIMARY KEY);", [])
            .unwrap();
    }

    let db = Database::open_in_memory().expect("open in-memory db");
    let res = db.restore_backup(&invalid_schema_file);
    match res {
        Err(DbError::InvalidBackupSchema(msg)) => {
            assert!(msg.contains("missing required tables"));
        }
        other => panic!("expected DbError::InvalidBackupSchema, got {other:?}"),
    }

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_restore_rejects_nonexistent_file() {
    let nonexistent_file = std::path::PathBuf::from("/nonexistent/path/habit_backup.db");
    let db = Database::open_in_memory().expect("open in-memory db");
    let res = db.restore_backup(&nonexistent_file);
    match res {
        Err(DbError::InvalidBackupSchema(msg)) => {
            assert!(msg.contains("does not exist or is not a regular file"));
        }
        other => panic!("expected DbError::InvalidBackupSchema, got {other:?}"),
    }
}

#[test]
fn test_backup_export_overwrites_existing_file() {
    let temp_dir =
        std::env::temp_dir().join(format!("portal_test_overwrite_{}", std::process::id()));
    std::fs::create_dir_all(&temp_dir).unwrap();
    let backup_path = temp_dir.join("existing.db");

    // Write dummy file at destination
    std::fs::write(&backup_path, b"initial old file contents").unwrap();

    let db_src = Database::open_in_memory().expect("open source db");
    let habit = Habit::new_binary("Journaling");
    db_src.create_habit(&habit).unwrap();

    // Export over existing file
    db_src
        .export_backup(&backup_path)
        .expect("export backup should succeed over existing file");
    assert!(backup_path.exists());

    let db_dst = Database::open_in_memory().expect("open dst db");
    db_dst
        .restore_backup(&backup_path)
        .expect("restore from overwritten file should succeed");
    let habits = db_dst.get_habits().unwrap();
    assert_eq!(habits.len(), 1);
    assert_eq!(habits[0].name, "Journaling");

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_database_shared_restore_without_refcell() {
    let temp_dir =
        std::env::temp_dir().join(format!("portal_test_shared_{}", std::process::id()));
    std::fs::create_dir_all(&temp_dir).unwrap();
    let backup_path = temp_dir.join("backup.db");

    let db_src = Database::open_in_memory().expect("open source db");
    let habit = Habit::new_binary("Morning Routine");
    db_src.create_habit(&habit).unwrap();
    db_src.export_backup(&backup_path).unwrap();

    let db = std::rc::Rc::new(Database::open_in_memory().expect("open target db"));

    // Can call restore_backup on &Database (via Rc<Database> directly) without RefCell or &mut self!
    let restore_res = db.restore_backup(&backup_path);
    assert!(restore_res.is_ok());

    let habits = db.get_habits().unwrap();
    assert_eq!(habits.len(), 1);
    assert_eq!(habits[0].name, "Morning Routine");

    let _ = std::fs::remove_dir_all(&temp_dir);
}
