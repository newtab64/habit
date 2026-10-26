# Habit

A native, privacy-first Linux habit tracker for GNOME, inspired by Android's [Loop Habit Tracker](https://github.com/iSoron/uhabits). Built with strict Rust (`#![forbid(unsafe_code)]`), local-only SQLite persistence, and Libadwaita.

![Habit Icon](data/icons/hicolor/scalable/apps/io.github.habit.Habit.svg)

---

## Features

- **Loop Exponential Decay Algorithm:**
  Habit strength scores $S_t \in [0.0, 1.0]$ reflect consistency over time using standard half-life decay ($T_{1/2} = 14.0$ days):
  $$S_t = S_{t-1} + (1.0 - S_{t-1}) \cdot \alpha \quad \text{(completed)}$$
  $$S_t = S_{t-1} \cdot (1.0 - \alpha) \quad \text{(missed)}$$

- **Pre-defined Rest Days:**
  Habits with custom weekly schedules (e.g., Gym on Mon, Wed, Fri, Sat) automatically treat off-days as rest days (`—`). Rest days freeze strength decay ($S_t = S_{t-1}$) and preserve active streaks without penalty.

- **All-or-Nothing Measurable Goals:**
  For numerical habits (e.g. read 30 pages), partial progress ($v < \text{target}$) breaks the streak and grants zero habit strength. Only achieving $v \ge \text{target}$ marks the session completed (`✓`).

- **Minimalist Checklist Interface:**
  No circular backgrounds or button borders—just bare accent-colored symbols:
  - `✓` (completed session in habit accent color)
  - `—` (rest day or manual skip in habit accent color)
  - `✕` (missed day in muted gray)
  - Raw numbers for measurable entries (e.g. `30` in accent, `15` in muted gray)
  - Clean 5px progress bars without percentage clutter.
  - 7-day checklist with `Today` first on the left, followed by past days (`Yest`, `-2d`... `-6d`).

- **50/50 Responsive Split View:**
  - **Left Pane:** Fast checklist with instant single-click toggling for binary habits and popover steppers for measurable habits.
  - **Right Pane:** Deep-dive analytics, 3 compact stat cards (Strength %, Current Streak, Best Streak), smooth 30-day continuous Cairo strength curve with gradient fill, and an interactive monthly heatmap.
  - **Interactive Heatmap:** Page through historical months with `<` and `>`, and click any past cell to backfill or adjust entries.
  - **Responsive:** Automatically collapses into a single-pane view with native back navigation on screens narrower than `760px`.

- **Per-Habit Accent Colors:**
  Set any custom 6-digit Hex code (`#FF7800`) per habit, with quick access to 8 standard GNOME palette presets.

- **Atomic SQLite Storage & Portals:**
  - Zero cloud, zero network, zero telemetry.
  - Stored locally in a high-performance WAL-mode SQLite database.
  - Atomic `.db` database snapshots exported and imported via the XDG FileChooser Portal with `PRAGMA integrity_check` validation.

---

## Layout

```text
+---------------------------------------------------------------------------------------------------+
|  [●] Habits                                                                   [+ New Habit]  [⋮]  |
+-------------------------------------------------+-------------------------------------------------+
|  HABIT & PROGRESS           Today Yest -2d  -3d |  Gym / Workout                                  |
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

---

## Building and Running

### Prerequisites

- Rust 1.85+ (Edition 2021)
- GTK 4 (`>= 4.10.0`) & Libadwaita (`>= 1.5.0`)
- SQLite 3 (bundled automatically via `rusqlite`)

### Run Natively

```bash
# Run with GUI
cargo run --features ui

# Run unit and integration tests (57 tests)
cargo test --all
cargo test --features ui

# Run strict linter audit
cargo clippy --all-targets --features ui -- -D warnings
```

### Build & Run with Flatpak

`Habit` is configured to adhere strictly to the GNOME Software **"Safe"** sandboxing criteria (strictly no `--share=network`, no `--filesystem=host`, no `--filesystem=home`):

```bash
# Build and install locally
flatpak-builder --user --install --force-clean build-dir build-aux/io.github.habit.Habit.json

# Run the installed Flatpak application
flatpak run io.github.habit.Habit
```

---

## License

This project is licensed under the **GNU General Public License v3.0 or later** ([GPL-3.0-or-later](LICENSE)).
