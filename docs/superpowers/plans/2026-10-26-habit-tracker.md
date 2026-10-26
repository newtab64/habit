# Habit Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a native, local-only Linux habit tracker app from scratch in strict Rust with `gtk4` and `libadwaita`, packaged as a Flatpak meeting all GNOME Software "Safe" tags, featuring Loop-style exponential decay scoring, pre-defined rest days, 50/50 split view, and atomic SQLite persistence.

**Architecture:** Pure Rust `engine` library containing domain models, SQLite storage, and the exponential decay scoring engine with zero GTK dependencies (100% headlessly testable via `cargo test`), paired with a direct GTK4/Libadwaita `ui` layer utilizing `AdwNavigationSplitView`, dynamic CSS providers for per-habit hex accent styling, and Cairo-rendered canvas charts.

**Tech Stack:** Rust (edition 2021), `gtk4` (v0.9+), `libadwaita` (v0.7+), `cairo-rs`, `rusqlite` (bundled SQLite 3, WAL mode), Meson + Flatpak (GNOME 47 runtime).

**Spec:** [`docs/superpowers/specs/2026-10-26-habit-tracker-design.md`](file:///home/user/Downloads/Repos/habit/docs/superpowers/specs/2026-10-26-habit-tracker-design.md)

## Global Constraints

- `#![forbid(unsafe_code)]` enforced across all crates and source files.
- `#![warn(clippy::pedantic, clippy::nursery)]` enforced cleanly without warnings.
- Zero network permissions in Flatpak: `--share=network` is strictly prohibited.
- Zero host/home filesystem permissions: `--filesystem=host` and `--filesystem=home` are strictly prohibited.
- Persistence is isolated strictly to `$XDG_DATA_HOME/habit/habit.db`.
- Backup export/import must strictly use atomic SQLite `.db` format via the XDG File Chooser Portal (`gtk4::FileDialog`). No CSV/JSON.
- Zero UI animations or transition delay fluff.
- All-or-nothing measurable habit evaluation: progress below target ($v < \text{target}$) is marked incomplete (`✕`), does not count toward streaks, and does not add points to habit strength.
- Pre-defined rest days: unscheduled days for a habit are automatically treated as rest days (`—`), preserving streaks and habit strength without decay penalty.

## Review Focus

1. **Date/Time Boundary Edge Cases:** Crossing midnight while the app remains open must shift the 7-day window (`Today` through `-6d`) without corrupting logs or locking timestamps.
2. **Database Corruption on Sudden Termination:** Power cut or `kill -9` during write must leave SQLite intact via WAL mode atomic transactions.
3. **Corrupted Database Import:** An invalid or tampered `.db` file selected during import must fail schema and `PRAGMA integrity_check` validation gracefully without overwriting the user's active database.
4. **Leap Year and Month Traversal in Heatmap:** Year-boundary month traversal (December to January) and leap-year Februarys must compute accurate day-of-week cell offsets without panics.
5. **Extreme Measurable Values:** Zero targets, negative values, or values $> 1,000,000$ must be clamped or rejected cleanly without division-by-zero or graph scaling overflows.

---

### Task 1: Project Scaffolding & Strict Rust Configuration

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `src/lib.rs`
- Create: `build.rs`
- Create: `data/io.github.habit.Habit.metainfo.xml`
- Create: `data/io.github.habit.Habit.desktop`
- Create: `build-aux/io.github.habit.Habit.json`
- Test: `tests/test_scaffolding.rs`

**Interfaces:**
- Produces: Base crate configuration with `#![forbid(unsafe_code)]`, strict linting flags, and Flatpak build definition.

- [ ] **Step 1: Write the test verifying strict compilation and crate sanity**

```rust
// tests/test_scaffolding.rs
#[test]
fn test_crate_compiles_under_strict_safety() {
    assert_eq!(habit::APP_ID, "io.github.habit.Habit");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test test_scaffolding`
Expected: FAIL with "cannot find crate `habit`"

- [ ] **Step 3: Create `Cargo.toml`, `src/lib.rs`, `src/main.rs`, and Flatpak manifest**

Configure `Cargo.toml` with dependencies: `rusqlite = { version = "0.32", features = ["bundled"] }`, `chrono = { version = "0.4", default-features = false, features = ["clock", "std"] }`, `gtk4 = "0.9"`, `libadwaita = "0.7"`, `cairo-rs = "0.20"`.
Add `src/lib.rs` with `pub const APP_ID: &str = "io.github.habit.Habit";` and inner attribute `#![forbid(unsafe_code)]`.
Create Flatpak manifest `build-aux/io.github.habit.Habit.json` omitting `--share=network` and `--filesystem=home/host`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test test_scaffolding`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml src/ tests/ data/ build-aux/
git commit -m "chore: scaffold project structure with strict rust and flatpak manifest"
```

---

### Task 2: Core Domain Models & Frequency Logic

**Files:**
- Create: `src/engine/models.rs`
- Create: `src/engine/mod.rs`
- Test: `tests/test_models.rs`

**Interfaces:**
- Produces:
  - `enum HabitType { Binary, Measurable { target: f64, unit: String } }`
  - `enum EntryStatus { Completed, Skipped, Missed }`
  - `struct Habit { id: Option<i64>, name: String, habit_type: HabitType, accent_color: String, scheduled_days: u8, archived: bool, created_at: i64 }`
  - `impl Habit { pub fn is_scheduled_on(&self, weekday: chrono::Weekday) -> bool; }`

- [ ] **Step 1: Write failing tests for habit models and schedule masks**

```rust
// tests/test_models.rs
use chrono::Weekday;
use habit::engine::models::{Habit, HabitType};

#[test]
fn test_habit_schedule_mask() {
    let gym = Habit::new_gym_example(); // scheduled Mon, Wed, Fri, Sat
    assert!(gym.is_scheduled_on(Weekday::Mon));
    assert!(!gym.is_scheduled_on(Weekday::Tue)); // pre-defined rest day
    assert!(gym.is_scheduled_on(Weekday::Wed));
    assert!(!gym.is_scheduled_on(Weekday::Sun)); // pre-defined rest day
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test test_models`
Expected: FAIL with "module `engine` not found"

- [ ] **Step 3: Implement `Habit`, `HabitType`, `EntryStatus`, and weekday bitmask helpers**

In `src/engine/models.rs`, implement `is_scheduled_on` using bitmask representation ($1 = \text{Mon} \dots 64 = \text{Sun}$). Add hex color validation ensuring standard `#RRGGBB` format.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test test_models`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/engine/ tests/test_models.rs
git commit -m "feat(engine): add domain models and weekday schedule bitmask"
```

---

### Task 3: SQLite Storage & Atomic Transactions

**Files:**
- Create: `src/engine/db.rs`
- Test: `tests/test_db.rs`

**Interfaces:**
- Consumes: `Habit`, `HabitType`, `EntryStatus` from `src/engine/models.rs`
- Produces:
  - `struct Database { conn: rusqlite::Connection }`
  - `impl Database { pub fn open_in_memory() -> Result<Self, DbError>; }`
  - `impl Database { pub fn open(path: &std::path::Path) -> Result<Self, DbError>; }`
  - `impl Database { pub fn create_habit(&self, habit: &Habit) -> Result<i64, DbError>; }`
  - `impl Database { pub fn get_habits(&self) -> Result<Vec<Habit>, DbError>; }`
  - `impl Database { pub fn log_entry(&self, habit_id: i64, date: chrono::NaiveDate, status: EntryStatus, value: f64) -> Result<(), DbError>; }`
  - `impl Database { pub fn get_entries(&self, habit_id: i64, from: chrono::NaiveDate, to: chrono::NaiveDate) -> Result<Vec<HabitEntry>, DbError>; }`
  - `impl Database { pub fn export_backup(&self, target_path: &std::path::Path) -> Result<(), DbError>; }`
  - `impl Database { pub fn restore_backup(&mut self, source_path: &std::path::Path) -> Result<(), DbError>; }`

- [ ] **Step 1: Write failing tests for atomic DB operations and backup export/restore**

```rust
// tests/test_db.rs
use habit::engine::db::Database;
use habit::engine::models::{Habit, EntryStatus};
use chrono::NaiveDate;

#[test]
fn test_atomic_entry_logging_and_backup() {
    let mut db = Database::open_in_memory().unwrap();
    let id = db.create_habit(&Habit::test_fixture()).unwrap();
    let today = NaiveDate::from_ymd_opt(2026, 10, 26).unwrap();
    
    db.log_entry(id, today, EntryStatus::Completed, 1.0).unwrap();
    let entries = db.get_entries(id, today, today).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].status, EntryStatus::Completed);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test test_db`
Expected: FAIL with "cannot find type `Database`"

- [ ] **Step 3: Implement SQLite schema with WAL mode and `rusqlite::backup`**

Implement `Database` in `src/engine/db.rs`. Enable `PRAGMA journal_mode = WAL;` and foreign keys. Implement atomic export and import using `rusqlite::backup::Backup` with `PRAGMA integrity_check` verification.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test test_db`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/engine/db.rs tests/test_db.rs
git commit -m "feat(engine): implement atomic sqlite persistence and backup api"
```

---

### Task 4: Exponential Moving Average Scoring Engine & Streaks

**Files:**
- Create: `src/engine/scoring.rs`
- Test: `tests/test_scoring.rs`

**Interfaces:**
- Consumes: `Habit`, `EntryStatus`, `HabitEntry` from `src/engine/models.rs` and `db.rs`
- Produces:
  - `pub fn calculate_habit_strength(habit: &Habit, entries: &[HabitEntry], as_of: chrono::NaiveDate) -> f64;`
  - `pub fn calculate_streaks(habit: &Habit, entries: &[HabitEntry], as_of: chrono::NaiveDate) -> (u32, u32);` // (current, best)
  - `pub fn calculate_strength_history(habit: &Habit, entries: &[HabitEntry], days: usize, as_of: chrono::NaiveDate) -> Vec<f64>;`

- [ ] **Step 1: Write failing tests for exponential decay, streak preservation on rest days, and all-or-nothing measurable evaluation**

```rust
// tests/test_scoring.rs
use chrono::NaiveDate;
use habit::engine::models::{Habit, HabitType, EntryStatus, HabitEntry};
use habit::engine::scoring::{calculate_habit_strength, calculate_streaks};

#[test]
fn test_predefined_rest_days_do_not_decay_or_break_streak() {
    let gym = Habit::new_gym_example(); // Mon, Wed, Fri, Sat
    // Mon: Done, Tue: Pre-defined Rest, Wed: Done
    let d1 = NaiveDate::from_ymd_opt(2026, 10, 19).unwrap(); // Mon
    let d2 = NaiveDate::from_ymd_opt(2026, 10, 20).unwrap(); // Tue (rest)
    let d3 = NaiveDate::from_ymd_opt(2026, 10, 21).unwrap(); // Wed
    
    let entries = vec![
        HabitEntry { habit_id: 1, date: d1, status: EntryStatus::Completed, value: 1.0 },
        HabitEntry { habit_id: 1, date: d2, status: EntryStatus::Skipped, value: 0.0 },
        HabitEntry { habit_id: 1, date: d3, status: EntryStatus::Completed, value: 1.0 },
    ];
    let (curr, best) = calculate_streaks(&gym, &entries, d3);
    assert_eq!(curr, 2); // 2 workout sessions completed across rest day
}

#[test]
fn test_measurable_partial_progress_does_not_add_strength_or_keep_streak() {
    let reading = Habit::new_measurable("Read", 30.0, "pages");
    let d = NaiveDate::from_ymd_opt(2026, 10, 26).unwrap();
    let partial_entries = vec![
        HabitEntry { habit_id: 2, date: d, status: EntryStatus::Missed, value: 15.0 },
    ];
    let (curr, _) = calculate_streaks(&reading, &partial_entries, d);
    assert_eq!(curr, 0); // Partial progress breaks streak
    let strength = calculate_habit_strength(&reading, &partial_entries, d);
    assert_eq!(strength, 0.0); // Does not add to strength
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test test_scoring`
Expected: FAIL with "cannot find module `scoring`"

- [ ] **Step 3: Implement exponential decay and streak logic in `src/engine/scoring.rs`**

Implement exponential moving average: $S_t = S_{t-1} + (1.0 - S_{t-1}) \cdot \alpha$ for completion, $S_t = S_{t-1} \cdot (1.0 - \alpha)$ for misses, and $S_t = S_{t-1}$ for pre-defined rest days (`EntryStatus::Skipped`). Compute streaks as consecutive active scheduled days met without an unexcused miss.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test test_scoring`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/engine/scoring.rs tests/test_scoring.rs
git commit -m "feat(engine): implement loop decay formula and rest day streak preservation"
```

---

### Task 5: Cairo Drawing Engine for Charts

**Files:**
- Create: `src/ui/charts.rs`
- Create: `src/ui/mod.rs`
- Test: `tests/test_charts.rs`

**Interfaces:**
- Consumes: Habit strength series `&[f64]` and historical entry maps
- Produces:
  - `pub fn draw_strength_curve(cr: &cairo::Context, width: f64, height: f64, data: &[f64], color: &gdk::RGBA);`
  - `pub fn draw_monthly_heatmap(cr: &cairo::Context, width: f64, height: f64, year: i32, month: u32, entries: &std::collections::HashMap<chrono::NaiveDate, EntryStatus>, color: &gdk::RGBA);`
  - `pub fn heatmap_hit_test(x: f64, y: f64, width: f64, height: f64, year: i32, month: u32) -> Option<chrono::NaiveDate>;`

- [ ] **Step 1: Write tests for heatmap coordinate hit-testing and calendar layout**

```rust
// tests/test_charts.rs
use chrono::NaiveDate;
use habit::ui::charts::heatmap_hit_test;

#[test]
fn test_heatmap_hit_test_identifies_correct_date() {
    let date = heatmap_hit_test(20.0, 20.0, 200.0, 150.0, 2026, 10);
    assert!(date.is_some());
    assert_eq!(date.unwrap().month(), 10);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test test_charts`
Expected: FAIL with "module `ui` not found"

- [ ] **Step 3: Implement Cairo chart rendering and hit testing**

In `src/ui/charts.rs`, implement smooth bezier line rendering for `draw_strength_curve` with gradient fill under the curve. Implement 7-column calendar matrix rendering for `draw_monthly_heatmap` and coordinate projection in `heatmap_hit_test`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test test_charts`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/ui/ tests/test_charts.rs
git commit -m "feat(ui): implement cairo strength curve and interactive heatmap drawing"
```

---

### Task 6: UI Habit Row & Minimalist Checklist Component

**Files:**
- Create: `src/ui/habit_row.rs`
- Modify: `src/ui/mod.rs`
- Test: Manual UI verification & unit tests in `src/ui/habit_row.rs`

**Interfaces:**
- Consumes: `Habit`, `HabitEntry` from `engine`
- Produces:
  - `pub struct HabitRowWidget;` (GTK4 widget representing a habit row)
  - Signals/Callbacks: `on_toggle_day(habit_id, date)`, `on_select(habit_id)`

- [ ] **Step 1: Create `HabitRowWidget` declaration and state layout**

Implement layout:
- Left title column with habit name.
- Underneath title: `GtkProgressBar` with custom CSS class `.habit-progress` and no text labels.
- 7 day symbols horizontally: `Today`, `Yest`, `-2d`, `-3d`, `-4d`, `-5d`, `-6d`.
- Render bare symbols without circles: `✓` (accent), `—` (accent dash), `✕` (muted gray), or raw measurable number.

- [ ] **Step 2: Add click gesture handlers for fast day toggling**

Attach `gtk4::GestureClick` to each day column. Toggling a day invokes the `on_toggle_day` callback, emitting state transitions instantly without modal popups for binary habits. For measurable habits, display a compact `GtkPopover` containing a number entry stepper.

- [ ] **Step 3: Apply dynamic CSS provider for per-habit accent color**

Set CSS styles per row using `gtk4::CssProvider` to tint progress bar fill and text symbols to the habit's parsed `accent_color`.

- [ ] **Step 4: Verify row layout compiles cleanly under strict Rust**

Run: `cargo check --all-targets`
Expected: PASS with zero warnings under `clippy::pedantic`.

- [ ] **Step 5: Commit**

```bash
git add src/ui/habit_row.rs src/ui/mod.rs
git commit -m "feat(ui): build habit row with minimal bare symbols and accent styling"
```

---

### Task 7: Main Window & 50/50 Split View Integration

**Files:**
- Create: `src/ui/window.rs`
- Modify: `src/ui/mod.rs`
- Modify: `src/main.rs`

**Interfaces:**
- Consumes: `Database`, `HabitRowWidget`, `charts`
- Produces:
  - `pub struct HabitWindow;`
  - Connects `AdwNavigationSplitView` with left sidebar and right analytics detail view.

- [ ] **Step 1: Build the `AdwNavigationSplitView` layout**

In `src/ui/window.rs`:
- Left Page (`AdwNavigationPage`): Headerbar with "Habits" title, "+ New Habit" button, and list container of `HabitRowWidget`s.
- Right Page: Details pane featuring:
  - Header with habit title, frequency label, and Hex color badge.
  - 3 metric cards: Strength %, Current Streak, Best Streak.
  - `GtkDrawingArea` for the 30-day Habit Strength Curve.
  - `GtkDrawingArea` for the Monthly Calendar Heatmap with `<` and `>` month navigation buttons.

- [ ] **Step 2: Connect interactive heatmap click gestures for historical editing**

Attach `gtk4::GestureClick` to the monthly heatmap canvas. When a user clicks any day, use `heatmap_hit_test` to determine the target date and toggle or log its entry in the SQLite database, immediately refreshing the canvas and left pane.

- [ ] **Step 3: Implement responsive collapse breakpoint**

Configure `AdwNavigationSplitView::set_collapsed()` when window width drops below 760px. When collapsed, selecting a habit pushes the details navigation page into the foreground with a native Back button.

- [ ] **Step 4: Verify application launches and renders without runtime warnings**

Run: `cargo test` and `cargo check`
Expected: PASS with zero warnings.

- [ ] **Step 5: Commit**

```bash
git add src/ui/window.rs src/ui/mod.rs src/main.rs
git commit -m "feat(ui): integrate 50/50 responsive split view and interactive details pane"
```

---

### Task 8: Habit Creation / Edit Dialog & Hex Color Picker

**Files:**
- Create: `src/ui/habit_dialog.rs`
- Modify: `src/ui/window.rs`
- Modify: `src/ui/mod.rs`

**Interfaces:**
- Consumes: `Habit`, `Database`
- Produces:
  - `pub fn show_habit_dialog(parent: &gtk4::Window, habit: Option<&Habit>, on_save: impl Fn(Habit) + 'static);`

- [ ] **Step 1: Implement `AdwPreferencesWindow` for habit creation and editing**

Fields:
- Name (`AdwEntryRow`).
- Habit Type (Binary vs Measurable toggle).
- Target Value & Unit fields (visible when Measurable selected).
- Schedule: Checkboxes for Monday through Sunday (pre-defining active workout days vs rest days).
- Hex Color: Text entry validating 6-digit hex format (e.g. `#FF7800`) alongside 8 GNOME preset color buttons.

- [ ] **Step 2: Wire save action to SQLite database creation/update**

On confirm, validate that name is non-empty, target value is $> 0$, and at least one day is scheduled. Save to `Database` and refresh `HabitWindow`.

- [ ] **Step 3: Verify dialog validation logic with unit tests**

Run: `cargo test`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src/ui/habit_dialog.rs src/ui/window.rs
git commit -m "feat(ui): add habit creation and editing dialog with hex color picker"
```

---

### Task 9: Database Export & Import via XDG FileChooser Portal

**Files:**
- Create: `src/ui/portal.rs`
- Modify: `src/ui/window.rs`
- Test: `tests/test_portal_integration.rs`

**Interfaces:**
- Consumes: `Database::export_backup`, `Database::restore_backup`
- Produces:
  - `pub fn trigger_export_dialog(window: &gtk4::Window, db: &Database);`
  - `pub fn trigger_import_dialog(window: &gtk4::Window, db: &mut Database, on_success: impl Fn() + 'static);`

- [ ] **Step 1: Write test verifying backup database validation before restore**

```rust
// tests/test_portal_integration.rs
use habit::engine::db::Database;
use std::fs::File;
use std::io::Write;

#[test]
fn test_restore_rejects_non_sqlite_file() {
    let mut db = Database::open_in_memory().unwrap();
    let temp_dir = std::env::temp_dir();
    let corrupt_file = temp_dir.join("corrupt.db");
    let mut f = File::create(&corrupt_file).unwrap();
    f.write_all(b"not a valid sqlite database").unwrap();
    
    assert!(db.restore_backup(&corrupt_file).is_err());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test test_portal_integration`
Expected: FAIL with "cannot find function `restore_backup`"

- [ ] **Step 3: Implement Portal FileDialog handlers**

In `src/ui/portal.rs`:
- Use `gtk4::FileDialog` with filter `*.db` ("SQLite Database").
- `trigger_export_dialog`: prompts user for save location and executes atomic SQLite backup.
- `trigger_import_dialog`: prompts user for file, performs integrity check, swaps database file atomically, and reloads window data.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test test_portal_integration`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/ui/portal.rs tests/test_portal_integration.rs src/ui/window.rs
git commit -m "feat(ui): integrate xdg file chooser portal for atomic db export and import"
```

---

### Task 10: Flatpak Build & Sandboxing Verification

**Files:**
- Create: `meson.build`
- Create: `src/meson.build`
- Modify: `build-aux/io.github.habit.Habit.json`
- Test: Sandbox and lint verification scripts

**Interfaces:**
- Produces: Complete Flatpak build definition adhering to GNOME Software "Safe" tag criteria.

- [ ] **Step 1: Create Meson build definitions for Flatpak build stream**

Create `meson.build` integrating Cargo build with Meson install targets for binary, desktop file, metainfo XML, and scalable SVG icon.

- [ ] **Step 2: Verify Flatpak permissions adhere strictly to Safe criteria**

Inspect `build-aux/io.github.habit.Habit.json`:
- `--socket=wayland`
- `--socket=fallback-x11`
- `--share=ipc`
- Confirm absence of `--share=network`, `--filesystem=host`, `--filesystem=home`.

- [ ] **Step 3: Run full clippy pedantic audit and test suite**

Run:
```bash
cargo clippy --all-targets -- -D warnings -D clippy::pedantic -D clippy::nursery
cargo test --all
```
Expected: 0 errors, 0 warnings. All tests pass.

- [ ] **Step 4: Commit**

```bash
git add meson.build src/meson.build build-aux/
git commit -m "build: finalize meson build and flatpak sandboxing configuration"
```
