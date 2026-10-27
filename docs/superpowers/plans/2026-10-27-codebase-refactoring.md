# Codebase Modularization & UI Component Decomposition Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Decompose monolithic UI files (`src/ui/charts.rs` and `src/ui/window.rs`) into single-responsibility modules while keeping all 81 automated tests passing, preserving the locked 520x700 window size, 180ms sheet animations, instant page transitions, and passing strict clippy lints.

**Architecture:** 
1. Convert `src/ui/charts.rs` into a package directory `src/ui/charts/` (`geometry.rs`, `progress_ring.rs`, `curve.rs`, `heatmap.rs`, `calendar.rs`, `mod.rs`).
2. Extract pure domain calculations and formatters from `src/ui/window.rs` into `src/ui/metrics.rs` (zero UI dependencies).
3. Extract application state and the refresh orchestration pipeline into `src/ui/state.rs`.
4. Extract views into `src/ui/sidebar.rs`, `src/ui/details.rs`, and `src/ui/history_editor.rs`.
5. Minimize `src/ui/window.rs` into an application coordinator (~150 lines).

**Tech Stack:** Rust 2021 (`#![forbid(unsafe_code)]`), GTK4 (`gtk4`), Libadwaita (`libadwaita`), Cairo (`cairo-rs`), SQLite (`rusqlite`), Flatpak.

**Spec:** `docs/superpowers/specs/2026-10-27-codebase-refactoring-design.md`

## Global Constraints

- Window size must remain strictly locked at 520x700 (`set_default_size(520, 700)`, `set_resizable(false)`).
- Navigation view page transitions must remain instant (`nav_view.set_animate_transitions(false)`).
- Heatmap square spacing must remain locked at 6.0px (`HEATMAP_SPACING = 6.0`, `square_spacing = 6.0`).
- Bottom sheet history editor animation must remain locked at 180ms duration with 20px side margins (`sheet_revealer.set_transition_duration(180)`, `sheet_box.set_margin_start(20)`, `sheet_box.set_margin_end(20)`).
- Zero unsafe code (`#![forbid(unsafe_code)]`).
- All 81 automated tests in `tests/` must pass after every task (`cargo test --all --features ui`).
- Clippy must pass with zero warnings (`cargo clippy --all --features ui -- -D warnings`).
- Local-only sandboxed operation: zero network calls, pure local SQLite storage.

## Review Focus

1. **Re-export Completeness in `src/ui/charts/mod.rs`:** If any helper or type previously exported by `src/ui/charts.rs` is omitted from `charts/mod.rs`, `tests/test_charts.rs` or other modules will fail to compile. All 10 tests in `tests/test_charts.rs` must pass untouched.
2. **Re-export Completeness in `src/ui/window.rs`:** If any metric function or type (`calculate_habit_metrics`, `format_schedule_summary`, `format_streak_badge`, `format_strength_percentage`, `format_target_summary`, `toggle_entry_state`, `HabitMetrics`, `MonthNavigation`) is not re-exported from `window.rs`, `tests/test_window.rs` will fail to compile. All 11 tests in `tests/test_window.rs` must pass untouched.
3. **Purity of `src/ui/metrics.rs`:** `src/ui/metrics.rs` must not import GTK, Libadwaita, or Cairo. Pure tests running via `cargo test` (without `--features ui`) must compile and pass `tests/test_window.rs`.
4. **History Editor Sheet Lifecycle:** Ensure `HistoryEditorWidgets` correctly reveals in 180ms on "Edit History" click, closes on backdrop tap, Cancel click, and Escape keypress without re-triggering lag or dangling callbacks.
5. **Flatpak Sandboxed Packaging Integrity:** The refactored codebase must compile and bundle cleanly inside the Flatpak build sandbox (`build-aux/io.github.habit.Habit.json`) without missing file errors.

---

### Task 1: Split `src/ui/charts.rs` into `src/ui/charts/` Package Directory

**Files:**
- Create: `src/ui/charts/geometry.rs`
- Create: `src/ui/charts/progress_ring.rs`
- Create: `src/ui/charts/curve.rs`
- Create: `src/ui/charts/heatmap.rs`
- Create: `src/ui/charts/calendar.rs`
- Create: `src/ui/charts/mod.rs`
- Delete: `src/ui/charts.rs`
- Test: `tests/test_charts.rs`

