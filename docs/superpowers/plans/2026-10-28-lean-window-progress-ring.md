# Lean Window & Circular Progress Ring Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Narrow the application window from `520px` to `430px` with `42px` day columns and replace the horizontal `gtk4::ProgressBar` in each habit row with an inline `20×20px` circular progress ring.

**Architecture:** `HabitRowWidget` in `src/ui/habit_row.rs` switches its left pane from a vertical stack (`title_label` + `gtk4::ProgressBar`) to a horizontal row (`20×20px` `gtk4::DrawingArea` drawn by `draw_progress_ring` + `title_label`), while `generate_row_css` drops obsolete `.habit-progress` rules and updates `.day-symbol-button` `min-width` to `42px`. `src/ui/sidebar.rs` and `src/ui/window.rs` align column widths (`42px` with `4px` spacing) and window default width (`430px`).

**Tech Stack:** Rust, GTK4, libadwaita, Cairo (`cairo-rs`)

**Spec:** `docs/superpowers/specs/2026-10-28-lean-window-progress-ring-design.md`

## Global Constraints

- Single-threaded GTK4/libadwaita architecture; no new dependencies or database schema changes.
- Zero warnings under `cargo clippy --features ui --all-targets -- -D warnings`.
- Preserve GTK4 CSS parser cleanliness (`test_css_parsing_cleanliness`).

## Review Focus

- Invalid or unprefixed hex color passed to `HabitRowWidget::new` (e.g. `"3584E4"` or malformed): `Rgba::from_hex` fallback (`Rgba::new(0.21, 0.52, 0.89, 1.0)`) ensures the ring still renders cleanly without panicking.
- Out-of-range or `NaN` score passed to `HabitRowWidget::new`: `draw_progress_ring` and tooltip calculation clamp finite scores to `[0.0, 1.0]` and treat `NaN` safely.

---

### Task 1: Replace Habit Row Progress Bar with Inline Progress Ring and Update Row Sizing

**Files:**
- Modify: `src/ui/habit_row.rs:143-412`
- Modify: `src/ui/sidebar.rs:55-83`
- Modify: `src/ui/window.rs:241-247`
- Test: `tests/test_habit_row.rs:277-318`

**Interfaces:**
- Consumes: `crate::ui::charts::{draw_progress_ring, Rgba}`
- Produces:
  - `HabitRowWidget { container: gtk4::Box, left_box: gtk4::Box, title_label: gtk4::Label, progress_ring: gtk4::DrawingArea, days_box: gtk4::Box, day_buttons: Vec<gtk4::Button> }`
  - `generate_row_css(habit_id: i64, hex_color: &str) -> String` (with `min-width: 42px;` and no `.habit-progress` selectors)

- [ ] **Step 1: Update the CSS test in `tests/test_habit_row.rs`**

```rust
#[test]
fn test_generate_row_css_structure_and_styling() {
    let css = generate_row_css(42, "#3584E4");

    // Scoped row selector
    assert!(
        css.contains(".habit-row-42"),
        "Must scope styles to habit row"
    );

    // Accent color applied to symbols
    assert!(css.contains(".symbol-completed"));
    assert!(css.contains(".symbol-rest"));
    assert!(css.contains("#3584E4"));

    // Muted color applied to missed and partial symbols
    assert!(css.contains(".symbol-missed"));
    assert!(css.contains(".symbol-partial"));

    // Day button width is 42px and obsolete progress bar rules are absent
    assert!(css.contains("min-width: 42px;"));
    assert!(!css.contains(".habit-progress"));

    // CSS syntax purity for GTK4 (no !important, no width/height properties)
    assert!(
        !css.contains("!important"),
        "GTK4 CSS parser rejects !important as junk at end of value"
    );
    assert!(
        !css.contains(" height:"),
        "GTK4 CSS has no 'height' property; must use min-height"
    );
    assert!(
        !css.contains(" width:"),
        "GTK4 CSS has no 'width' property; must use min-width"
    );

    // Test normalization when passed without leading '#'
    let css_no_hash = generate_row_css(10, "FF7800");
    assert!(css_no_hash.contains(".habit-row-10"));
    assert!(css_no_hash.contains("#FF7800"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --features ui --test test_habit_row test_generate_row_css_structure_and_styling`
Expected: FAIL (`css.contains("min-width: 42px;")` is false)

- [ ] **Step 3: Update `src/ui/habit_row.rs`, `src/ui/sidebar.rs`, and `src/ui/window.rs`**

In `src/ui/habit_row.rs`:
- Remove `.habit-progress` rules from `generate_row_css` and change `min-width: 50px;` to `min-width: 42px;`.
- Replace `pub progress_bar: gtk4::ProgressBar` in `HabitRowWidget` with `pub progress_ring: gtk4::DrawingArea`.
- Update `create_left_pane(name: &str, strength: f64, accent_color: &str, habit_id: i64, on_select: F) -> (gtk4::Box, gtk4::Label, gtk4::DrawingArea)` to use a horizontal `gtk4::Box` (`spacing = 8`), a `20×20` `gtk4::DrawingArea` with `set_draw_func` calling `crate::ui::charts::draw_progress_ring` using `Rgba::from_hex(accent_color).unwrap_or(Rgba::new(0.21, 0.52, 0.89, 1.0))`, and tooltip `format!("{pct:.0}%")` where `pct = (strength.clamp(0.0, 1.0) * 100.0).round()` (defaulting non-finite `strength` to `0.0`).
- In `HabitRowWidget::new`, use `4px` spacing in `days_box` and `day_btn.set_size_request(42, 46)`.

In `src/ui/sidebar.rs`:
- Change `quick_check_header` and `days_header_box` spacing from `6` to `4`, and `lbl.set_size_request(50, -1)` to `lbl.set_size_request(42, -1)`.

In `src/ui/window.rs`:
- Change `.default_width(520)` to `.default_width(430)`.

- [ ] **Step 4: Run formatting, clippy, and full test suite to verify**

Run: `cargo fmt && cargo clippy --features ui --all-targets -- -D warnings && cargo test --features ui`
Expected: PASS with 0 warnings and all tests passing.

- [ ] **Step 5: Commit**

```bash
git add src/ui/habit_row.rs src/ui/sidebar.rs src/ui/window.rs tests/test_habit_row.rs
git commit -m "feat(ui): narrow window to 430px and replace row progress bar with circular ring"
```
