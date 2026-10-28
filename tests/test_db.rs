use chrono::NaiveDate;
use habit::engine::db::{Database, DbError};
use habit::engine::models::{EntryStatus, Habit, HabitType, SCHEDULE_MON, SCHEDULE_WED};

#[test]
fn test_in_memory_creation_and_habit_crud() {
    let db = Database::open_in_memory().expect("failed to open in-memory db");

    // 1. Create binary habit
    let habit = Habit::new_binary("Morning Meditation").with_color("#33D17A");
    let id1 = db.create_habit(&habit).expect("failed to create habit");
    assert!(id1 > 0);

    // 2. Create measurable habit
    let measurable = Habit::new_measurable("Read Book", 25.0, "pages")
        .with_schedule(SCHEDULE_MON | SCHEDULE_WED);
    let id2 = db
        .create_habit(&measurable)
        .expect("failed to create measurable habit");
    assert!(id2 > id1);

    // 3. Get single habit
    let fetched1 = db
        .get_habit(id1)
        .expect("get_habit failed")
        .expect("habit not found");
    assert_eq!(fetched1.id, Some(id1));
    assert_eq!(fetched1.name, "Morning Meditation");
    assert_eq!(fetched1.habit_type, HabitType::Binary);
    assert_eq!(fetched1.accent_color, "#33D17A");
    assert!(!fetched1.archived);

    let fetched2 = db
        .get_habit(id2)
        .expect("get_habit failed")
        .expect("habit not found");
    assert_eq!(fetched2.id, Some(id2));
    assert_eq!(fetched2.name, "Read Book");
    assert_eq!(
        fetched2.habit_type,
        HabitType::Measurable {
            target: 25.0,
            unit: "pages".to_string()
        }
    );
    assert_eq!(fetched2.scheduled_days, SCHEDULE_MON | SCHEDULE_WED);

    // Non-existent habit
    let fetched_none = db.get_habit(9999).expect("query should succeed");
    assert!(fetched_none.is_none());

    // 4. List habits
    let habits = db.get_habits().expect("get_habits failed");
    assert_eq!(habits.len(), 2);
    assert_eq!(habits[0].id, Some(id1));
    assert_eq!(habits[1].id, Some(id2));

    // 5. Update habit
    let mut updated_habit = fetched1;
    updated_habit.name = "Evening Meditation".to_string();
    updated_habit.archived = true;
    db.update_habit(&updated_habit).expect("update failed");

    let re_fetched = db.get_habit(id1).expect("get_habit failed").unwrap();
    assert_eq!(re_fetched.name, "Evening Meditation");
    assert!(re_fetched.archived);

    // 6. Delete habit
    db.delete_habit(id1).expect("delete failed");
    let deleted = db.get_habit(id1).expect("get_habit failed");
    assert!(deleted.is_none());

    let remaining = db.get_habits().expect("get_habits failed");
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].id, Some(id2));
}