**Interfaces:**
- Consumes: `cairo::Context` (when `feature = "ui"`), `chrono::NaiveDate`, `crate::engine::models::EntryStatus`.
- Produces:
  - `geometry`: `Point { pub x: f64, pub y: f64 }`, `Rect { pub x: f64, pub y: f64, pub width: f64, pub height: f64 }`, `Rgba { pub r: f64, pub g: f64, pub b: f64, pub a: f64 }`, `Rgba::from_hex(hex: &str) -> Option<Self>`.
  - `progress_ring`: `draw_progress_ring(cr: &cairo::Context, width: f64, height: f64, progress: f64, color: Rgba)`.
  - `curve`: `calculate_curve_points(scores: &[f64], width: f64, height: f64) -> Vec<Point>`, `draw_strength_curve(cr: &cairo::Context, width: f64, height: f64, scores: &[f64], color: Rgba)`.
  - `heatmap`: `HEATMAP_SPACING: f64 = 6.0`, `LoopHeatmapLayout`, `calculate_loop_heatmap_layout(total_width: f64, total_height: f64, today: NaiveDate, week_offset: usize) -> LoopHeatmapLayout`, `loop_heatmap_cell_rect(layout: &LoopHeatmapLayout, col: usize, row: usize) -> Rect`, `loop_heatmap_hit_test(layout: &LoopHeatmapLayout, x: f64, y: f64) -> Option<NaiveDate>`, `draw_loop_heatmap(cr: &cairo::Context, width: f64, height: f64, entries: &HashMap<NaiveDate, EntryStatus>, today: NaiveDate, week_offset: usize, color: Rgba)`.
  - `calendar`: `days_in_month(year: i32, month: u32) -> u32`, `month_name(month: u32) -> &'static str`, `heatmap_cell_rect(col: usize, row: usize, cell_size: f64, spacing: f64) -> Rect`, `heatmap_hit_test(x: f64, y: f64, year: i32, month: u32, cell_size: f64, spacing: f64) -> Option<NaiveDate>`, `draw_monthly_heatmap(cr: &cairo::Context, width: f64, height: f64, year: i32, month: u32, entries: &HashMap<NaiveDate, EntryStatus>, color: Rgba)`.
  - `mod.rs`: Re-exports all of the above so `use crate::ui::charts::*;` and `tests/test_charts.rs` compile identically.

- [ ] **Step 1: Create `src/ui/charts/geometry.rs`**
Move `Point`, `Rect`, and `Rgba` (including `Rgba::from_hex`) into `src/ui/charts/geometry.rs`.
Preserve all docstrings and unit tests.

- [ ] **Step 2: Create `src/ui/charts/progress_ring.rs`**
Move `draw_progress_ring` behind `#[cfg(feature = "ui")]` into `src/ui/charts/progress_ring.rs`.
Imports `cairo::Context` and `super::geometry::Rgba`.

- [ ] **Step 3: Create `src/ui/charts/curve.rs`**
Move `calculate_curve_points` and `draw_strength_curve` into `src/ui/charts/curve.rs`.
Imports `super::geometry::{Point, Rgba}` and `cairo::Context` (`#[cfg(feature = "ui")]`).

- [ ] **Step 4: Create `src/ui/charts/heatmap.rs`**
Move `LoopHeatmapLayout`, `calculate_loop_heatmap_layout`, `loop_heatmap_cell_rect`, `loop_heatmap_hit_test`, and `draw_loop_heatmap` into `src/ui/charts/heatmap.rs`.
Ensure `HEATMAP_SPACING = 6.0` is preserved.

- [ ] **Step 5: Create `src/ui/charts/calendar.rs`**
Move `days_in_month`, `month_name`, `heatmap_cell_rect`, `heatmap_hit_test`, and `draw_monthly_heatmap` into `src/ui/charts/calendar.rs`.

- [ ] **Step 6: Create `src/ui/charts/mod.rs` and delete `src/ui/charts.rs`**
In `src/ui/charts/mod.rs`, declare `pub mod calendar; pub mod curve; pub mod geometry; pub mod heatmap; pub mod progress_ring;` and `pub use` all items.
Remove `src/ui/charts.rs`.

- [ ] **Step 7: Run tests to verify charts refactoring**
Run: `cargo test --test test_charts --features ui`
Expected: 10 passed; 0 failed.
Run: `cargo test --all --features ui`
Expected: 81 passed; 0 failed.

- [ ] **Step 8: Commit**
```bash
git add src/ui/charts/ src/ui/charts.rs
git commit -m "refactor(ui): split charts module into cohesive package directory"
```

---

### Task 2: Extract `src/ui/metrics.rs` from `src/ui/window.rs`

**Files:**
- Create: `src/ui/metrics.rs`
- Modify: `src/ui/mod.rs`
- Modify: `src/ui/window.rs`
- Test: `tests/test_window.rs`

