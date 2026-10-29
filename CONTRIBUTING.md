# Contributing

## Branches

- `unstable` is where all work happens.
- `stable` is what users get. Never commit to it directly. Merge `unstable` into `stable` only through a pull request, using "Create a merge commit", after the checklist below passes.

## Test build (from unstable)

```bash
flatpak-builder --user --install --force-clean --default-branch=unstable build-dir build-aux/io.github.habit.Habit.json
```

Note: all branches share the same data folder, so back up your data first.

## Checklist before merging to stable

Run:

```bash
cargo fmt
cargo clippy --features ui --all-targets -- -D warnings
cargo test --features ui
cargo test
```

Then in the app: create a backup, restore a backup, cancel both file pickers, add, edit and delete a habit, toggle days, log a measurable value, open the details page and the heatmap, reorder habits, start a second copy (it must only focus the first one).

## Release

Bump the version in `Cargo.toml` and the release entry (version and date) in the metainfo. Merge to `stable`, tag it (for example `v0.1.0`), then:

```bash
flatpak-builder --force-clean --default-branch=stable --repo=repo build-dir build-aux/io.github.habit.Habit.json
flatpak build-bundle repo habit.flatpak io.github.habit.Habit stable --runtime-repo=https://flathub.org/repo/flathub.flatpakrepo
```

Attach `habit.flatpak` to the GitHub release.
