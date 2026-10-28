# Habit Reorder Lock & Drag Mode Design

**Date:** 2026-10-28
**Size:** Small change

## Intent & Why
Users want to place priority habits at the top of the list without cluttering daily logging or risking accidental drags. A top-left lock button in the header bar keeps the habit list locked during normal use and unlocks a dedicated drag-to-reorder mode with a confirmation dialog when re-locking.

## Goals, Non-Goals, and Anti-Goals
- **Goals:**
  - Persist custom habit ordering in SQLite (`sort_order` column on `habits`) with automatic idempotent migration for existing `.db` files and restored backups.
  - Add a top-left lock button (`changes-prevent-symbolic` / `changes-allow-symbolic`) to `sidebar_header`.
  - When unlocked (Drag Mode), display a drag handle (`list-drag-handle-symbolic`) on each habit row, disable the 5 day tickboxes and Details navigation, and allow dragging habits up and down via GTK4 `DragSource` + `DropTarget`.
  - When clicking the lock button again to finish reordering, present an `adw::AlertDialog` ("Save Habit Order?") to **Save** (persist `sort_order` in a single transaction) or **Discard** (revert to saved order) before locking back.
- **Non-Goals:**
  - Multi-select dragging or keyboard-shortcut reordering.
- **Anti-Goals:**
  - Allowing accidental day-entry toggles or Details page navigation while in Drag Mode.

## Constraints
- Single-threaded GTK4/libadwaita architecture; no new dependencies.
- Backward-compatible SQLite migration so existing user databases and imported backups without `sort_order` automatically upgrade on open/restore.
- Zero warnings under `cargo clippy --features ui --all-targets -- -D warnings`.

## Approaches Considered
1. **Approach 1 (Chosen): GTK4 Native Drag-and-Drop (`gtk4::DragSource` + `gtk4::DropTarget`)**
   - Uses native GTK4 drag-and-drop controllers carrying the `habit_id` (`i64`) to reorder the staged `Vec<i64>` and refresh the list with drop-target hover styling.
   - Chosen for reliable Wayland cursor feedback, hover highlighting, and scroll handling.
2. **Approach 2: Raw Pointer Y-Delta Tracking (`gtk4::GestureDrag`)**
   - Tracks vertical pixel movement and swaps widgets at fixed height thresholds.
   - Rejected due to jumpiness across multiple rows and manual coordinate recalculation during scrolling.

## Decisions Made
- **Database:** `habits` table adds `sort_order INTEGER NOT NULL DEFAULT 0` (applied via `CREATE TABLE` and idempotent `ALTER TABLE` migration in `init_tables` and `restore_backup`). `get_habits()` orders by `sort_order ASC, id ASC`. `Database::reorder_habits(&self, ordered_ids: &[i64]) -> Result<(), DbError>` updates `sort_order` in a single transaction.
- **Header UI:** `SidebarWidgets` adds `lock_btn: gtk4::Button` packed at the start of `sidebar_header` (`changes-prevent-symbolic` when locked, `changes-allow-symbolic` when unlocked).
- **Drag Mode State:** `WindowInner` tracks `reorder_mode: bool` and `staged_habit_order: Vec<i64>` (held in `DrawCaches` or `WindowInner`). In Drag Mode, `add_btn` is disabled, each `HabitRowWidget` shows a leading `list-drag-handle-symbolic` icon, disables `days_box` (`set_sensitive(false)`), skips attaching the Details click gesture, and attaches `DragSource` + `DropTarget`.
- **Confirmation Dialog:** Clicking `lock_btn` while in Drag Mode opens `adw::AlertDialog` ("Save Habit Order?", responses `"discard"` / `"save"`). Choosing `"save"` calls `db.reorder_habits(&staged_order)` and exits Drag Mode; choosing `"discard"` clears `staged_order` and exits Drag Mode.

## Left to the Builder
- CSS styling for `.drag-handle` and `.drop-target-hover` feedback during drag operations.
