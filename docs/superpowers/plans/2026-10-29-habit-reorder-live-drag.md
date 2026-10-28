# Habit Reorder Live Drag & Unchanged-Order Detection Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace static DND habit reordering with live pointer-tracked row movement (`gtk4::GestureDrag`) and skip the "Save Habit Order?" confirmation dialog when locking if the order did not change.

**Architecture:** We store `initial_habit_order` alongside `staged_habit_order` in `DrawCaches` when entering reorder mode and compare them on lock click to bypass `libadwaita::AlertDialog` when equal. For dragging, each row in reorder mode attaches a `gtk4::GestureDrag` controller that translates pointer coordinates to `habits_list_box`, checks whether the pointer crossed a sibling row's vertical midpoint via a pure helper `target_index_for_drag_y`, and swaps both `staged_habit_order` and the GTK widget child order (`habits_list_box.reorder_child_after`) in-place without rebuilding the widget list mid-drag.

**Tech Stack:** Rust (2021 edition), GTK4 (`gtk4`), Libadwaita (`libadwaita`)

**Spec:** [`docs/superpowers/specs/2026-10-28-habit-reorder-lock-design.md`](file:///home/user/Downloads/Repos/habit/docs/superpowers/specs/2026-10-28-habit-reorder-lock-design.md)

## Global Constraints

- Strictly single-threaded GTK4 architecture (`Rc`/`RefCell`).
- Never hold a `RefCell` borrow across GTK widget/signal calls.
- Zero warnings under `cargo clippy --features ui --all-targets -- -D warnings`.
- No new crate dependencies and no `#[allow(...)]` attributes.

## Review Focus

- Unlocking and immediately locking without dragging any row: must exit reorder mode immediately with no confirmation dialog.
- Dragging a row to another position and then dragging it back to its original slot before locking: `staged_habit_order == initial_habit_order`, so locking must also skip the confirmation dialog.
- Dragging above the top edge (`y < 0`) or below the bottom edge of the list: `target_index_for_drag_y` clamps cleanly to the first (`0`) or last (`len - 1`) row index without out-of-bounds panics.
- Moving a row to index `0` (top of list) vs index `> 0`: `habits_list_box.reorder_child_after(row, None)` places it at the front, whereas `reorder_child_after(row, Some(&prev_sibling))` places it after `index - 1`.
- Releasing the drag gesture (`drag_end`): `.dragging-row` CSS highlight is always removed even if no swap occurred.

---

### Task 1: Pure Midpoint Hit-Testing Helper, Live `GestureDrag` Row Movement, and Unchanged-Order Detection

**Files:**
- Modify: `src/ui/habit_row.rs:435-495`
- Modify: `src/ui/state.rs:19-57,173-186,294-312`
- Modify: `src/ui/window.rs:29-94`
- Test: `tests/test_habit_row.rs`

**Interfaces:**
- Produces:
  - `pub fn target_index_for_drag_y(row_spans: &[(f64, f64)], current_idx: usize, pointer_y: f64) -> usize` in `src/ui/habit_row.rs`
  - `pub fn is_habit_order_unchanged(initial: &[i64], staged: &[i64]) -> bool` in `src/ui/habit_row.rs`
  - `HabitRowWidget::enable_reorder_drag(&self, habit_id: i64, list_box: &gtk4::Box, staged_order: &Rc<RefCell<Vec<i64>>>)` in `src/ui/habit_row.rs`
  - `DrawCaches::initial_habit_order: Rc<RefCell<Vec<i64>>>` in `src/ui/state.rs`

- [ ] **Step 1: Write failing unit tests in `tests/test_habit_row.rs`**

Add unit tests for `target_index_for_drag_y` and `is_habit_order_unchanged` at the end of `tests/test_habit_row.rs`:

```rust
#[test]
fn test_target_index_for_drag_y_midpoint_crossing() {
    // 4 rows of height 46px with 6px spacing: (top, height)
    let spans = vec![
        (0.0, 46.0),   // idx 0: midpoint 23.0
        (52.0, 46.0),  // idx 1: midpoint 75.0
        (104.0, 46.0), // idx 2: midpoint 127.0
        (156.0, 46.0), // idx 3: midpoint 179.0
    ];

    // Dragging row 1 downward: before row 2 midpoint (127.0) stays at 1; past 127.0 moves to 2
    assert_eq!(
        habit::ui::habit_row::target_index_for_drag_y(&spans, 1, 120.0),
        1
    );
    assert_eq!(
        habit::ui::habit_row::target_index_for_drag_y(&spans, 1, 130.0),
        2
    );

    // Dragging row 2 upward: above row 1 midpoint (75.0) moves to 1; above row 0 midpoint (23.0) moves to 0
    assert_eq!(
        habit::ui::habit_row::target_index_for_drag_y(&spans, 2, 70.0),
        1
    );
    assert_eq!(
        habit::ui::habit_row::target_index_for_drag_y(&spans, 2, -15.0),
        0
    );

    // Dragging past bottom edge clamps to last index (3)
    assert_eq!(
        habit::ui::habit_row::target_index_for_drag_y(&spans, 0, 500.0),
        3
    );
}

#[test]
fn test_is_habit_order_unchanged() {
    assert!(habit::ui::habit_row::is_habit_order_unchanged(
        &[1, 2, 3],
        &[1, 2, 3]
    ));
    assert!(!habit::ui::habit_row::is_habit_order_unchanged(
        &[1, 2, 3],
        &[2, 1, 3]
    ));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --features ui --test test_habit_row`
Expected: FAIL with unresolved functions `target_index_for_drag_y` and `is_habit_order_unchanged`.

- [ ] **Step 3: Implement live `GestureDrag` reordering and unchanged-order check**

1. In `src/ui/habit_row.rs`:
   - Add `pub fn is_habit_order_unchanged(initial: &[i64], staged: &[i64]) -> bool` returning `initial == staged`.
   - Add `pub fn target_index_for_drag_y(row_spans: &[(f64, f64)], current_idx: usize, pointer_y: f64) -> usize`:
     - If `row_spans.is_empty()`, return `0`.
     - Clamp `current_idx` to `row_spans.len() - 1`.
     - Count how many rows have their midpoint `top + height * 0.5` strictly above `pointer_y` when moving down (`i > current_idx && pointer_y > midpoint`) or below `pointer_y` when moving up (`i < current_idx && pointer_y < midpoint`), returning the resulting target index in `0..row_spans.len()`.
   - Replace `HabitRowWidget::enable_reorder_drag` so it takes `(&self, habit_id: i64, list_box: &gtk4::Box, staged_order: & std::rc::Rc<std::cell::RefCell<Vec<i64>>>)`:
     - Prepends `list-drag-handle-symbolic` (`drag-handle`, `dim-label`) to `self.left_box` and sets `self.container.set_cursor_from_name(Some("grab"))`.
     - Disables `self.days_box.set_sensitive(false)`.
     - Attaches `gtk4::GestureDrag` to `self.container`:
       - `connect_drag_begin`: records the starting `(x, y)` in `list_box` coordinates using `row.compute_point(&list_box, &gtk4::graphene::Point::new(x as f32, y as f32))` and adds CSS class `"dragging-row"` to `row`.
       - `connect_drag_update`: computes current pointer Y in `list_box` coordinates (`start_list_y + offset_y`), collects `row_spans: Vec<(f64, f64)>` and `children: Vec<gtk4::Widget>` by iterating `list_box.first_child()` / `next_sibling()` and querying `child.compute_bounds(&list_box)`, finds `current_idx` of `habit_id` in `staged_order`, calls `target_index_for_drag_y(&row_spans, current_idx, pointer_y)`, and if `target_idx != current_idx`, updates `staged_order` via `move_habit_in_order` and reorders `row` in `list_box` via `list_box.reorder_child_after(&row, prev_sibling)`.
       - `connect_drag_end`: removes `"dragging-row"` CSS class from `row`.

2. In `src/ui/state.rs`:
   - Add `pub initial_habit_order: Rc<RefCell<Vec<i64>>>` to `DrawCaches` and initialize it in `DrawCaches::new`.
   - Replace `.drop-target-hover` in `update_css` with `.dragging-row { background-color: rgba(53, 132, 228, 0.18); border-radius: 8px; }`.
   - In `rebuild_habit_list`, call `row_widget.enable_reorder_drag(habit_id, &inner.habits_list_box, &caches.staged_habit_order)` when `is_reorder` is `true`.

3. In `src/ui/window.rs`:
   - In `connect_lock_button`:
     - When entering reorder mode (`!is_reorder`), set both `*caches_lock.initial_habit_order.borrow_mut() = ids.clone()` and `*caches_lock.staged_habit_order.borrow_mut() = ids`.
     - When exiting reorder mode (`is_reorder`), check `crate::ui::habit_row::is_habit_order_unchanged(&caches_lock.initial_habit_order.borrow(), &caches_lock.staged_habit_order.borrow())`. If `true`, set `*caches_lock.reorder_mode.borrow_mut() = false`, clear both `initial_habit_order` and `staged_habit_order`, call `refresh(&inner_lock, &caches_lock)`, and return without showing `AlertDialog`.
     - Also clear `initial_habit_order` in the `"save"` and `"discard"` alert response handlers.

- [ ] **Step 4: Run formatting, lints, and tests**

Run: `cargo fmt && cargo clippy --features ui --all-targets -- -D warnings && cargo test --features ui`
Expected: PASS with 0 warnings and all tests passing.

- [ ] **Step 5: Commit**

```bash
git add docs/superpowers/plans/2026-10-29-habit-reorder-live-drag.md src/ui/habit_row.rs src/ui/state.rs src/ui/window.rs tests/test_habit_row.rs
git commit -m "feat(ui): add live gesture-drag habit reordering and skip confirm when unchanged"
```
