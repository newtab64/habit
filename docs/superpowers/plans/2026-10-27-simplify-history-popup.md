# Simplify History Popup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Streamline the history editor popup dialog to match the minimalist design of Loop Habit Tracker by removing redundant header/navigation buttons, legends, and subtitles, leaving only the clean heatmap grid with intuitive mouse/touch scroll support.

**Architecture:** Refactor `show_history_editor_dialog` in `src/ui/window.rs` to strip away the secondary navigation bar (`nav_row`), "Done" action button, subtitle, and legend caption. Add an `EventControllerScroll` to `drawing_area` for fluid touchpad/mouse-wheel week navigation.

**Tech Stack:** Rust 2021, GTK4, Libadwaita, Cairo.

**Spec:** Direct user request: "make the pop up app even simpler" / "yeah, make it like loop".

## Global Constraints

- Safe Rust only (`#![forbid(unsafe_code)]`).
- Zero GTK CSS syntax warnings or errors.
- Preserve all existing unit and integration tests (100% pass rate).
- Strictly no technical jargon in user-facing communication; concise, friendly Vietnamese.

## Review Focus

- Header bar cleanliness: only shows habit name with standard window controls (no redundant "Done" button or subtitle).
- Content simplicity: no navigation button rows or status legends cluttering the dialog.
- Smooth navigation: mouse wheel / touchpad scroll cleanly adjusts `week_offset` (clamped between 0 and 104) and triggers redraw.
- Direct click editing: clicking any date cell continues to toggle state properly and immediately sync with SQLite DB and main view.

---

### Task 1: Streamline `show_history_editor_dialog` to Minimal Heatmap View

**Files:**
- Modify: `src/ui/window.rs:880-1060`
- Test: `tests/test_window.rs`

- [ ] **Step 1: Edit `show_history_editor_dialog` in `src/ui/window.rs`**
  - Remove `done_btn` and header subtitle ("Click day to toggle status").
  - Remove `nav_row`, `prev_btn`, `today_btn`, `next_btn`, and `hint_lbl`.
  - Add `EventControllerScroll` to `drawing_area` for trackpad / mouse scroll week navigation.
  - Set clean compact dialog dimensions (500x340).
- [ ] **Step 2: Run cargo check & clippy**
  - Run `cargo check --all --features ui`
  - Run `cargo clippy --all --features ui -- -D warnings`
- [ ] **Step 3: Run existing test suite**
  - Run `cargo test --all --features ui`
- [ ] **Step 4: Commit changes**
  - `git commit -am "refactor(ui): streamline history editor dialog to minimalist heatmap"`

---

### Task 2: Build & Reinstall Flatpak Bundle

**Files:**
- Target: `habit.flatpak`

- [ ] **Step 1: Build and reinstall Flatpak**
  - Run `flatpak-builder --force-clean --repo=repo build-dir build-aux/io.github.habit.Habit.json && flatpak build-bundle repo habit.flatpak io.github.habit.Habit && flatpak install --user -y --reinstall habit.flatpak`
- [ ] **Step 2: Verify git status and push to remote**
  - Run `git push origin main`
