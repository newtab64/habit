# Overview Progress Ring, Bottom Sheet Dialog, and Audit Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:**
1. Add the circular progress ring (`scoreRing`) to the Overview card and refine the Overview card styling.
2. Present the history editor dialog as a bottom-aligned sheet (`DialogPresentationMode::BottomSheet`) using `libadwaita::Dialog`.
3. Disallow clicking on the heatmap to open the editor (user must click the pen edit icon).
4. Perform aggressive testing and verification across the codebase to uncover and eliminate any bugs.

**Architecture:**
- Implement `draw_progress_ring` in `src/ui/charts.rs`.
- Add `ring_drawing_area` to `build_overview_section()` in `src/ui/window.rs`, wired to habit score & accent color in `caches` and updated on `refresh()`.
- Refactor `show_history_editor_dialog` to use `libadwaita::Dialog` with `presentation_mode(DialogPresentationMode::BottomSheet)` and `dialog.present(Some(parent))`.
- Remove `heatmap_drawing_area.add_controller(heatmap_gesture)` in `src/ui/window.rs` so only `edit_mode_btn` launches the editor.
- Write aggressive unit tests covering `draw_progress_ring`, edge cases of habit metrics, and dialog interactions.

**Tech Stack:** Rust 2021, GTK4, Libadwaita, Cairo.

**Spec:** Direct user request: "you missing a circle in overview, add it and make the overview better looking, it fine but it can be better, and the edit pop up pop should be align bottom, and dont let user click on the heat map to edit, they have to click on the edit icon (the pen), then aggresively testing to find out bugs".

## Global Constraints

- Safe Rust only (`#![forbid(unsafe_code)]`).
- Zero GTK CSS syntax warnings or errors.
- 100% test pass rate with aggressive test coverage.
- Strictly no technical jargon in user-facing communication; concise, friendly Vietnamese.

---

### Task 1: Implement `draw_progress_ring` and Unit Tests

**Files:**
- Modify: `src/ui/charts.rs`
- Test: `tests/test_charts.rs`

- [ ] **Step 1: Write unit tests in `tests/test_charts.rs` for `draw_progress_ring`**
- [ ] **Step 2: Implement `draw_progress_ring` in `src/ui/charts.rs`**
- [ ] **Step 3: Run `cargo test --test test_charts` and ensure all tests pass**
- [ ] **Step 4: Commit changes**
  - `git commit -am "feat(charts): add draw_progress_ring with unit tests"`

---

### Task 2: Add RingView to Overview Card and Refine Overview Styling

**Files:**
- Modify: `src/ui/window.rs:325-345`, `src/ui/window.rs:760-840`, `src/ui/window.rs:1350-1370`, `src/ui/window.rs:550-600`

- [ ] **Step 1: Add `overview_score` and `overview_color` to `DrawCaches`**
- [ ] **Step 2: Add `ring_drawing_area` in `build_overview_section` alongside Score, Month, Year, Total**
- [ ] **Step 3: Connect draw function to `ring_drawing_area` using `draw_progress_ring`**
- [ ] **Step 4: Update `refresh()` to queue draw on `ring_drawing_area` with current score and accent color**
- [ ] **Step 5: Run `cargo check --all --features ui` and `cargo test --all --features ui`**
- [ ] **Step 6: Commit changes**
  - `git commit -am "feat(ui): add progress ring to Overview card and refine layout"`

---

### Task 3: Align History Editor Popup to Bottom and Restrict Editing to Pen Button

**Files:**
- Modify: `src/ui/window.rs:960-1060`, `src/ui/window.rs:1230-1260`

- [ ] **Step 1: Convert `show_history_editor_dialog` to `libadwaita::Dialog` with `DialogPresentationMode::BottomSheet`**
- [ ] **Step 2: Remove click gesture controller from `details.heatmap_drawing_area`**
- [ ] **Step 3: Run `cargo check --all --features ui` and `cargo test --all --features ui`**
- [ ] **Step 4: Commit changes**
  - `git commit -am "feat(ui): align editor dialog to bottom and restrict trigger to edit button"`

---

### Task 4: Aggressive Testing & Edge Case Bug Hunting

**Files:**
- Test: `tests/test_window.rs`, `tests/test_scoring.rs`, `tests/test_charts.rs`

- [ ] **Step 1: Add tests for edge cases in metrics and streak calculation**
  - Habits with 0 entries, empty DB, single day entries
  - Measurable habits with various target completions
  - Clamping and negative/positive bounds in progress ring and diffs
- [ ] **Step 2: Run full clippy with strict warnings: `cargo clippy --all --features ui -- -D warnings`**
- [ ] **Step 3: Run full test suite: `cargo test --all --features ui`**
- [ ] **Step 4: Commit changes**
  - `git commit -am "test: aggressively verify all edge cases for overview and editor"`

---

### Task 5: Build & Reinstall Flatpak Bundle & Push to Main

**Files:**
- Target: `habit.flatpak`

- [ ] **Step 1: Run flatpak-builder and reinstall habit.flatpak**
- [ ] **Step 2: Push commits to origin main**