**Interfaces:**
- Consumes: `chrono::NaiveDate`, `crate::engine::models::*`, `crate::engine::scoring::*`, `crate::ui::habit_row::{format_measurable_value, next_toggle_status}`.
- Produces:
  - `MonthNavigation`: `new(year: i32, month: u32) -> Self`, `prev(&self) -> Self`, `next(&self) -> Self`, `format_title(&self) -> String`.
  - Formatters: `format_schedule_summary(scheduled_days: u8) -> String`, `format_streak_badge(current_streak: u32) -> String`, `format_strength_percentage(score: f64) -> String`, `format_percentage_diff(diff: f64) -> String`, `format_target_summary(habit: &Habit) -> String`.
  - `HabitMetrics`: struct with score, diffs, streak, and formatted summary strings.
  - `calculate_habit_metrics(habit: &Habit, entries: &[HabitEntry], today: NaiveDate) -> HabitMetrics`.
  - `toggle_entry_state(habit: &Habit, current_entry: Option<&HabitEntry>, explicit_value: Option<f64>) -> (EntryStatus, f64)`.
  - Note: zero GTK or Libadwaita dependencies!

- [ ] **Step 1: Create `src/ui/metrics.rs`**
Extract `MonthNavigation`, `format_schedule_summary`, `format_streak_badge`, `format_strength_percentage`, `format_percentage_diff`, `format_target_summary`, `HabitMetrics`, `calculate_habit_metrics`, and `toggle_entry_state` into `src/ui/metrics.rs`.
Ensure no GTK/cairo dependencies are included.

- [ ] **Step 2: Update `src/ui/mod.rs` and `src/ui/window.rs`**
In `src/ui/mod.rs`: add `pub mod metrics;`.
In `src/ui/window.rs`: remove extracted definitions and add `pub use crate::ui::metrics::*;`.

- [ ] **Step 3: Run pure non-UI tests and full tests**
Run: `cargo test --test test_window`
Expected: 11 passed; 0 failed (verifies metrics compiles without `--features ui`).
Run: `cargo test --all --features ui`
Expected: 81 passed; 0 failed.

- [ ] **Step 4: Commit**
```bash
git add src/ui/metrics.rs src/ui/mod.rs src/ui/window.rs
git commit -m "refactor(ui): extract pure domain metrics and formatting to src/ui/metrics.rs"
```

---

### Task 3: Extract `src/ui/state.rs` from `src/ui/window.rs`

**Files:**
- Create: `src/ui/state.rs`
- Modify: `src/ui/mod.rs`
- Modify: `src/ui/window.rs`
- Test: `tests/test_window.rs`

**Interfaces:**
- Consumes: `crate::engine::db::Database`, `crate::engine::models::*`, `crate::engine::scoring::*`, `crate::ui::charts::*`, `crate::ui::metrics::*`, `crate::ui::habit_row::*`, `gtk4`, `libadwaita`.
- Produces:
  - `DrawCaches`: struct containing shared `Rc<RefCell<...>>` caches for curve, heatmap, overview ring, and history editor.
  - `DrawCaches::new(today: NaiveDate) -> Self`.
  - `WindowInner`: struct holding application UI widget handles, `db: Rc<RefCell<Database>>`, `selected_habit_id: Option<i64>`, and `heatmap_week_offset: usize`.
  - `get_database_path() -> PathBuf`.
  - `handle_toggle(inner_rc: &Rc<RefCell<WindowInner>>, caches: &DrawCaches, habit_id: i64, date: NaiveDate, explicit_value: Option<f64>)`.
  - `handle_select(inner_rc: &Rc<RefCell<WindowInner>>, caches: &DrawCaches, habit_id: i64)`.
  - `refresh(inner_rc: &Rc<RefCell<WindowInner>>, caches: &DrawCaches)`.

- [ ] **Step 1: Create `src/ui/state.rs`**
Extract `DrawCaches`, `WindowInner`, `get_database_path`, `handle_toggle`, `handle_select`, and `refresh` under `#[cfg(feature = "ui")]`.

- [ ] **Step 2: Update `src/ui/mod.rs` and `src/ui/window.rs`**
In `src/ui/mod.rs`: add `#[cfg(feature = "ui")] pub mod state;`.
In `src/ui/window.rs`: import `state::{DrawCaches, WindowInner, get_database_path, handle_select, handle_toggle, refresh}` and remove their definitions from `window.rs`.

- [ ] **Step 3: Run test suite**
Run: `cargo test --all --features ui`
Expected: 81 passed; 0 failed.

