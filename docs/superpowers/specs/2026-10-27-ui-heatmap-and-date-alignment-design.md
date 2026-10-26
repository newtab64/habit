# Design Spec: Fast-Check List & Loop Tracker Polish

## 1. Intent & Why
Refocus the Linux Habit tracker around the core mobile Loop Habit Tracker experience:
- Provide a dedicated **Fast-Check List** as the primary window view (clean list of habits with 7-day quick-action buttons and score progress bars).
- Clicking a habit title transitions into the **Habit Details** view (Score History curve, Loop-style monthly calendar heatmap, streak analytics, edit/delete) with a standard Back button navigation.
- Replace the word "strength" with **"Score"** across all UI labels and cards.
- Recalibrate the exponential decay algorithm so that habit score is **harder to gain** (requiring sustained consistency, $T_{\text{gain}} = 28$ days vs $T_{\text{loss}} = 14$ days).
- Align date headers (`"MM/DD"`, left-to-right chronological) 1:1 with tick buttons and support quick toggle for Completed (`✓`), Rest (`⏸`), and Missed (`✕`).

## 2. Goals & Anti-Goals
- **Goals:**
  - Fast-Check main view: Full-window or clamped single-view list of habits for zero-distraction daily check-ins.
  - Smooth navigation: `AdwNavigationView` or `AdwNavigationSplitView` collapsed to push/pop between "Habits" and "Details".
  - Alignment: 7 date columns formatted as `MM/DD` (chronological from `today - 6` to `today` on the right), with strict horizontal alignment between header and rows.
  - Multi-state toggling: Quick left-click cycling (Missed `✕` -> Completed `✓` -> Rest `⏸` -> Missed `✕`) or right-click to set Rest (`⏸`) directly.
  - Heatmap (Loop Style): Centered square tiles with rounded corners (radius 4px), weekday headers (`M T W T F S S`), day numbers (`1..31`), vibrant fill for completed days, soft neutral fill for unlogged past days.
  - Score terminology: Use "Score" everywhere ("Habit Score", "Score (Last 30 Days)").
  - Harder score gain: Asymmetric decay where gaining score has a 28-day half-life while losing has a 14-day half-life.
- **Anti-Goals:**
  - Do not keep an always-visible empty right pane on desktop screens.
  - Do not introduce external dependencies.

## 3. Approaches Considered
- **Approach 1: Always-visible split screen (Previous)**
  - *Tradeoffs:* Left half list, right half details. Cluttered on standard screens, right side often shows empty placeholder. Rejected.
- **Approach 2: Fast-Check List with Navigation View (Selected)**
  - *Tradeoffs:* Main screen is 100% focused on daily check-ins. Tapping habit title smoothly opens detailed analytics with a Back button. Matches Loop Habit Tracker UX.

## 4. Implementation Details
### A. Algorithm Calibration (`src/engine/scoring.rs`)
- Introduce asymmetric half-life:
  - `GAIN_HALF_LIFE_DAYS = 28.0` ($\alpha_{\text{gain}} \approx 0.0244$)
  - `LOSS_HALF_LIFE_DAYS = 14.0` ($\alpha_{\text{loss}} \approx 0.0483$)
- On completed day: $S_t = S_{t-1} + (1.0 - S_{t-1}) \cdot \alpha_{\text{gain}}$
- On missed day: $S_t = S_{t-1} \cdot (1.0 - \alpha_{\text{loss}})$
- On rest / skipped day: $S_t = S_{t-1}$ (neutral).

### B. View Structure & Window Geometry (`src/ui/window.rs`)
- Lock window size at an optimal compact window geometry: default & fixed size `520px` width x `700px` height.
- Set `window.set_resizable(false)` and disable maximization so the app stays locked at this optimal compact utility size without stretching across a full screen.
- Use `AdwNavigationView` with two `AdwNavigationPage`s:
  - Page 1 (`habits_page`): Header bar with "Habits", Add button (`+`), and Menu button. Main body has date headers (`MM/DD`) and habit rows.
  - Page 2 (`details_page`): Header bar with Back button, habit title, Edit, and Delete. Content has Score card, Streaks, 30-day Score trend curve, and Loop-style Monthly Calendar Heatmap.
- Clicking a habit's title pushes `details_page` onto the navigation view. Back button pops back to `habits_page`.

### C. Quick-Check Alignment & Toggling (`src/ui/habit_row.rs`)
- Header and rows use 36px fixed-width column cells.
- Dates: `[today - 6, today - 5, today - 4, today - 3, today - 2, today - 1, today]`, formatted as `MM/DD`.
- Toggling cycles: `Missed` -> `Completed` (`✓`) -> `Skipped` (`⏸`) -> `Missed` (`✕`).
- Secondary click (right click) on binary habit directly sets `Skipped` (`⏸`).

### D. Loop-Style Heatmap (`src/ui/charts.rs`)
- Grid: 7 columns (Mon..Sun) x up to 6 rows.
- Square cells: `cell_size = min((avail_w - 6 * gap) / 7, (avail_h - header_h - 5 * gap) / 6).clamp(16.0, 36.0)`.
- Centered horizontally within the drawing area.
- Weekday labels row at top: `M`, `T`, `W`, `T`, `F`, `S`, `S`.
- Day numbers (`1..31`) drawn centered in each cell.
- Rounded rectangle tiles: completed (accent fill + white text), rest (muted accent + `—` + text), missed (soft neutral `#2e2e2e` in dark / `#e0e0e0` in light), future (faint outline).
- `heatmap_hit_test` updated with the centered geometry.

### E. Terminology Audit
- Rename "Strength" -> "Score" throughout all UI strings, labels, and card headers.

## 5. Verification Plan
- Unit tests in `tests/` and `src/engine/scoring.rs` updated for new gain rate and verified.
- `cargo test --all` and `cargo test --features ui`.
- `flatpak-builder` and `habit.flatpak` bundle generation.
