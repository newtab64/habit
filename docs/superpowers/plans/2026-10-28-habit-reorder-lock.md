# Habit Reorder Lock & Drag Mode Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a top-left header lock button that unlocks drag-and-drop habit reordering (with a leading drag handle per row and disabled day tickboxes), confirms saving or discarding on re-lock, and strictly enforces the new `sort_order` database schema on backup imports.

**Architecture:** `Database` adds `sort_order INTEGER NOT NULL DEFAULT 0` to `habits`, orders `get_habits()` by `sort_order ASC, id ASC`, provides `reorder_habits(&[i64])`, and rejects imported backups missing `sort_order`. The UI tracks `reorder_mode` and `staged_habit_order` in `DrawCaches`, exposes a top-left `lock_btn` in `SidebarWidgets`, attaches GTK4 `DragSource`/`DropTarget` controllers via `HabitRowWidget::enable_reorder_drag` when unlocked, and shows an `adw::AlertDialog` when locking back.

**Tech Stack:** Rust, SQLite (`rusqlite`), GTK4, libadwaita

**Spec:** `docs/superpowers/specs/2026-10-28-habit-reorder-lock-design.md`

## Global Constraints

- Single-threaded GTK4/libadwaita architecture; no new dependencies.
- Strict schema validation: reject any backup database missing the `sort_order` column on `habits`.
- Zero warnings under `cargo clippy --features ui --all-targets -- -D warnings`.
- Never hold a `RefCell` borrow across GTK widget/signal calls.

## Review Focus

- Importing an older `.db` backup without the `sort_order` column: `restore_backup` must return `Err(DbError::InvalidBackupSchema(...))` and leave the active database untouched.
- Dragging a habit row onto itself (`dragged_id == target_id`): `move_habit_in_order` returns `false` without mutating the order or triggering a redundant refresh.
- Calling `refresh()` from inside a GTK `DropTarget::connect_drop` callback: schedule `refresh` via `glib::idle_add_local_once` so GTK finishes the drop lifecycle before widgets are removed/rebuilt.

---

### Task 1: Add `sort_order` Schema, `reorder_habits`, and Strict Backup Schema Validation

**Files:**
- Modify: `src/engine/db.rs:155-235, 445-470`
- Test: `tests/test_db.rs`
- Test: `tests/test_portal_integration.rs`

**Interfaces:**
- Produces:
  - `Database::reorder_habits(&self, ordered_ids: &[i64]) -> Result<(), DbError>`
  - `Database::get_habits(&self) -> Result<Vec<Habit>, DbError>` (ordered by `sort_order ASC, id ASC`)
  - Strict `sort_order` column validation in `Database::restore_backup`

- [ ] **Step 1: Write failing tests in `tests/test_db.rs` and `tests/test_portal_integration.rs`**

In `tests/test_db.rs`:
```rust
#[test]
fn test_reorder_habits_and_sort_order() {
    let db = Database::open_in_memory().expect("in-memory db");
    let h1 = db
        .create_habit(&Habit::new("First", HabitType::Binary, SCHEDULE_ALL_DAYS, "#3584E4"))
        .unwrap();
    let h2 = db
        .create_habit(&Habit::new("Second", HabitType::Binary, SCHEDULE_ALL_DAYS, "#33D17A"))
        .unwrap();
    let h3 = db
        .create_habit(&Habit::new("Third", HabitType::Binary, SCHEDULE_ALL_DAYS, "#FF7800"))
        .unwrap();

    let initial: Vec<i64> = db.get_habits().unwrap().into_iter().filter_map(|h| h.id).collect();
    assert_eq!(initial, vec![h1, h2, h3]);

    db.reorder_habits(&[h3, h1, h2]).unwrap();
    let reordered: Vec<i64> = db.get_habits().unwrap().into_iter().filter_map(|h| h.id).collect();
    assert_eq!(reordered, vec![h3, h1, h2]);
}
```

In `tests/test_portal_integration.rs`:
```rust
#[test]
fn test_restore_rejects_old_schema_without_sort_order() {
    let temp_dir = std::env::temp_dir();
    let old_db_path = temp_dir.join(format!("habit_old_schema_{}.db", std::process::id()));
    let _ = std::fs::remove_file(&old_db_path);

    {
        let conn = rusqlite::Connection::open(&old_db_path).unwrap();
        conn.execute_batch(
            "CREATE TABLE habits (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                habit_type TEXT NOT NULL,
                target_value REAL NOT NULL DEFAULT 1.0,
                unit TEXT,
                accent_color TEXT NOT NULL DEFAULT '#3584E4',
                scheduled_days INTEGER NOT NULL DEFAULT 127,
                archived INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL
            );
            CREATE TABLE habit_entries (
                habit_id INTEGER NOT NULL,
                date TEXT NOT NULL,
                entry_status TEXT NOT NULL,
                value REAL NOT NULL DEFAULT 0.0,
                PRIMARY KEY (habit_id, date)
            );",
        )
        .unwrap();
    }

    let db = Database::open_in_memory().unwrap();
    let res = db.restore_backup(&old_db_path);
    let _ = std::fs::remove_file(&old_db_path);
    assert!(matches!(res, Err(DbError::InvalidBackupSchema(_))));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --features ui test_reorder_habits_and_sort_order`
Expected: FAIL (`no method named reorder_habits`)

- [ ] **Step 3: Implement `sort_order`, `reorder_habits`, and strict schema check in `src/engine/db.rs`**

