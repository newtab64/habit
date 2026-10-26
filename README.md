# Habit

A clean, simple habit tracker for Linux, inspired by Loop Habit Tracker.

Designed to feel right at home on GNOME, Habit helps you build good routines without distractions, accounts, or cloud sync. Your data stays entirely on your computer.

![Habit Icon](data/icons/hicolor/scalable/apps/io.github.habit.Habit.svg)

---

## Why Habit?

- **Smart habit scoring:** Reflects how consistent you've actually been over time, rather than just counting a raw streak.
- **Built-in rest days:** Taking a scheduled break (like rest days between workouts) won't break your streak or hurt your score.
- **Clear goal tracking:** For habits with target numbers (like drinking 8 glasses of water or reading 30 pages), you either hit your daily goal or you don't—no half-credit confusion.
- **Clean and distraction-free:** Simple, color-coded symbols (`✓` done, `—` rest day, `✕` missed). No circular buttons, badges, or flashy animations.
- **Detailed history at a glance:** See your 30-day progress curve and browse a monthly calendar heatmap to see your consistency or fill in days you forgot to log.
- **Personal colors:** Pick any color you like for each habit to make your list easy to scan.
- **100% private & offline:** No accounts, no internet access, and no telemetry. You can easily back up your data to a single file anytime you want.

---

## How to Run

### Run directly with Cargo

```bash
cargo run --features ui
```

### Install as Flatpak

```bash
flatpak-builder --user --install --force-clean build-dir build-aux/io.github.habit.Habit.json
flatpak run io.github.habit.Habit
```

---

## License

Free and open-source under the [GPL-3.0 License](LICENSE).