#[test]
fn test_atomic_entry_logging_and_queries() {
    let db = Database::open_in_memory().expect("failed to open in-memory db");
    let habit = Habit::test_fixture();
    let id = db.create_habit(&habit).expect("failed to create habit");

    let day1 = NaiveDate::from_ymd_opt(2026, 10, 24).unwrap();
    let day2 = NaiveDate::from_ymd_opt(2026, 10, 25).unwrap();
    let day3 = NaiveDate::from_ymd_opt(2026, 10, 26).unwrap();

    // Log entries
    db.log_entry(id, day1, EntryStatus::Completed, 1.0)
        .expect("log entry failed");
    db.log_entry(id, day2, EntryStatus::Skipped, 0.0)
        .expect("log entry failed");
    db.log_entry(id, day3, EntryStatus::Missed, 0.0)
        .expect("log entry failed");

    // Single entry query
    let entry1 = db
        .get_entry(id, day1)
        .expect("query failed")
        .expect("entry missing");
    assert_eq!(entry1.habit_id, id);
    assert_eq!(entry1.date, day1);
    assert_eq!(entry1.status, EntryStatus::Completed);
    assert!((entry1.value - 1.0).abs() < f64::EPSILON);

    let entry_none = db
        .get_entry(id, NaiveDate::from_ymd_opt(2026, 10, 20).unwrap())
        .expect("query failed");
    assert!(entry_none.is_none());

    // Replace entry (INSERT OR REPLACE)
    db.log_entry(id, day1, EntryStatus::Skipped, 0.5)
        .expect("replace entry failed");
    let entry1_updated = db.get_entry(id, day1).expect("query failed").unwrap();
    assert_eq!(entry1_updated.status, EntryStatus::Skipped);
    assert!((entry1_updated.value - 0.5).abs() < f64::EPSILON);

    // Range queries
    let range_all = db.get_entries(id, day1, day3).expect("range query failed");
    assert_eq!(range_all.len(), 3);
    assert_eq!(range_all[0].date, day1);
    assert_eq!(range_all[1].date, day2);
    assert_eq!(range_all[2].date, day3);

    let range_subset = db.get_entries(id, day2, day3).expect("range query failed");
    assert_eq!(range_subset.len(), 2);
    assert_eq!(range_subset[0].date, day2);
    assert_eq!(range_subset[1].date, day3);

    // Empty range
    let empty_range = db.get_entries(id, day3, day1).expect("range query failed");
    assert_eq!(empty_range.len(), 0);

    // Delete entry (unrecorded)
    db.delete_entry(id, day2).expect("delete_entry failed");
    let deleted_entry = db.get_entry(id, day2).expect("get_entry failed");
    assert!(deleted_entry.is_none());
    let remaining = db.get_entries(id, day1, day3).expect("range query failed");
    assert_eq!(remaining.len(), 2);
    assert_eq!(remaining[0].date, day1);
    assert_eq!(remaining[1].date, day3);
}

#[test]
fn test_foreign_key_constraints_and_cascade() {
    let db = Database::open_in_memory().expect("failed to open in-memory db");
    let habit = Habit::test_fixture();
    let id = db.create_habit(&habit).expect("failed to create habit");

    let today = NaiveDate::from_ymd_opt(2026, 10, 26).unwrap();
    db.log_entry(id, today, EntryStatus::Completed, 1.0)
        .unwrap();

    // Verify foreign key violation on non-existent habit
    let bad_id = 999_999;
    let fk_err = db.log_entry(bad_id, today, EntryStatus::Completed, 1.0);
    assert!(
        fk_err.is_err(),
        "logging entry for missing habit should fail FK check"
    );

    // Verify cascade deletion
    db.delete_habit(id).expect("delete habit failed");
    let entries = db.get_entries(id, today, today).expect("query failed");
    assert!(entries.is_empty(), "cascade should delete child entries");
}

