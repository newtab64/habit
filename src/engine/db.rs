use std::fmt;
use std::path::Path;

use chrono::NaiveDate;
use rusqlite::Connection;

use crate::engine::models::{EntryStatus, Habit, HabitEntry, HabitType};

/// Errors originating from database operations and schema integrity checks.
#[derive(Debug)]
pub enum DbError {
    /// SQLite driver error.
    Sqlite(rusqlite::Error),
    /// Date parsing error.
    DateParse(chrono::ParseError),
    /// Encountered an unexpected habit type string in the database.
    InvalidHabitType(String),
    /// Encountered an unexpected entry status string in the database.
    InvalidEntryStatus(String),
    /// Scheduled days bitmask in database exceeded `u8` bounds.
    InvalidScheduledDays(i64),
    /// Habit ID was required for update but was missing.
    MissingId,
    /// Habit with the specified ID was not found.
    HabitNotFound(i64),
    /// Database file failed SQLite integrity check during restore.
    IntegrityCheckFailed(String),
    /// Backup file is missing required tables.
    InvalidBackupSchema(String),
}

impl fmt::Display for DbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sqlite(err) => write!(f, "database error: {err}"),
            Self::DateParse(err) => write!(f, "date parse error: {err}"),
            Self::InvalidHabitType(s) => write!(f, "invalid habit type in database: '{s}'"),
            Self::InvalidEntryStatus(s) => write!(f, "invalid entry status in database: '{s}'"),
            Self::InvalidScheduledDays(n) => {
                write!(f, "invalid scheduled days bitmask in database: {n}")
            }
            Self::MissingId => write!(f, "habit has no id assigned"),
            Self::HabitNotFound(id) => write!(f, "habit with id {id} not found"),
            Self::IntegrityCheckFailed(msg) => {
                write!(f, "sqlite pragma integrity check failed: {msg}")
            }
            Self::InvalidBackupSchema(msg) => write!(f, "invalid backup schema: {msg}"),
        }
    }
}

impl std::error::Error for DbError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Sqlite(err) => Some(err),
            Self::DateParse(err) => Some(err),
            _ => None,
        }
    }
}

impl From<rusqlite::Error> for DbError {
    fn from(err: rusqlite::Error) -> Self {
        Self::Sqlite(err)
    }
}

impl From<chrono::ParseError> for DbError {
    fn from(err: chrono::ParseError) -> Self {
        Self::DateParse(err)
    }
}

