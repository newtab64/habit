# Fast-Check List & Loop Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Transform Habit into a true Loop Habit Tracker experience: compact fixed-size Fast-Check List with 7-day quick toggles (`✓`, `⏸`, `✕`), push-navigation for details, authentic Loop-style monthly heatmap with day numbers, harder-to-gain score decay, and complete removal of "strength" terminology.

**Architecture:** 
- `src/engine/scoring.rs`: Recalibrate decay to asymmetric half-life ($T_{\text{gain}} = 28$ days, $T_{\text{loss}} = 14$ days).
- `src/ui/charts.rs`: Render Cairo square cells with rounded corners, weekday headers, and centered day numbers.
- `src/ui/habit_row.rs`: Chronological 7-day dates formatted as `MM/DD`, fixed 36px width alignment, and tri-state cycling (`✕` -> `✓` -> `⏸` -> `✕`).
- `src/ui/window.rs`: Lock window at 520x700 non-resizable, migrate from `NavigationSplitView` to `AdwNavigationView` with Back-button navigation, and rename all "Strength" to "Score".

**Tech Stack:** Rust 2021, GTK4 (`gtk4`), Libadwaita (`libadwaita`), Cairo (`cairo-rs`), Chrono (`chrono`), SQLite (`rusqlite`).

**Spec:** `docs/superpowers/specs/2026-10-27-ui-heatmap-and-date-alignment-design.md`

## Global Constraints
- Strictly `#![forbid(unsafe_code)]` across the entire codebase.
- Zero compiler or clippy warnings under `#![warn(clippy::pedantic, clippy::nursery)]`.
- Window locked at 520x700px, `resizable = false`.
- Replace all occurrences of "strength" with "Score" in user-facing strings and identifiers.
- Flatpak sandbox must remain fully isolated (no network, no host filesystem access).

## Review Focus
1. First day of month weekday offset rendering in calendar heatmap (verify 1st falls on correct column Monday..Sunday).
2. Asymmetric scoring calculation when transitioning between completed, skipped, and missed days.
3. 7-day chronological header alignment matches row action buttons to the pixel across light and dark themes.
4. Tri-state cycling (`✕` -> `✓` -> `⏸` -> `✕`) correctly persists `EntryStatus::Completed`, `EntryStatus::Skipped`, and `EntryStatus::Missed`.
5. Navigation view push/pop cleanly updates both master list and detail views without stale cached widgets.

---

### Task 1: Asymmetric Score Calibration & Terminology Update in Scoring Engine

**Files:**
- Modify: `src/engine/scoring.rs:1-170`
- Test: `tests/scoring_tests.rs`

**Interfaces:**
- Consumes: `Habit`, `HabitEntry`, `EntryStatus` from `src/engine/models.rs`.
- Produces: `calculate_habit_score(habit: &Habit, entries: &[HabitEntry], as_of: NaiveDate) -> f64` (aliased or renamed from `calculate_habit_strength`).
- Constants: `pub const GAIN_HALF_LIFE_DAYS: f64 = 28.0;`, `pub const LOSS_HALF_LIFE_DAYS: f64 = 14.0;`.

- [ ] **Step 1: Write tests for asymmetric decay ($T_{\text{gain}} = 28$ days, $T_{\text{loss}} = 14$ days)**

Add test in `tests/scoring_tests.rs`:
```rust
#[test]
fn test_asymmetric_scoring_harder_to_gain() {
    let habit = Habit::new("Exercise", "#3584e4");
    let today = NaiveDate::from_ymd_opt(2026, 10, 27).unwrap();
    
    // 7 consecutive completed days should yield ~15.9% (not 30%+)
    let mut entries = Vec::new();
    for i in (0..7).rev() {
        let d = today - chrono::Duration::days(i);
        entries.push(HabitEntry::new(0, d, EntryStatus::Completed, 1.0));
    }
    let score = calculate_habit_score(&habit, &entries, today);
    assert!((score - 0.159).abs() < 0.02, "Expected ~15.9%, got {score}");
}
```

- [ ] **Step 2: Run test to verify failure**

Run: `cargo test --test scoring_tests test_asymmetric_scoring_harder_to_gain`
Expected: FAIL (function not found or expected score mismatch)

- [ ] **Step 3: Implement asymmetric decay and rename to `calculate_habit_score` in `src/engine/scoring.rs`**

