# Habit - Design Document

## 1. Intent & Purpose

**Habit** is a native, local-only Linux habit tracking desktop application inspired by the minimalism and mechanics of Android's Loop Habit Tracker. Built from scratch in strict Rust using `gtk4` and `libadwaita`, it is engineered for users who value extreme privacy, zero telemetry, lightning-fast response times with zero animation fluff, and full compliance with GNOME Software "Safe" sandboxing standards.

Unlike basic streak counters that penalize users arbitrarily on planned recovery days (e.g., gym rest days), Habit implements an exponential moving average decay formula that rewards consistency while treating pre-defined rest days neutrally.

---

## 2. Goals, Non-Goals & Anti-Goals

### 2.1 Goals
- **Native GNOME Experience:** Modern, clean GTK4/Libadwaita desktop application following GNOME Human Interface Guidelines (HIG) with adaptive desktop scaling.
- **Loop-Style Habit Engine:**
  - **Binary habits** and **measurable numerical habits** (e.g. pages read, water intake).
  - **Pre-defined Rest Days:** Days outside a habit's schedule (e.g., non-gym days on Tue/Thu/Sun for a Mon/Wed/Fri/Sat workout schedule) are pre-defined rest days (`—`) that preserve streaks and score without penalty.
  - **All-or-Nothing Measurable Goals:** Logging partial progress ($v < \text{target}$) is treated as incomplete: it does not advance or preserve the streak, does not add to habit strength, and is marked as not complete (`✕`). Only meeting or exceeding the target ($v \ge \text{target}$) grants completion credit (`✓`).
  - **Exponential Moving Average Decay Scoring:** Habit strength computed from 0% to 100%.
- **Minimalist 50/50 Responsive Split View:**
  - Left pane: Habit list with inline 7-day fast-action symbols. The current day (`Today`) is positioned first on the left, followed by past days (`Yest`, `-2d`, `-3d`, ...).
  - Right pane: In-depth analytics for the selected habit (strength score card, current streak, best streak, 30-day Cairo strength curve, interactive monthly calendar heatmap with `<` / `>` pagination).
  - Responsive collapse: When the window width is narrowed below threshold, the right pane collapses completely, displaying only the fast-check list.
- **Ultra-Minimalist Symbol-Only Interaction:**
  - No enclosing shapes, circles, or button borders on the checklist.
  - Done: Accent-colored checkmark (`✓`).
  - Pre-defined Rest Day / Skip: Accent-colored dash (`—`).
  - Incomplete / Missed: Muted gray `✕` (or raw value if partial, e.g. `15`).
  - Completed measurable habits: Bare number in accent color (e.g., `35`, `30`).
- **Habit Strength Progress Bar:** Sleek horizontal bar under the habit title reflecting habit strength visually, with no score numbers cluttering the list.
- **Custom Accent Colors:** Full custom hex code support (e.g., `#FF7800`) per habit, with a palette of quick presets. All icons, progress bars, and graphs synchronize with the habit's accent color.
- **Atomic Local Persistence & Backup:**
  - Embedded SQLite database configured with Write-Ahead Logging (WAL) for atomic transactions.
  - Export and import restricted strictly to atomic `.db` snapshots through the XDG FileChooser Portal.
- **Strict GNOME Software "Safe" Sandbox:**
  - Zero network access permissions (no `--share=network`).
  - No host or home filesystem access (no `--filesystem=host` / `--filesystem=home`). Data resides strictly in isolated app storage (`~/.var/app/.../data/`).

### 2.2 Non-Goals
- Cloud synchronization, accounts, authentication, or remote backups.
- Telemetry, crash reporting over network, or analytics collection.
- CSV or JSON import/export (avoiding partial/corrupted schema states; only atomic `.db` snapshots).
- WebViews, Electron wrappers, or web technologies.
- Cross-platform support (Linux Flatpak only).

### 2.3 Anti-Goals
- **Animations / Transition Lag:** Zero unnecessary animations; interactions and screen updates occur instantaneously.
- **Bloat & Dependency Creep:** No heavy reactive macro frameworks; direct, transparent Rust bindings.
- **Fragile Streaks on Planned Rest:** No brittle counters that reset progress to zero when taking pre-defined rest days.

---

## 3. Constraints & Technical Environment

- **Language:** Rust (edition 2021 or 2024).
- **Strict Safety Rules:**
  - `#![forbid(unsafe_code)]` across all application crates/modules.
  - `#![warn(clippy::pedantic, clippy::nursery)]` enforced in CI and build.
- **UI Tooling:**
  - `gtk4` (v0.9+ bindings)
  - `libadwaita` (v0.7+ bindings, Adw 1.5+)
  - `cairo-rs` for drawing habit strength curves and calendar heatmaps on `GtkDrawingArea`.
- **Database:**
  - `rusqlite` with bundled SQLite or system SQLite.
  - WAL journal mode (`PRAGMA journal_mode = WAL;`) and immediate transactions for full ACID safety.