/// SQLite persistence manager for habits and daily completion entries.
#[derive(Debug)]
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Creates and initializes a fresh in-memory SQLite database.
    ///
    /// # Errors
    /// Returns [`DbError`] if SQLite in-memory initialization fails.
    pub fn open_in_memory() -> Result<Self, DbError> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        Self::init_tables(&conn)?;
        Ok(Self { conn })
    }

    /// Opens or creates an on-disk SQLite database at `path` with WAL mode enabled.
    ///
    /// # Errors
    /// Returns [`DbError`] if opening the database or configuring WAL/foreign keys fails.
    pub fn open(path: &Path) -> Result<Self, DbError> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON;")?;
        Self::init_tables(&conn)?;
        Ok(Self { conn })
    }

    fn init_tables(conn: &Connection) -> Result<(), DbError> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS habits (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                habit_type TEXT NOT NULL CHECK(habit_type IN ('binary', 'measurable')),
                target_value REAL NOT NULL DEFAULT 1.0,
                unit TEXT,
                accent_color TEXT NOT NULL DEFAULT '#3584E4',
                scheduled_days INTEGER NOT NULL DEFAULT 127,
                archived INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS habit_entries (
                habit_id INTEGER NOT NULL REFERENCES habits(id) ON DELETE CASCADE,
                date TEXT NOT NULL,
                entry_status TEXT NOT NULL CHECK(entry_status IN ('completed', 'skipped', 'missed')),
                value REAL NOT NULL DEFAULT 0.0,
                PRIMARY KEY (habit_id, date)
            );

            CREATE INDEX IF NOT EXISTS idx_entries_habit_date ON habit_entries(habit_id, date);",
        )?;
        Ok(())
    }

    /// Inserts a new habit into the database, returning its assigned SQLite row ID.
    ///
    /// # Errors
    /// Returns [`DbError`] if insertion fails.
    pub fn create_habit(&self, habit: &Habit) -> Result<i64, DbError> {
        let (habit_type_str, target_value, unit) = match &habit.habit_type {
            HabitType::Binary => ("binary", 1.0, None),
            HabitType::Measurable { target, unit } => ("measurable", *target, Some(unit.as_str())),
        };

        let created_at = if habit.created_at != 0 {
            habit.created_at
        } else {
            chrono::Utc::now().timestamp()
        };

        let archived_int: i64 = i64::from(habit.archived);

        self.conn.execute(
            "INSERT INTO habits (name, habit_type, target_value, unit, accent_color, scheduled_days, archived, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8);",
            rusqlite::params![
                habit.name,
                habit_type_str,
                target_value,
                unit,
                habit.accent_color,
                i64::from(habit.scheduled_days),
                archived_int,
                created_at,
            ],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Retrieves all habits in the database, ordered by ID ascending.
    ///
    /// # Errors
    /// Returns [`DbError`] if querying or row deserialization fails.
    pub fn get_habits(&self) -> Result<Vec<Habit>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, habit_type, target_value, unit, accent_color, scheduled_days, archived, created_at
             FROM habits ORDER BY id ASC;",
        )?;
        let mut rows = stmt.query([])?;
        let mut habits = Vec::new();
        while let Some(row) = rows.next()? {
            habits.push(Self::row_to_habit(row)?);
        }
        Ok(habits)
    }

    /// Retrieves a single habit by its primary key ID.
    ///
    /// # Errors
    /// Returns [`DbError`] if querying fails.
    pub fn get_habit(&self, id: i64) -> Result<Option<Habit>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, habit_type, target_value, unit, accent_color, scheduled_days, archived, created_at
             FROM habits WHERE id = ?1;",
        )?;
        let mut rows = stmt.query(rusqlite::params![id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Self::row_to_habit(row)?))
        } else {
            Ok(None)
        }
    }

    /// Updates an existing habit in the database.
    ///
    /// # Errors
    /// Returns [`DbError::MissingId`] if `habit.id` is `None`, [`DbError::HabitNotFound`]
    /// if no row matched `habit.id`, or [`DbError::Sqlite`] on driver error.
    pub fn update_habit(&self, habit: &Habit) -> Result<(), DbError> {
        let id = habit.id.ok_or(DbError::MissingId)?;
        let (habit_type_str, target_value, unit) = match &habit.habit_type {
            HabitType::Binary => ("binary", 1.0, None),
            HabitType::Measurable { target, unit } => ("measurable", *target, Some(unit.as_str())),
        };

        let archived_int: i64 = i64::from(habit.archived);

        let affected = self.conn.execute(
            "UPDATE habits SET
                name = ?1,
                habit_type = ?2,
                target_value = ?3,
                unit = ?4,
                accent_color = ?5,
                scheduled_days = ?6,
                archived = ?7
             WHERE id = ?8;",
            rusqlite::params![
                habit.name,
                habit_type_str,
                target_value,
                unit,
                habit.accent_color,
                i64::from(habit.scheduled_days),
                archived_int,
                id,
            ],
        )?;

        if affected == 0 {
            return Err(DbError::HabitNotFound(id));
        }
        Ok(())
    }

    /// Deletes a habit by ID. Child entries in `habit_entries` are deleted via foreign key cascade.
    ///
    /// # Errors
    /// Returns [`DbError::HabitNotFound`] if the habit ID does not exist, or [`DbError::Sqlite`].
    pub fn delete_habit(&self, id: i64) -> Result<(), DbError> {
        let affected = self
            .conn
            .execute("DELETE FROM habits WHERE id = ?1;", rusqlite::params![id])?;
        if affected == 0 {
            return Err(DbError::HabitNotFound(id));
        }
        Ok(())
    }

    /// Logs or replaces an entry for the given habit and date.
    ///
    /// # Errors
    /// Returns [`DbError`] if habit foreign key constraint fails or write fails.
    pub fn log_entry(
        &self,
        habit_id: i64,
        date: NaiveDate,
        status: EntryStatus,
        value: f64,
    ) -> Result<(), DbError> {
        let date_str = date.format("%Y-%m-%d").to_string();
        self.conn.execute(
            "INSERT OR REPLACE INTO habit_entries (habit_id, date, entry_status, value)
             VALUES (?1, ?2, ?3, ?4);",
            rusqlite::params![habit_id, date_str, status.as_str(), value],
        )?;
        Ok(())
    }

    /// Retrieves entries for a habit within the inclusive date range `[from, to]`.
    ///
    /// # Errors
    /// Returns [`DbError`] if query execution or date parsing fails.
    pub fn get_entries(
        &self,
        habit_id: i64,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<HabitEntry>, DbError> {
        let from_str = from.format("%Y-%m-%d").to_string();
        let to_str = to.format("%Y-%m-%d").to_string();
        let mut stmt = self.conn.prepare(
            "SELECT habit_id, date, entry_status, value
             FROM habit_entries
             WHERE habit_id = ?1 AND date >= ?2 AND date <= ?3
             ORDER BY date ASC;",
        )?;
        let mut rows = stmt.query(rusqlite::params![habit_id, from_str, to_str])?;
        let mut entries = Vec::new();
        while let Some(row) = rows.next()? {
            entries.push(Self::row_to_entry(row)?);
        }
        Ok(entries)
    }

    /// Retrieves an entry for a specific habit on a specific date, if one exists.
    ///
    /// # Errors
    /// Returns [`DbError`] if querying or row deserialization fails.
    pub fn get_entry(&self, habit_id: i64, date: NaiveDate) -> Result<Option<HabitEntry>, DbError> {
        let date_str = date.format("%Y-%m-%d").to_string();
        let mut stmt = self.conn.prepare(
            "SELECT habit_id, date, entry_status, value
             FROM habit_entries
             WHERE habit_id = ?1 AND date = ?2;",
        )?;
        let mut rows = stmt.query(rusqlite::params![habit_id, date_str])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Self::row_to_entry(row)?))
        } else {
            Ok(None)
        }
    }

    /// Creates an atomic snapshot backup of the current database to `target_path`.
    ///
    /// # Errors
    /// Returns [`DbError`] if opening target or running backup fails.
    pub fn export_backup(&self, target_path: &Path) -> Result<(), DbError> {
        if target_path.exists() {
            let _ = std::fs::remove_file(target_path);
            let mut wal_path = target_path.to_path_buf().into_os_string();
            wal_path.push("-wal");
            let _ = std::fs::remove_file(wal_path);
            let mut shm_path = target_path.to_path_buf().into_os_string();
            shm_path.push("-shm");
            let _ = std::fs::remove_file(shm_path);
        }
        let mut dst_conn = Connection::open(target_path)?;
        let backup = rusqlite::backup::Backup::new(&self.conn, &mut dst_conn)?;
        backup.run_to_completion(100, std::time::Duration::from_millis(0), None)?;
        Ok(())
    }

    /// Restores the current database from `source_path` after verifying integrity and schema.
    ///
    /// # Errors
    /// Returns [`DbError::IntegrityCheckFailed`] if SQLite integrity check fails,
    /// [`DbError::InvalidBackupSchema`] if the file is missing or required tables are absent,
    /// or [`DbError::Sqlite`] on restore failure.
    pub fn restore_backup(&mut self, source_path: &Path) -> Result<(), DbError> {
        if !source_path.is_file() {
            return Err(DbError::InvalidBackupSchema(format!(
                "Backup path does not exist or is not a regular file: {}",
                source_path.display()
            )));
        }
        let src_conn =
            Connection::open_with_flags(source_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;

        let integrity: String =
            src_conn.query_row("PRAGMA integrity_check;", [], |row| row.get(0))?;
        if integrity != "ok" {
            return Err(DbError::IntegrityCheckFailed(integrity));
        }

        let table_count: i64 = src_conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('habits', 'habit_entries');",
            [],
            |row| row.get(0),
        )?;
        if table_count < 2 {
            return Err(DbError::InvalidBackupSchema(
                "Backup database is missing required tables ('habits', 'habit_entries')"
                    .to_string(),
            ));
        }

        {
            let backup = rusqlite::backup::Backup::new(&src_conn, &mut self.conn)?;
            backup.run_to_completion(100, std::time::Duration::from_millis(0), None)?;
        }

        self.conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        Ok(())
    }

    fn row_to_habit(row: &rusqlite::Row<'_>) -> Result<Habit, DbError> {
        let id: i64 = row.get(0)?;
        let name: String = row.get(1)?;
        let habit_type_str: String = row.get(2)?;
        let target_value: f64 = row.get(3)?;
        let unit: Option<String> = row.get(4)?;
        let accent_color: String = row.get(5)?;
        let scheduled_days_i64: i64 = row.get(6)?;
        let archived_int: i64 = row.get(7)?;
        let created_at: i64 = row.get(8)?;

        let habit_type = match habit_type_str.as_str() {
            "binary" => HabitType::Binary,
            "measurable" => HabitType::Measurable {
                target: target_value,
                unit: unit.unwrap_or_default(),
            },
            _ => return Err(DbError::InvalidHabitType(habit_type_str)),
        };

        let scheduled_days = u8::try_from(scheduled_days_i64)
            .map_err(|_| DbError::InvalidScheduledDays(scheduled_days_i64))?;

        Ok(Habit {
            id: Some(id),
            name,
            habit_type,
            accent_color,
            scheduled_days,
            archived: archived_int != 0,
            created_at,
        })
    }

    fn row_to_entry(row: &rusqlite::Row<'_>) -> Result<HabitEntry, DbError> {
        let habit_id: i64 = row.get(0)?;
        let date_str: String = row.get(1)?;
        let status_str: String = row.get(2)?;
        let value: f64 = row.get(3)?;

        let date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")?;
        let status = EntryStatus::parse_status(&status_str)
            .ok_or_else(|| DbError::InvalidEntryStatus(status_str))?;

        Ok(HabitEntry {
            habit_id,
            date,
            status,
            value,
        })
    }
}
