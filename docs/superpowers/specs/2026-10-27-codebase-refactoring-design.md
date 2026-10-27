# Design Document: Codebase Modularization & UI Component Decomposition

- **Date:** 2026-10-27
- **Topic:** Codebase Refactoring & UI Component Decomposition
- **Status:** Proposed
- **Author:** Antigravity & User

---

## 1. Intent & Context

The Habit application is a native GNOME/Adwaita habit tracking utility written in Rust (~4,900 lines total). While the core engine (`src/engine/`: SQLite persistence, data models, exponential decay scoring) is already cleanly modularized into separate files (~1,100 lines), the user interface layer has grown organically into large monolithic files:

- **`src/ui/window.rs` (1,794 lines):** Currently houses domain metric calculations (`HabitMetrics`, string formatters, toggle rules), state management (`WindowInner`, `DrawCaches`, refresh orchestration), the sidebar list view, the habit details screen (overview card, stats, charts), and the bottom-sheet history editor overlay.
- **`src/ui/charts.rs` (825 lines):** Bundles fundamental 2D geometry math, Cairo color parsing, monthly calendar layout, score curve rendering, progress rings, and the continuous rolling heatmap engine.

This coupling increases cognitive load when maintaining code, makes unit testing UI-adjacent logic cumbersome, and increases the likelihood of regressions when updating individual views.

The goal of this refactoring is to divide the monolithic files into cohesive, single-responsibility components with clean dependency boundaries, while preserving exact existing behaviors, visual aesthetics, performance, and 100% test pass rates across all 81 automated tests.

---

## 2. Goals, Non-Goals, and Anti-Goals

### Goals
- **Decompose `src/ui/window.rs`** into dedicated, focused modules under `src/ui/`:
  - `metrics.rs`: Pure calculations, metric structs, and display formatters (100% decoupling from GTK).
  - `state.rs`: Application state, draw caches, database connection management, and central refresh orchestration.
  - `sidebar.rs`: Main habit list view, fast-check 5-day / 7-day day headers, add habit button, and sidebar container.
  - `details.rs`: Habit details navigation page, overview card (progress ring + 4 metrics columns), and section containers.
  - `history_editor.rs`: Bottom sheet history editor overlay, gestures, week scrolling, and dismissal logic.
  - `window.rs`: Minimized root coordinator (~150 lines) managing `HabitWindow`, `NavigationView`, primary hamburger menu, and app initialization.
- **Modularize `src/ui/charts.rs`** into a clean directory package `src/ui/charts/`:
  - `geometry.rs`: Primitives (`Point`, `Rect`, `Rgba`).
  - `progress_ring.rs`: Circular score completion ring.
  - `curve.rs`: 30-day score history curve and point calculation.
  - `heatmap.rs`: Continuous rolling heatmap layout geometry, hit testing, and Cairo rendering.
  - `calendar.rs`: Monthly calendar layout and rendering.
  - `mod.rs`: Re-exports maintaining backwards compatibility.
- **Maintain Re-export Facades:** Ensure all 81 existing automated tests (which import items like `calculate_habit_metrics`, `toggle_entry_state`, `draw_loop_heatmap` from `crate::ui::window` or `crate::ui::charts`) compile and pass without regressions.
- **Preserve All Runtime Behaviors:** Maintain exact 520x700 locked window size, 20px side margins on sheets, 180ms bottom sheet animations, and instant navigation transitions.

### Non-Goals
- **No Architectural Paradigm Shift:** We are not introducing an Elm/Redux/Relm4 architecture or altering GTK4's event/closure model.
- **No Visual or Feature Changes:** No changes to colors, fonts, layouts, animations, or styling.
- **No Backend/Database Changes:** `src/engine/` remains completely untouched.

### Anti-Goals
- **No Cyclic Dependencies:** Modules will have strict, one-way dependencies (e.g. `metrics` has no dependencies on UI or state; `sidebar` and `details` depend on `state`, not each other).
- **No Test Breakage:** Refactoring must not disable, mock out, or delete any of the 81 existing tests.

---

## 3. Approaches Considered & Selected Approach