- **Packaging:** Flatpak manifest (`io.github.habit.Habit.json` or YAML) using Freedesktop/GNOME runtime (SDK 47+), configured strictly for GNOME Software Safe tags.

---

## 4. Architectural Approaches & Selection

During the brainstorm, two technical approaches were evaluated:

### Approach 1: Native GTK4/Libadwaita + Pure Rust Engine (Selected)
- **Architecture:**
  - `engine/`: Independent Rust module containing pure domain models, SQLite database management with atomic transactions, streak calculations, and Loop's exponential decay formula. Has zero GTK dependencies, enabling 100% test coverage using headless `cargo test`.
  - `ui/`: Native GTK4/Libadwaita widgets (`AdwApplicationWindow`, `AdwNavigationSplitView`, `GtkDrawingArea`). UI state updates communicate across threads using `glib::MainContext::channel` or idiomatic signals.
- **Tradeoffs:**
  - *Pros:* Cleanest separation of logic from UI; zero third-party framework overhead; effortless compliance with `#![forbid(unsafe_code)]` and strict clippy; instant startup.
  - *Cons:* Requires explicit UI refresh routines upon data change rather than macro-driven automatic bindings.

### Approach 2: Relm4 Component Architecture (Rejected)
- **Architecture:**
  - Uses `relm4` to wrap GTK4 in a centralized Elm-style message dispatch loop (`Msg::ToggleDay`, `Msg::SelectHabit`).
- **Tradeoffs:**
  - *Pros:* Centralized state reducer and declarative widget macros.
  - *Cons:* Heavy procedural macro expansion increases Flatpak build times; generates boilerplate code that conflicts with strict pedantic clippy warnings; introduces external abstractions contrary to the "no crap" requirement.

**Decision:** **Approach 1** was selected by unanimous agreement for its minimalism, auditability, and adherence to strict Rust standards.

---

## 5. Detailed Design & Mechanics

### 5.1 Habit Data Model & Scoring Engine

#### Habit Types & Completion Rules
1. **Binary Habit:**
   - Goal: 1 completion per scheduled day.
   - Entry states: `Completed` (1.0), `PreDefinedRest`/`Skipped` (neutral), `Missed` (0.0).
2. **Measurable Habit:**
   - Goal: Target numeric value (e.g. 30) with unit string (e.g. `pages`).
   - Rule: **All-or-Nothing.** If logged value $v < \text{target}$, it is marked as `missed` / not complete ($c = 0.0$); it does not count toward the streak and does not add points to the habit strength progress bar. If $v \ge \text{target}$, it is marked `completed` ($c = 1.0$), advancing the streak and increasing habit strength.

#### Pre-Defined Rest Days (e.g. Gym Recovery)
- Habits specify an active schedule:
  - **Daily:** Every day is an active target day.
  - **Specific Days of Week:** (e.g., Mon, Wed, Fri, Sat for gym).
- Any day **outside** the pre-defined schedule is automatically a **Pre-defined Rest Day (`—`)**:
  - Score is frozen ($S_t = S_{t-1}$); zero decay penalty is applied.
  - Streak is preserved across rest days (e.g., completing Sat and Mon maintains the streak).
  - Rendered automatically as an accent-colored dash (`—`) in the checklist and heatmap.

#### Scoring Algorithm (Loop Exponential Decay)
- Habit strength $S_t \in [0.0, 1.0]$ represents consistency over time.
- For day $t$:
  - If **Completed** ($v \ge \text{target}$ or checked):
    $$S_t = S_{t-1} + (1.0 - S_{t-1}) \cdot \alpha$$
  - If **Missed** or **Partial** ($v < \text{target}$ or uncompleted):
    $$S_t = S_{t-1} \cdot (1.0 - \alpha)$$
  - If **Pre-defined Rest Day / Skipped**:
    $$S_t = S_{t-1}$$
- The decay parameter $\alpha \approx 0.05$ to $0.10$ is derived from habit schedule and half-life.

### 5.2 UI Layout & Interactions