#[test]
fn test_on_disk_wal_mode_and_persistence() {
    let temp_dir = std::env::temp_dir().join(format!("habit_test_{}", std::process::id()));
    std::fs::create_dir_all(&temp_dir).unwrap();
    let db_path = temp_dir.join("test.db");

    {
        let db = Database::open(&db_path).expect("open on-disk db failed");
        let id = db.create_habit(&Habit::new_binary("Drink Water")).unwrap();
        let today = NaiveDate::from_ymd_opt(2026, 10, 26).unwrap();
        db.log_entry(id, today, EntryStatus::Completed, 1.0)
            .unwrap();
    }

    // Reopen database from same file
    {
        let db = Database::open(&db_path).expect("reopen on-disk db failed");
        let habits = db.get_habits().expect("get habits failed");
        assert_eq!(habits.len(), 1);
        assert_eq!(habits[0].name, "Drink Water");

        let today = NaiveDate::from_ymd_opt(2026, 10, 26).unwrap();
        let entries = db.get_entries(habits[0].id.unwrap(), today, today).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].status, EntryStatus::Completed);
    }

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_backup_export_and_restore() {
    let temp_dir = std::env::temp_dir().join(format!("habit_backup_test_{}", std::process::id()));
    std::fs::create_dir_all(&temp_dir).unwrap();
    let backup_path = temp_dir.join("backup.db");

    let db1 = Database::open_in_memory().expect("open db1 failed");
    let habit1 = Habit::new_binary("Read Book");
    let habit2 = Habit::new_measurable("Pushups", 50.0, "reps");
    let id1 = db1.create_habit(&habit1).unwrap();
    let id2 = db1.create_habit(&habit2).unwrap();

    let day = NaiveDate::from_ymd_opt(2026, 10, 26).unwrap();
    db1.log_entry(id1, day, EntryStatus::Completed, 1.0)
        .unwrap();
    db1.log_entry(id2, day, EntryStatus::Completed, 50.0)
        .unwrap();

    // Export atomic backup
    db1.export_backup(&backup_path)
        .expect("export backup failed");
    assert!(backup_path.exists());

    // Restore into a fresh in-memory database
    let db2 = Database::open_in_memory().expect("open db2 failed");
    assert_eq!(db2.get_habits().unwrap().len(), 0);

    db2.restore_backup(&backup_path)
        .expect("restore backup failed");

    let restored_habits = db2.get_habits().unwrap();
    assert_eq!(restored_habits.len(), 2);
    assert_eq!(restored_habits[0].name, "Read Book");
    assert_eq!(restored_habits[1].name, "Pushups");

    let restored_entries1 = db2.get_entries(id1, day, day).unwrap();
    assert_eq!(restored_entries1.len(), 1);
    assert_eq!(restored_entries1[0].status, EntryStatus::Completed);

    let restored_entries2 = db2.get_entries(id2, day, day).unwrap();
    assert_eq!(restored_entries2.len(), 1);
    assert!((restored_entries2[0].value - 50.0).abs() < f64::EPSILON);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_restore_validation_failures() {
    let temp_dir = std::env::temp_dir().join(format!("habit_corrupt_test_{}", std::process::id()));
    std::fs::create_dir_all(&temp_dir).unwrap();
    let invalid_path = temp_dir.join("garbage.db");
    std::fs::write(&invalid_path, b"not a sqlite database file at all").unwrap();

    let db = Database::open_in_memory().unwrap();
    let res = db.restore_backup(&invalid_path);
    assert!(res.is_err(), "restoring non-sqlite file must fail");

    // Test database with missing required tables
    let empty_db_path = temp_dir.join("empty.db");
    {
        let conn = rusqlite::Connection::open(&empty_db_path).unwrap();
        conn.execute("CREATE TABLE foo (id INT);", []).unwrap();
    }
    let res_empty = db.restore_backup(&empty_db_path);
    match res_empty {
        Err(DbError::InvalidBackupSchema(_)) => {}
        other => panic!("expected DbError::InvalidBackupSchema, got {other:?}"),
    }

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_error_handling() {
    let db = Database::open_in_memory().unwrap();

    // Update with no ID
    let unpersisted = Habit::new_binary("Unsaved");
    match db.update_habit(&unpersisted) {
        Err(DbError::MissingId) => {}
        other => panic!("expected DbError::MissingId, got {other:?}"),
    }

    // Update non-existent ID
    let ghost = Habit::new_binary("Ghost").with_id(404);
    match db.update_habit(&ghost) {
        Err(DbError::HabitNotFound(404)) => {}
        other => panic!("expected DbError::HabitNotFound(404), got {other:?}"),
    }

    // Delete non-existent ID
    match db.delete_habit(404) {
        Err(DbError::HabitNotFound(404)) => {}
        other => panic!("expected DbError::HabitNotFound(404), got {other:?}"),
    }
}

#[test]
fn test_reorder_habits_and_sort_order() {
    let db = Database::open_in_memory().expect("in-memory db");
    let h1 = db.create_habit(&Habit::new_binary("First")).unwrap();
    let h2 = db.create_habit(&Habit::new_binary("Second")).unwrap();
    let h3 = db.create_habit(&Habit::new_binary("Third")).unwrap();

    let initial: Vec<i64> = db
        .get_habits()
        .unwrap()
        .into_iter()
        .filter_map(|h| h.id)
        .collect();
    assert_eq!(initial, vec![h1, h2, h3]);

    db.reorder_habits(&[h3, h1, h2]).unwrap();
    let reordered: Vec<i64> = db
        .get_habits()
        .unwrap()
        .into_iter()
        .filter_map(|h| h.id)
        .collect();
    assert_eq!(reordered, vec![h3, h1, h2]);
}