### Approach 1: Domain & View Component Decomposition (Selected)
Decompose `window.rs` and `charts.rs` into clean, single-responsibility files within the existing crate structure. Pure calculations are isolated from UI widgets. Re-exports preserve full compatibility with existing integration tests.
- **Pros:** High cohesion, easy to navigate (~150–250 lines per file), low risk of regressions, no build system changes.
- **Cons:** Requires careful module structuring to prevent circular references between state and view widgets.

### Approach 2: Global State Machine / Action Reducer Architecture (Rejected)
Rewrite UI state around a global `Action` enum and dispatcher function.
- **Why rejected:** Would require rewriting every signal handler and closure across the entire app. Highly disruptive and prone to subtle animation or timing regressions.

### Approach 3: Cargo Workspace Multi-Crate Split (Rejected)
Split into multiple Cargo crates (`habit-engine`, `habit-charts`, `habit-ui`).
- **Why rejected:** Adds unnecessary complexity to `Cargo.toml`, Meson build scripts, and Flatpak manifests for a ~4,900-line codebase.

---

## 4. Component Architecture & Module Breakdown

### 4.1 Dependency Hierarchy
```
               +--------------------+
               |   src/engine/*     |  (db, models, scoring)
               +---------+----------+
                         |
                         v
               +--------------------+
               |   ui/metrics.rs    |  (Pure math, formatting, metrics)
               +---------+----------+
                         |
                         v
               +--------------------+
               |   ui/charts/*      |  (geometry, progress_ring, curve, heatmap)
               +---------+----------+
                         |
                         v
               +--------------------+
               |    ui/state.rs     |  (WindowInner, DrawCaches, refresh pipeline)
               +---+----+-------+---+
                   |    |       |
       +-----------+    |       +-----------+
       v                v                   v
+-------------+  +-------------+  +--------------------+
| ui/sidebar  |  | ui/details  |  | ui/history_editor  |
+------+------+  +------+------+  +---------+----------+
       |                |                   |
       +----------------+-------------------+
                        v
               +--------------------+
               |    ui/window.rs    |  (HabitWindow coordinator & run_app)
               +--------------------+
```

### 4.2 Module Specifications

#### 1. `src/ui/metrics.rs` (~260 lines)
- **Dependencies:** `chrono`, `crate::engine::models::*`, `crate::engine::scoring::*`. (Zero GTK dependencies).
- **Contents:**
  - `MonthNavigation` struct and implementation (`prev()`, `next()`, rollover logic).
  - Formatting functions: `format_schedule_summary`, `format_score_percentage`, `format_strength_percentage`, `format_streak_badge`, `format_percentage_diff`, `format_target_summary`.
  - `HabitMetrics` struct and `calculate_habit_metrics()`.
  - `toggle_entry_state()`.

#### 2. `src/ui/state.rs` (~240 lines)
- **Dependencies:** `gtk4`, `libadwaita`, `crate::engine::db::*`, `crate::engine::models::*`, `crate::ui::metrics::*`, `crate::ui::charts::*`.
- **Contents:**
  - `DrawCaches`: Shared `Rc<RefCell<...>>` caches for Cairo drawing functions (`overview_score`, `curve_data`, `heatmap_entries`, etc.).
  - `WindowInner`: Holds database connection, `nav_view`, `details_page`, `habit_list_box`, `selected_habit_id`.
  - `default_database_path()`: Returns standard XDG data directory path.
  - `refresh()`: Full application reload pipeline (queries DB, calculates scores, updates sidebar rows, updates details overview card, queues redraws).
  - Action handlers: `handle_toggle()`, `handle_select()`.

#### 3. `src/ui/sidebar.rs` (~180 lines)
- **Dependencies:** `gtk4`, `libadwaita`, `crate::ui::state::*`.
- **Contents:**
  - `SidebarWidgets`: Struct holding references to sidebar `page`, `list_box`, `add_btn`, `menu_btn`, and `day_labels`.
  - `build_sidebar()`: Constructs the sidebar navigation page, title bar, quick-check 5-day / 7-day weekday header bar, and scrolled habit list container.

