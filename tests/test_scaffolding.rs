#[test]
fn test_crate_compiles_under_strict_safety() {
    assert_eq!(habit::APP_ID, "io.github.habit.Habit");
}

#[test]
fn test_flatpak_manifest_sandbox_safety() {
    let manifest_str = std::fs::read_to_string("build-aux/io.github.habit.Habit.json")
        .expect("build-aux/io.github.habit.Habit.json should exist");
    assert!(
        !manifest_str.contains("--share=network"),
        "Must not request network permission"
    );
    assert!(
        !manifest_str.contains("--filesystem=host"),
        "Must not request host filesystem"
    );
    assert!(
        !manifest_str.contains("--filesystem=home"),
        "Must not request home filesystem"
    );
    assert!(manifest_str.contains("\"app-id\": \"io.github.habit.Habit\""));
    assert!(manifest_str.contains("\"runtime-version\": \"50\""));
    assert!(manifest_str.contains("\"runtime\": \"org.gnome.Platform\""));
    assert!(manifest_str.contains("\"sdk\": \"org.gnome.Sdk\""));
    assert!(
        manifest_str.contains("data/icons/hicolor/scalable/apps/io.github.habit.Habit.svg"),
        "Build commands must install scalable icon"
    );
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

    let icon_path = std::path::Path::new("data/icons/hicolor/scalable/apps/io.github.habit.Habit.svg");
    assert!(icon_path.exists(), "SVG icon must exist");
    let icon_content = std::fs::read_to_string(icon_path).expect("SVG icon must be readable");
    assert!(icon_content.contains("<svg"), "Must be a valid SVG");
    assert!(icon_content.contains("viewBox=\"0 0 128 128\""));
}

#[test]
fn test_meson_build_files_configured() {
    let root_meson = std::fs::read_to_string("meson.build").expect("meson.build should exist");
    assert!(root_meson.contains("project(\n  'habit'"));
    assert!(root_meson.contains("dependency('gtk4'"));
    assert!(root_meson.contains("dependency('libadwaita-1'"));
    assert!(root_meson.contains("subdir('data')"));
    assert!(root_meson.contains("subdir('src')"));

    let data_meson =
        std::fs::read_to_string("data/meson.build").expect("data/meson.build should exist");
    assert!(data_meson.contains("io.github.habit.Habit.desktop"));
    assert!(data_meson.contains("io.github.habit.Habit.metainfo.xml"));
    assert!(data_meson.contains("io.github.habit.Habit.svg"));

    let src_meson =
        std::fs::read_to_string("src/meson.build").expect("src/meson.build should exist");
    assert!(src_meson.contains("find_program('cargo'"));
    assert!(src_meson.contains("custom_target"));
    assert!(src_meson.contains("--features"));
    assert!(src_meson.contains("ui"));
}
