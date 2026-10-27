# Loop Habit Detail Overview Stats Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the 4-stat Overview Card at the top of habit details exactly like Loop Habit Tracker (`Score`, `Month`, `Year`, `Total`), replacing the previous 3-box metrics section.

**Architecture:**
- Extend `HabitMetrics` in `src/ui/window.rs` with `month_diff`, `month_diff_text`, `year_diff`, `year_diff_text`, `total_count`, and `total_count_text`.
- Update `calculate_habit_metrics` to compute 30-day score diff, 365-day score diff, and total completed entries count matching Loop Habit Tracker's `OverviewCardPresenter`.
- Replace `build_metrics_section()` with `build_overview_section()` in `src/ui/window.rs`, providing an Adw/GTK Card with a header title "Overview" and 4 homogeneous columns for Score, Month, Year, and Total.
- Update `refresh()` to populate the 4 stats and apply accent styling.
- Update existing unit tests in `tests/test_window.rs` and add new test cases for 4 stats calculation and percentage diff formatting.

**Tech Stack:** Rust 2021, GTK4, Libadwaita.

**Spec:** User request: "then make the habit detail basic stats in the top of habit details like loop habit tracker" / "just make it like loop habit tracker, 4 stats".

## Global Constraints

- Safe Rust only (`#![forbid(unsafe_code)]`).
- Zero GTK CSS syntax warnings or errors.
- Preserve all existing unit and integration tests (100% pass rate).
- Strictly no technical jargon in user-facing communication; concise, friendly Vietnamese.

## Review Focus

- Calculation accuracy: 30-day diff compares score today against `as_of - 30 days`; 365-day diff compares score today against `as_of - 365 days`.
- Diff formatting: positive diffs prefixed with `+` (e.g. `+5%`, `+0%`), negative diffs with `-` (e.g. `-2%`).
- Total count: counts all completed entries on or before `as_of`.
- Layout: 4 columns nicely balanced across the card width with values on top and labels below.

---

### Task 1: Update `HabitMetrics` and `calculate_habit_metrics` with 4 Overview Stats

**Files:**
- Modify: `src/ui/window.rs:190-235`
- Test: `tests/test_window.rs`

- [ ] **Step 1: Write failing unit test in `tests/test_window.rs` for 4 stats calculation and diff formatting**
- [ ] **Step 2: Run `cargo test --test test_window` to watch it fail**
- [ ] **Step 3: Implement `format_percentage_diff` and update `HabitMetrics` and `calculate_habit_metrics`**
- [ ] **Step 4: Run `cargo test --test test_window` to ensure all tests pass**
- [ ] **Step 5: Commit changes**
  - `git commit -am "feat(engine): add 30-day diff, 365-day diff, and total count to habit metrics"`

---

### Task 2: Build Overview Section Card UI and Wire into `refresh()`

**Files:**
- Modify: `src/ui/window.rs:310-375`, `src/ui/window.rs:690-720`, `src/ui/window.rs:790-870`, `src/ui/window.rs:510-550`

- [ ] **Step 1: Replace `build_metrics_section()` with `build_overview_section()`**
  - Construct card container with "Overview" heading.
  - Construct 4 stat columns: Score, Month, Year, Total.
  - Update `DetailsWidgets` and `WindowInner` fields to store the 4 stat labels.
- [ ] **Step 2: Update `refresh()` to set text and accent styling for the 4 stat labels**
- [ ] **Step 3: Run `cargo check --all --features ui` and `cargo clippy --all --features ui -- -D warnings`**
- [ ] **Step 4: Run `cargo test --all --features ui`**
- [ ] **Step 5: Commit changes**
  - `git commit -am "feat(ui): display 4-stat Overview Card at top of habit details"`

---

### Task 3: Build & Reinstall Flatpak Bundle & Final Push

**Files:**
- Target: `habit.flatpak`

- [ ] **Step 1: Build and reinstall Flatpak**
  - Run `flatpak-builder --force-clean --repo=repo build-dir build-aux/io.github.habit.Habit.json && flatpak build-bundle repo habit.flatpak io.github.habit.Habit && flatpak install --user -y --reinstall habit.flatpak`
- [ ] **Step 2: Verify git status and push to remote `main`**
  - Run `git push origin main`