#### 4. `src/ui/details.rs` (~250 lines)
- **Dependencies:** `gtk4`, `libadwaita`, `crate::ui::state::*`, `crate::ui::metrics::*`, `crate::ui::charts::*`.
- **Contents:**
  - `DetailsWidgets`: References to detail page widgets (title bar, delete button, overview stat labels, drawing areas).
  - `build_overview_section()`: Constructs the 4-column overview card and score progress ring.
  - `build_heatmap_section()`: Constructs the continuous heatmap week-navigation controls and drawing area.
  - `build_details()`: Assembles the complete details `NavigationPage`.
  - `connect_detail_actions()`: Wires up delete button dialog and habit deletion pipeline.

#### 5. `src/ui/history_editor.rs` (~160 lines)
- **Dependencies:** `gtk4`, `libadwaita`, `crate::ui::state::*`, `crate::ui::charts::*`.
- **Contents:**
  - `HistoryEditorWidgets`: Struct holding the bottom sheet container, revealer, title, drawing area, and close button.
  - `build_history_editor()`: Builds the bottom sheet box, 180ms revealer, and drawing area.
  - `setup_history_editor_controllers()`: Connects Escape key controller, backdrop click gesture, scroll gesture for week pagination, and heatmap cell click gesture.

#### 6. `src/ui/window.rs` (~150 lines)
- **Dependencies:** `gtk4`, `libadwaita`, `crate::ui::*`.
- **Contents:**
  - `HabitWindow`: Application window struct implementing `Deref` and `AsRef<Widget>`.
  - Window creation and layout: sets 520x700 locked dimensions, instantiates `NavigationView` with `animate_transitions(false)`.
  - `setup_primary_menu()`: Populates the hamburger menu (Import/Export backups, About dialog).
  - `connect_drawing_areas()`: Connects `DrawCaches` to Cairo drawing areas on sidebar and details.
  - `run_app()`: Application entry point for Flatpak / CLI.
  - Backwards-compatible re-exports: `pub use crate::ui::metrics::*;`.

#### 7. `src/ui/charts/` package (~825 lines split into ~100–250 lines each)
- `src/ui/charts/geometry.rs`: `Point`, `Rect`, `Rgba`.
- `src/ui/charts/progress_ring.rs`: `draw_progress_ring()`.
- `src/ui/charts/curve.rs`: `calculate_curve_points()`, `draw_strength_curve()`.
- `src/ui/charts/heatmap.rs`: `LoopHeatmapLayout`, `calculate_loop_heatmap_layout()`, `loop_heatmap_cell_rect()`, `loop_heatmap_hit_test()`, `draw_loop_heatmap()`.
- `src/ui/charts/calendar.rs`: `days_in_month()`, `month_name()`, `heatmap_cell_rect()`, `heatmap_hit_test()`, `draw_monthly_heatmap()`.
- `src/ui/charts/mod.rs`: Re-exports all public functions and types so all external references remain unchanged.

---

## 5. Verification & Testing Strategy

1. **Compilation & Strict Lints:**
   - `cargo check --all --features ui`
   - `cargo clippy --all --features ui -- -D warnings`
2. **Automated Test Suite (All 81 tests must pass):**
   - `tests/test_charts.rs` (10 tests)
   - `tests/test_db.rs` (7 tests)
   - `tests/test_habit_dialog.rs` (9 tests)
   - `tests/test_habit_row.rs` (11 tests)
   - `tests/test_models.rs` (5 tests)
   - `tests/test_portal_integration.rs` (6 tests)
   - `tests/test_scaffolding.rs` (4 tests)
   - `tests/test_scoring.rs` (15 tests)
   - `tests/test_window.rs` (11 tests)
3. **Flatpak Sandboxed Packaging:**
   - Execute clean build: `flatpak-builder --force-clean --repo=repo build-dir build-aux/io.github.habit.Habit.json`
   - Re-bundle: `flatpak build-bundle repo habit.flatpak io.github.habit.Habit`
   - Re-install: `flatpak install --user -y --reinstall habit.flatpak`
4. **Behavioral Integrity Checks:**
   - Verify window size is locked at 520x700.
   - Verify selecting a habit opens details instantly with no page-slide transition.
   - Verify clicking edit history slides up the bottom sheet in 180ms without lag.
   - Verify clicking Cancel or Escape dismisses the sheet smoothly in 180ms.