- In `Database::init_tables`: add `sort_order INTEGER NOT NULL DEFAULT 0` to `habits`.
- In `Database::create_habit`: set `sort_order` via `COALESCE((SELECT MAX(sort_order) + 1 FROM habits), 0)`.
- In `Database::get_habits`: order by `sort_order ASC, id ASC`.
- Add `pub fn reorder_habits(&self, ordered_ids: &[i64]) -> Result<(), DbError>` updating `sort_order` in a transaction.
- In `Database::restore_backup`: query `SELECT COUNT(*) FROM pragma_table_info('habits') WHERE name = 'sort_order';` and return `Err(DbError::InvalidBackupSchema(...))` if `0`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --features ui --test test_db && cargo test --features ui --test test_portal_integration`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/engine/db.rs tests/test_db.rs tests/test_portal_integration.rs
git commit -m "feat(db): add habit sort_order, reorder_habits, and strict schema validation"
```

---

### Task 2: Add Header Lock Button, Drag-and-Drop Reorder Mode, and Save Confirmation Dialog

**Files:**
- Modify: `src/ui/sidebar.rs:10-47, 100-110`
- Modify: `src/ui/habit_row.rs:200-430`
- Modify: `src/ui/state.rs:25-110, 165-290`
- Modify: `src/ui/window.rs:28-66, 280-320`
- Test: `tests/test_habit_row.rs`

**Interfaces:**
- Consumes: `Database::reorder_habits(&self, ordered_ids: &[i64]) -> Result<(), DbError>`
- Produces:
  - `pub fn move_habit_in_order(order: &mut Vec<i64>, dragged_id: i64, target_id: i64) -> bool` in `src/ui/habit_row.rs`
  - `HabitRowWidget::enable_reorder_drag<F: Fn(i64, i64) + 'static>(&self, habit_id: i64, on_drop: F)` in `src/ui/habit_row.rs`
  - `SidebarWidgets::lock_btn: gtk4::Button` in `src/ui/sidebar.rs`

- [ ] **Step 1: Write failing unit test `test_move_habit_in_order` in `tests/test_habit_row.rs`**

```rust
#[test]
fn test_move_habit_in_order() {
    let mut order = vec![10, 20, 30, 40];

    // Move 40 to position of 20 (upwards)
    assert!(habit::ui::habit_row::move_habit_in_order(&mut order, 40, 20));
    assert_eq!(order, vec![10, 40, 20, 30]);

    // Move 10 to position of 30 (downwards)
    assert!(habit::ui::habit_row::move_habit_in_order(&mut order, 10, 30));
    assert_eq!(order, vec![40, 20, 30, 10]);

    // Dragging onto self is a no-op
    assert!(!habit::ui::habit_row::move_habit_in_order(&mut order, 20, 20));
    assert_eq!(order, vec![40, 20, 30, 10]);

    // Unknown ID is a no-op
    assert!(!habit::ui::habit_row::move_habit_in_order(&mut order, 99, 20));
    assert_eq!(order, vec![40, 20, 30, 10]);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --features ui --test test_habit_row test_move_habit_in_order`
Expected: FAIL (`cannot find function move_habit_in_order`)

- [ ] **Step 3: Implement `move_habit_in_order`, `enable_reorder_drag`, `lock_btn`, and confirm dialog**

- In `src/ui/habit_row.rs`:
  - Implement `pub fn move_habit_in_order(order: &mut Vec<i64>, dragged_id: i64, target_id: i64) -> bool`.
  - Implement `HabitRowWidget::enable_reorder_drag<F: Fn(i64, i64) + 'static>(&self, habit_id: i64, on_drop: F)` which prepends `gtk4::Image::from_icon_name("list-drag-handle-symbolic")` (with CSS classes `drag-handle` and `dim-label`) to `self.left_box`, sets `self.days_box.set_sensitive(false)`, and attaches `gtk4::DragSource` (providing `habit_id` as `i64`) and `gtk4::DropTarget` (`DragAction::MOVE`, `i64::static_type()`, toggling `.drop-target-hover` CSS class on enter/leave and invoking `on_drop(dragged_id, habit_id)` on drop).
- In `src/ui/sidebar.rs`:
  - Add `pub lock_btn: gtk4::Button` (`changes-prevent-symbolic`, flat, tooltip `"Unlock to Reorder Habits"`) packed via `sidebar_header.pack_start(&lock_btn)`.
- In `src/ui/state.rs`:
  - Add `pub reorder_mode: Rc<RefCell<bool>>` and `pub staged_habit_order: Rc<RefCell<Vec<i64>>>` to `DrawCaches`.
  - Add `pub lock_btn: gtk4::Button` and `pub add_btn: gtk4::Button` to `WindowInner`.
  - In `handle_select`, return early if `*caches.reorder_mode.borrow()` is true.
  - In `rebuild_habit_list`, update `lock_btn` icon (`changes-allow-symbolic` vs `changes-prevent-symbolic`), set `add_btn.set_sensitive(!is_reorder)`, order rows by `staged_habit_order` when `is_reorder` is true, and call `row_widget.enable_reorder_drag` to update `staged_habit_order` and schedule `refresh` via `gtk4::glib::idle_add_local_once`.
- In `src/ui/window.rs`:
  - Connect `lock_btn`: when locked, stage current habit IDs, set `reorder_mode = true`, and refresh; when unlocked, show `libadwaita::AlertDialog` ("Save Habit Order?", responses `"discard"` / `"save"`), saving via `db.reorder_habits` on `"save"` or reverting on `"discard"`.

- [ ] **Step 4: Run fmt, clippy, and full test suite**

Run: `cargo fmt && cargo clippy --features ui --all-targets -- -D warnings && cargo test --features ui`
Expected: PASS with 0 warnings and all tests passing.

- [ ] **Step 5: Commit**

```bash
git add src/ui/sidebar.rs src/ui/habit_row.rs src/ui/state.rs src/ui/window.rs tests/test_habit_row.rs
git commit -m "feat(ui): add header lock button, drag-to-reorder mode, and save confirmation dialog"
```