Update `src/engine/scoring.rs` with `GAIN_HALF_LIFE_DAYS = 28.0` and `LOSS_HALF_LIFE_DAYS = 14.0`:
```rust
pub const GAIN_HALF_LIFE_DAYS: f64 = 28.0;
pub const LOSS_HALF_LIFE_DAYS: f64 = 14.0;

// On completed:
let alpha_gain = decay_alpha(GAIN_HALF_LIFE_DAYS);
score = (1.0 - score).mul_add(alpha_gain, score);

// On missed:
let alpha_loss = decay_alpha(LOSS_HALF_LIFE_DAYS);
score *= 1.0 - alpha_loss;
```
Provide backwards compatibility alias `pub use calculate_habit_score as calculate_habit_strength;`.

- [ ] **Step 4: Run tests to verify pass**

Run: `cargo test --test scoring_tests`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/engine/scoring.rs tests/scoring_tests.rs
git commit -m "feat(engine): calibrate asymmetric score decay with 28-day gain half-life"
```

---

### Task 2: Cairo Loop-Style Heatmap with Centered Square Tiles and Day Numbers

**Files:**
- Modify: `src/ui/charts.rs:194-418`
- Test: `tests/chart_tests.rs`

**Interfaces:**
- Consumes: `cairo::Context`, `Rgba`, `EntryStatus`, `days_in_month`.
- Produces: `draw_monthly_heatmap(...)`, `heatmap_cell_rect(...)`, `heatmap_hit_test(...)`.

- [ ] **Step 1: Write unit tests for square cell calculation and hit testing**

```rust
#[test]
fn test_loop_heatmap_cells_are_square_and_centered() {
    let rect = heatmap_cell_rect(0, 0, 400.0, 300.0);
    assert!((rect.width - rect.height).abs() < 1e-6, "Cells must be square");
    assert!(rect.width >= 16.0 && rect.width <= 40.0);
}
```

- [ ] **Step 2: Run test to verify failure**

Run: `cargo test --test chart_tests test_loop_heatmap_cells_are_square_and_centered`
Expected: FAIL (cells were stretched rectangles)

- [ ] **Step 3: Implement centered square cells, rounded corners, weekday headers, and day numbers in `src/ui/charts.rs`**

- Calculate `cell_size = ((avail_w - 6.0 * gap) / 7.0).min((avail_h - header_h - 5.0 * gap) / 6.0).clamp(16.0, 36.0)`.
- Compute grid origin `(origin_x, origin_y)` centering the 7x6 matrix in canvas.
- Draw weekday header labels: `M`, `T`, `W`, `T`, `F`, `S`, `S`.
- Draw rounded rectangle for each day with radius `4.0` px.
- Draw centered day number text `1..31`.
- Colors:
  - Completed: solid accent color, white text.
  - Skipped: muted accent color (`0.25` alpha) with small horizontal dash, accent text.
  - Missed (past): `#2e2e2e` in dark / `#e0e0e0` in light, muted text.
  - Future: very faint outline `0.2` alpha.
- Update `heatmap_hit_test` to use the centered square cell bounds.

- [ ] **Step 4: Run test to verify pass**

Run: `cargo test --test chart_tests`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/ui/charts.rs tests/chart_tests.rs
git commit -m "feat(ui): implement Loop-style square calendar heatmap with day numbers"
```

---

### Task 3: 7-Day Quick-Check Chronological Alignment & Multi-State Cycling

**Files:**
- Modify: `src/ui/habit_row.rs:1-389`
- Test: `tests/ui_row_tests.rs`

**Interfaces:**
- Consumes: `Habit`, `HabitEntry`, `EntryStatus`, `NaiveDate`.
- Produces: `calculate_7_day_dates(today: NaiveDate) -> [NaiveDate; 7]` (chronological left to right: `[today - 6, ..., today]`).
- Produces: `next_toggle_status(current: Option<EntryStatus>) -> EntryStatus`.
- Symbol display: `✓` (completed), `⏸` (rest/skipped), `✕` (missed).

- [ ] **Step 1: Write test for chronological ordering and tri-state cycle**

```rust
#[test]
fn test_7_day_dates_chronological() {
    let today = NaiveDate::from_ymd_opt(2026, 10, 27).unwrap();
    let dates = calculate_7_day_dates(today);
    assert_eq!(dates[0], today - chrono::Duration::days(6));
    assert_eq!(dates[6], today);
}