- [ ] **Step 4: Commit**
```bash
git add src/ui/state.rs src/ui/mod.rs src/ui/window.rs
git commit -m "refactor(ui): extract state management and refresh orchestration to src/ui/state.rs"
```

---

### Task 4: Extract `src/ui/sidebar.rs` from `src/ui/window.rs`

**Files:**
- Create: `src/ui/sidebar.rs`
- Modify: `src/ui/mod.rs`
- Modify: `src/ui/window.rs`
- Test: `tests/test_window.rs`

**Interfaces:**
- Consumes: `src/ui/state::{WindowInner, DrawCaches, handle_toggle, handle_select, refresh}`, `gtk4`, `libadwaita`.
- Produces:
  - `SidebarWidgets`: struct holding `habits_list_box: gtk4::Box`, `header_day_labels: Vec<gtk4::Label>`, `habit_css_provider: gtk4::CssProvider`, `habit_score_label: gtk4::Label`, `add_btn: gtk4::Button`, `sidebar_page: libadwaita::NavigationPage`.
  - `build_sidebar(inner_rc: &Rc<RefCell<WindowInner>>, caches: &DrawCaches, root_overlay: &gtk4::Overlay) -> SidebarWidgets`.

- [ ] **Step 1: Create `src/ui/sidebar.rs`**
Extract `SidebarWidgets` and `build_sidebar` (including weekday headers, quick check list box, and Add Habit dialog button connection) into `src/ui/sidebar.rs`.
Ensure `#[cfg(feature = "ui")]` guard is in place.

- [ ] **Step 2: Update `src/ui/mod.rs` and wire `src/ui/sidebar.rs` into `src/ui/window.rs`**
In `src/ui/mod.rs`: add `#[cfg(feature = "ui")] pub mod sidebar;`.
In `src/ui/window.rs`: call `sidebar::build_sidebar(...)` and use the resulting widgets to populate `WindowInner` and add `sidebar_page` to `nav_view`.

- [ ] **Step 3: Run test suite**
Run: `cargo test --all --features ui`
Expected: 81 passed; 0 failed.

- [ ] **Step 4: Commit**
```bash
git add src/ui/sidebar.rs src/ui/mod.rs src/ui/window.rs
git commit -m "refactor(ui): extract habit list sidebar component to src/ui/sidebar.rs"
```

---

### Task 5: Extract `src/ui/history_editor.rs` and `src/ui/details.rs` from `src/ui/window.rs`

**Files:**
- Create: `src/ui/history_editor.rs`
- Create: `src/ui/details.rs`
- Modify: `src/ui/mod.rs`
- Modify: `src/ui/window.rs`
- Test: `tests/test_window.rs`

**Interfaces:**
- Consumes: `src/ui/state::{WindowInner, DrawCaches, refresh}`, `src/ui/metrics::*`, `src/ui/charts::*`, `gtk4`, `libadwaita`.
- Produces:
  - `HistoryEditorWidgets`: struct holding `container: gtk4::Box`, `revealer: gtk4::Revealer`, `sheet_box: gtk4::Box`, `drawing_area: gtk4::DrawingArea`, `prev_btn: gtk4::Button`, `today_btn: gtk4::Button`, `next_btn: gtk4::Button`, `cancel_btn: gtk4::Button`, `save_btn: gtk4::Button`, `year_title: gtk4::Label`.
  - `build_history_editor(inner_rc: &Rc<RefCell<WindowInner>>, caches: &DrawCaches, root_window: &gtk4::Window) -> HistoryEditorWidgets`.
  - `DetailsWidgets`: struct holding `details_page: libadwaita::NavigationPage`, `title_label`, `schedule_label`, `target_label`, `edit_btn`, `delete_btn`, `ring_drawing_area`, `score_card_value`, `month_diff_card_value`, `year_diff_card_value`, `total_count_card_value`, `curve_drawing_area`, `heatmap_drawing_area`, `heatmap_nav_box`, `prev_week_btn`, `today_week_btn`, `next_week_btn`, `edit_mode_btn`, `details_overlay: gtk4::Overlay`, `history_editor: HistoryEditorWidgets`.
  - `build_details(inner_rc: &Rc<RefCell<WindowInner>>, caches: &DrawCaches, root_window: &gtk4::Window, root_overlay: &gtk4::Overlay) -> DetailsWidgets`.

- [ ] **Step 1: Create `src/ui/history_editor.rs`**
Extract `HistoryEditorWidgets` and `build_history_editor` into `src/ui/history_editor.rs`.
Preserve:
- 180ms revealer duration (`sheet_revealer.set_transition_duration(180)`).
- 20px side margins (`sheet_box.set_margin_start(20)`, `sheet_box.set_margin_end(20)`).
- Gesture controllers (tap backdrop to close, week scroll on heatmap).
- Escape key shortcut controller.
- Cancel and Save button handlers with red styling on Cancel.

