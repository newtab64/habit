# Lean Window & Circular Progress Ring Design

**Date:** 2026-10-28
**Size:** Small change

## Intent & Why
The current `520px` window feels wider than necessary ("fat") for a 5-day quick-check habit list, and the horizontal progress bar under each habit title differs from Loop Habit Tracker's compact inline circular score ring. Narrowing the window and switching to an inline circular ring gives the app a leaner, Loop-like proportion without changing its page structure.

## Goals, Non-Goals, and Anti-Goals
- **Goals:**
  - Reduce the application window width from `520px` to `430px` and tighten the 5 day columns from `50px` (`6px` gap) to `42px` (`4px` gap).
  - Replace the `gtk4::ProgressBar` in `HabitRowWidget` with a `20×20px` circular progress ring (`gtk4::DrawingArea` using `draw_progress_ring`) placed inline to the left of the habit title.
- **Non-Goals:**
  - Re-architecting the navigation view, details page layout, or bottom sheets.
- **Anti-Goals:**
  - Cramping habit titles or clipping the Details page overview cards / heatmap at `430px` width.

## Constraints
- Single-threaded GTK4/libadwaita architecture; no new dependencies or schema changes.
- Preserve existing GTK4 CSS parser cleanliness and test suite passing under `--features ui --all-targets -- -D warnings`.

## Approaches Considered
1. **Approach 1 (Chosen): Loop Inline Ring + Tighter Day Columns (`430px` window)**
   - Inline `20×20px` `DrawingArea` ring to the left of the habit title; day columns trimmed from `50px` (`6px` gap) to `42px` (`4px` gap), reducing total day-column width from `274px` to `226px` and window width from `520px` to `430px`.
   - Chosen because it cuts 90px of width while preserving ~180px for the ring and habit title.
2. **Approach 2: Loop Inline Ring + Keep `50px` Day Columns (`460px` window)**
   - Inline `20×20px` ring, keeping `50px` day columns and shrinking the window to `460px`.
   - Rejected because it leaves the window 30px wider while giving less room (~156px) to habit names.

## Decisions Made
- **Window dimensions:** `default_width` in `src/ui/window.rs` changes from `520` to `430` (`default_height` stays `700`, `resizable(false)`).
- **Day column sizing:** Day header labels in `src/ui/sidebar.rs` and day buttons in `src/ui/habit_row.rs` (plus `.day-symbol-button` CSS `min-width`) change from `50px` to `42px`, and horizontal spacing between day columns changes from `6px` to `4px`.
- **Habit row progress indicator:** `HabitRowWidget` replaces `progress_bar: gtk4::ProgressBar` with `progress_ring: gtk4::DrawingArea` (`20×20px`) arranged horizontally in `left_box` (`[progress_ring, title_label]` with `8px` spacing), drawn via `draw_progress_ring` using the habit's parsed `Rgba` accent color and `score`, with the percentage tooltip on `progress_ring`.

## Left to the Builder
- Removing obsolete `.habit-progress` CSS rules from `generate_row_css` and updating `tests/test_habit_row.rs` assertions accordingly.