#[test]
fn test_cycle_toggle_status() {
    assert_eq!(next_toggle_status(None), EntryStatus::Completed);
    assert_eq!(next_toggle_status(Some(EntryStatus::Missed)), EntryStatus::Completed);
    assert_eq!(next_toggle_status(Some(EntryStatus::Completed)), EntryStatus::Skipped);
    assert_eq!(next_toggle_status(Some(EntryStatus::Skipped)), EntryStatus::Missed);
}
```

- [ ] **Step 2: Run test to verify failure**

Run: `cargo test --test ui_row_tests`
Expected: FAIL

- [ ] **Step 3: Update `src/ui/habit_row.rs`**

- Reverse `calculate_7_day_dates` to `[today - 6, today - 5, today - 4, today - 3, today - 2, today - 1, today]`.
- Implement `next_toggle_status`.
- Update symbol text: `⏸` for `EntryStatus::Skipped`.
- Set fixed 36px width request for each day button in `days_box` (`set_size_request(36, 36)`).
- Wire primary click to cycle (`✕` -> `✓` -> `⏸` -> `✕`).
- Add secondary gesture click (right click) to toggle `⏸` directly.
- Update CSS to style `symbol-rest` with `⏸`.

- [ ] **Step 4: Run tests to verify pass**

Run: `cargo test --test ui_row_tests`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/ui/habit_row.rs tests/ui_row_tests.rs
git commit -m "feat(ui): chronological 7-day row with tri-state cycling and 36px fixed alignment"
```

---

### Task 4: Fast-Check List View, Push Navigation & Fixed Window Geometry

**Files:**
- Modify: `src/ui/window.rs:1-850`
- Test: `tests/integration_tests.rs`

**Interfaces:**
- Window geometry: `520px` width x `700px` height, `resizable = false`.
- Root navigation: `AdwNavigationView` containing `habits_page` and `details_page`.
- Header date labels: formatted as `MM/DD` (`date.format("%m/%d")`), 36px fixed width matching row buttons.
- Terminology: "Score" everywhere ("HABIT & SCORE", "Score History (Last 30 Days)", "Score").

- [ ] **Step 1: Write integration tests for window navigation and date formatting**

```rust
#[test]
fn test_header_date_formatting() {
    let d = NaiveDate::from_ymd_opt(2026, 10, 27).unwrap();
    assert_eq!(d.format("%m/%d").to_string(), "10/27");
}
```

- [ ] **Step 2: Run test to verify passes**

Run: `cargo test test_header_date_formatting`
Expected: PASS

- [ ] **Step 3: Refactor `src/ui/window.rs` to Fast-Check List & Push Navigation**

- Set window default and fixed size:
  ```rust
  window.set_default_size(520, 700);
  window.set_resizable(false);
  ```
- Replace `NavigationSplitView` with `AdwNavigationView`.
  - `habits_page`:
    - HeaderBar: Title "Habits", Add Habit button (`+`), Menu button.
    - Quick-check column header: "HABIT & SCORE" label on left, 7 date labels `MM/DD` on right with `set_size_request(36, -1)` and `set_halign(gtk4::Align::Center)` matching row buttons.
    - Habit list with `HabitRowWidget`s.
  - `details_page`:
    - HeaderBar: Back button (automatically managed by `AdwNavigationView`), title with habit name, Edit button, Delete button.
    - Detail content: Score card, Current Streak card, Best Streak card, 30-day Score trend curve, Loop-style Monthly Calendar Heatmap.
- When user clicks habit title in row: push `details_page` to `navigation_view`.
- Rename all "Strength" references to "Score".

- [ ] **Step 4: Verify full test suite passes**

Run: `cargo test --all --features ui`
Expected: 57+ tests PASS with 0 errors.

- [ ] **Step 5: Commit**

```bash
git add src/ui/window.rs
git commit -m "feat(ui): switch to Fast-Check List navigation, fixed 520x700 window, and Score terminology"
```

---

### Task 5: End-to-End Verification, Clippy Audit & Flatpak Bundle Generation

**Files:**
- Modify: `build-aux/io.github.habit.Habit.json` (if needed)
- Target: `habit.flatpak`

- [ ] **Step 1: Run clippy and audit**

Run: `cargo clippy --all --features ui -- -D warnings`
Expected: 0 warnings.

- [ ] **Step 2: Run unit and integration tests**

Run: `cargo test --all --features ui`
Expected: All tests pass.

- [ ] **Step 3: Build Flatpak bundle**

Run:
```bash
flatpak-builder --force-clean --repo=repo build-dir build-aux/io.github.habit.Habit.json
flatpak build-bundle repo habit.flatpak io.github.habit.Habit
```
Expected: `habit.flatpak` created successfully (~1 MB).

- [ ] **Step 4: Install and verify flatpak app**

Run: `flatpak install --user -y habit.flatpak`
Expected: Installed successfully.

- [ ] **Step 5: Commit final changes**

```bash
git add .
git commit -m "chore: verify tests, clippy, and build updated flatpak bundle"
```