- [ ] **Step 2: Create `src/ui/details.rs`**
Extract `DetailsWidgets`, `build_details`, `build_overview_section`, `build_heatmap_section`, and `connect_detail_actions` into `src/ui/details.rs`.
Embed `HistoryEditorWidgets` inside the details overlay.
Ensure Edit and Delete habit actions are hooked up properly (with red destructive style on Delete confirmation).

- [ ] **Step 3: Update `src/ui/mod.rs` and wire into `src/ui/window.rs`**
In `src/ui/mod.rs`: add `#[cfg(feature = "ui")] pub mod history_editor;` and `#[cfg(feature = "ui")] pub mod details;`.
In `src/ui/window.rs`: call `details::build_details(...)` and add `details_page` to `nav_view`.

- [ ] **Step 4: Run test suite**
Run: `cargo test --all --features ui`
Expected: 81 passed; 0 failed.

- [ ] **Step 5: Commit**
```bash
git add src/ui/history_editor.rs src/ui/details.rs src/ui/mod.rs src/ui/window.rs
git commit -m "refactor(ui): extract details view and history editor overlay into separate modules"
```

---

### Task 6: Streamline `src/ui/window.rs` into Clean Coordinator (~150 lines)

**Files:**
- Modify: `src/ui/window.rs`
- Modify: `src/ui/mod.rs`
- Test: `tests/test_window.rs`

**Interfaces:**
- Consumes: `src/ui/sidebar::*`, `src/ui/details::*`, `src/ui/history_editor::*`, `src/ui/state::*`, `src/ui/metrics::*`.
- Produces:
  - `HabitWindow`: struct wrapping `adw::ApplicationWindow`.
  - `HabitWindow::new(app: &adw::Application) -> Self`.
  - `setup_primary_menu(hamburger_btn: &gtk4::MenuButton, window: &HabitWindow)`.
  - `connect_drawing_areas(sidebar: &SidebarWidgets, details: &DetailsWidgets, caches: &DrawCaches)`.
  - `run_app() -> glib::ExitCode`.
  - `pub use crate::ui::metrics::*;`.

- [ ] **Step 1: Simplify `src/ui/window.rs`**
Refactor `src/ui/window.rs` to keep only `HabitWindow`, `new()`, `setup_primary_menu()`, `connect_drawing_areas()`, `run_app()`, and the public metrics re-export.
Verify window sizing is locked at 520x700:
```rust
window.set_default_size(520, 700);
window.set_resizable(false);
```
Verify page transition animation is disabled:
```rust
nav_view.set_animate_transitions(false);
```

- [ ] **Step 2: Check compiler warnings and lints**
Run: `cargo check --all --features ui`
Expected: 0 errors.
Run: `cargo clippy --all --features ui -- -D warnings`
Expected: 0 warnings.

- [ ] **Step 3: Run all automated tests**
Run: `cargo test --all --features ui`
Expected: 81 passed; 0 failed.

- [ ] **Step 4: Commit**
```bash
git add src/ui/window.rs src/ui/mod.rs
git commit -m "refactor(ui): streamline HabitWindow into minimal coordinator"
```

---

### Task 7: Full System Verification, Flatpak Build & Behavioral Checks

**Files:**
- Test: all tests in `tests/`
- Build: `build-aux/io.github.habit.Habit.json`, `repo`, `habit.flatpak`

- [ ] **Step 1: Run comprehensive test suite across features**
Run: `cargo test` (pure unit tests without UI feature)
Expected: all non-UI tests pass.
Run: `cargo test --all --features ui`
Expected: 81 passed; 0 failed.

- [ ] **Step 2: Run clippy lints**
Run: `cargo clippy --all --features ui -- -D warnings`
Expected: clean with zero warnings.

- [ ] **Step 3: Build, bundle, and reinstall Flatpak package**
Run:
```bash
flatpak-builder --force-clean --repo=repo build-dir build-aux/io.github.habit.Habit.json
flatpak build-bundle repo habit.flatpak io.github.habit.Habit
flatpak install --user -y --reinstall habit.flatpak
```
Expected: successful build and installation.

- [ ] **Step 4: Verify Flatpak executes without runtime errors**
Run:
```bash
flatpak run io.github.habit.Habit --help
```
Expected: exits cleanly with no missing symbols or syntax failures.

- [ ] **Step 5: Final Git commit & push**
Run:
```bash
git status
git push origin main
```
Expected: repository is clean and up to date with remote.
