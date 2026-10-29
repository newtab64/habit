# Habit

Habit is a simple habit tracker for Linux, inspired by Loop Habit Tracker. Check off your habits each day and see how consistent you are, with no accounts and no cloud.

## Features

- Check off habits for today and the last few days, marking each day as done, a rest day or missed.
- Set rest days so planned breaks do not hurt your progress.
- Track goals with numbers, such as glasses of water or pages read.
- See your progress as a score, a 30-day curve and a calendar of past days.
- Fix days you forgot to log in the history editor.
- Pick a color for each habit.
- Create a backup file and restore it any time.

## Install

Install a `habit.flatpak` file:

```bash
flatpak install --user habit.flatpak
```

Then open Habit from your app grid. Habit needs a Wayland session, which is the default on current GNOME.

## Your data

Everything stays on your computer. To save a copy or move to another machine, open the menu and choose "Create Backup" or "Restore Backup".

When installed as a Flatpak, your data folder is `~/.var/app/io.github.habit.Habit/data/habit/` (`$XDG_DATA_HOME/habit/`). Outside Flatpak, it is `~/.local/share/habit/`.

## Building from source

See [CONTRIBUTING.md](CONTRIBUTING.md) for how to build, test and package Habit from source.

## License

GPL-3.0-or-later. See [LICENSE](LICENSE).