```
+---------------------------------------------------------------------------------------------------+
|  [●] Habits                                                                   [+ New Habit]  [⋮]  |
+-------------------------------------------------+-------------------------------------------------+
|  HABIT & PROGRESS           Today Yest -2d  -3d |  Gym / Workout                      [Hex: #FF78] |
|-------------------------------------------------|  Schedule: Mon, Wed, Fri, Sat • Target: 1 sess  |
|  Gym / Workout                                  |-------------------------------------------------|
|  [======    ]               ✕    ✓    —    ✓    |  [ Strength: 88% ] [ Current: 4d ] [ Best: 18d ] |
|-------------------------------------------------|-------------------------------------------------|
|  Read Books (30 pgs)                            |  Habit Strength Curve (Last 30 Days)            |
|  [====      ]               ✕    35   ✕    30   |   _.-''`--.____                                 |
|-------------------------------------------------|  ---------------------------------------------  |
|  Morning Meditation                             |  History Heatmap (October 2026)      [<] [>]    |
|  [========  ]               ✓    ✓    ✓    ✓    |  [■][■][■][—][■][■][■]                          |
+-------------------------------------------------+-------------------------------------------------+
```

#### Left Pane: Fast-Check List
- Header shows column titles: `Today`, `Yest`, `-2d`, `-3d`, `-4d`, `-5d`, `-6d`.
- Habit row contents:
  - Habit title (e.g. `Gym / Workout`).
  - Slim horizontal `GtkProgressBar` (5px height, rounded) filled to habit strength $S_t$ with the habit's accent color. No text in or next to the bar.
  - Inline action symbols (No enclosing boxes or circles):
    - Scheduled Day:
      - Binary: Click toggles between `✕` and `✓` (or manual skip `—`).
      - Measurable: Click opens compact inline popover with number entry stepper; hitting target shows accent number, below target shows muted `✕`.
    - Pre-defined Rest Day: Shows accent-colored dash (`—`) automatically.
- Clicking the habit title selects it and populates the right pane.

#### Right Pane: Deep-Dive Analytics & Interactive Heatmap
- Header: Title, schedule days, target/unit, and hex color badge.
- Summary Cards: 3 compact stat boxes showing Habit Strength %, Current Streak, and Best Streak.
- Strength Curve: Cairo custom drawing rendering the continuous line graph over the last 30/90 days in the habit's accent color.
- Interactive Monthly Calendar Heatmap:
  - Commit-style matrix of squares colored by entry state (filled accent for complete, dash for rest/skip, dark for missed).
  - Navigation controls (`<` and `>`) for paging through historical months.
  - **Interactive:** Clicking any historical cell allows toggling or backfilling entries older than 6 days ago.

#### Responsive Breakpoint
- Implemented with `AdwNavigationSplitView`.
- Below 760px window width: the right pane is hidden. Only the fast-check list is displayed. Clicking a habit navigates via `AdwNavigationView` to the detail page with a native header Back button.

### 5.3 Accent Colors & Theming
- Stored as standard 6-digit hex string (e.g. `#FF7800`).
- Color picker dialog offers a quick-select palette of 8 standard GNOME palette colors plus a raw Hex input field.
- Dynamic CSS provider applies the active hex color to the habit's row symbols, progress bar fill, and Cairo render contexts.

### 5.4 Database Schema & Atomic Transactions

```sql
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS habits (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    habit_type TEXT NOT NULL CHECK(habit_type IN ('binary', 'measurable')),
    target_value REAL NOT NULL DEFAULT 1.0,
    unit TEXT,
    accent_color TEXT NOT NULL DEFAULT '#3584e4',
    schedule_type TEXT NOT NULL DEFAULT 'daily', -- 'daily', 'weekdays'
    scheduled_days TEXT NOT NULL DEFAULT '[1,2,3,4,5,6,7]', -- JSON array of active ISO weekday numbers (1=Mon..7=Sun)
    archived INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS habit_entries (
    habit_id INTEGER NOT NULL REFERENCES habits(id) ON DELETE CASCADE,
    date TEXT NOT NULL, -- ISO-8601 YYYY-MM-DD
    entry_status TEXT NOT NULL CHECK(entry_status IN ('completed', 'skipped', 'missed')),
    value REAL NOT NULL DEFAULT 0.0,
    PRIMARY KEY (habit_id, date)
);

CREATE INDEX IF NOT EXISTS idx_entries_habit_date ON habit_entries(habit_id, date);
```

#### Atomic Export & Import
- **Export:** Executes SQLite backup API (`rusqlite::backup`) to write an exact, consistent database snapshot to a user-selected path requested via the XDG File Chooser Portal (`gtk4::FileDialog`).
- **Import:** Validates the selected file against schema integrity checks (`PRAGMA integrity_check`, table verification) before replacing the database in an atomic swap.

### 5.5 Flatpak Manifest & Sandboxing Security

- **App ID:** `io.github.habit.Habit`
- **Permissions (`finish-args`):**
  - `--socket=wayland`
  - `--socket=fallback-x11`
  - `--share=ipc`
  - **No network:** `--share=network` is strictly omitted.
  - **No filesystem:** `--filesystem=home` and `--filesystem=host` are strictly omitted.
  - Persistent storage is confined strictly to `~/.var/app/io.github.habit.Habit/data`.
  - All file interactions outside the sandbox use the XDG Desktop Portal.

---

## 6. What Is Left to the Builder

The builder has full implementation autonomy over:
1. Exact Meson and Cargo build pipeline integration.
2. Icon design, AppStream metadata (`io.github.habit.Habit.metainfo.xml`), and desktop entry file.
3. Keyboard navigation bindings (e.g., arrow navigation and Space key to toggle entries).
4. Cairo antialiasing parameters, bezier curve smoothing, and padding geometry for the strength graph.
5. In-memory caching layer for daily scores to avoid re-calculating historical decays on every render.
