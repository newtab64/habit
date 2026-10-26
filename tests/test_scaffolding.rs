#[test]
fn test_crate_compiles_under_strict_safety() {
    assert_eq!(habit::APP_ID, "io.github.habit.Habit");
}

#[test]
fn test_flatpak_manifest_sandbox_safety() {
    let manifest_str = std::fs::read_to_string("build-aux/io.github.habit.Habit.json")
        .expect("build-aux/io.github.habit.Habit.json should exist");
    assert!(!manifest_str.contains("--share=network"), "Must not request network permission");
    assert!(!manifest_str.contains("--filesystem=host"), "Must not request host filesystem");
    assert!(!manifest_str.contains("--filesystem=home"), "Must not request home filesystem");
    assert!(manifest_str.contains("\"app-id\": \"io.github.habit.Habit\""));
}

#[test]
fn test_desktop_and_metainfo_present() {
    let desktop = std::fs::read_to_string("data/io.github.habit.Habit.desktop")
        .expect("data/io.github.habit.Habit.desktop should exist");
    assert!(desktop.contains("Exec=habit"));
    assert!(desktop.contains("Icon=io.github.habit.Habit"));

    let metainfo = std::fs::read_to_string("data/io.github.habit.Habit.metainfo.xml")
        .expect("data/io.github.habit.Habit.metainfo.xml should exist");
    assert!(metainfo.contains("<id>io.github.habit.Habit</id>"));
}
