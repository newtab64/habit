pub use crate::ui::metrics::*;

#[cfg(feature = "ui")]
use std::cell::RefCell;
#[cfg(feature = "ui")]
use std::rc::Rc;

#[cfg(feature = "ui")]
use gtk4::prelude::*;
#[cfg(feature = "ui")]
use libadwaita::prelude::*;

#[cfg(feature = "ui")]
use crate::engine::db::Database;
#[cfg(feature = "ui")]
use crate::ui::details::{
    build_details, connect_detail_actions, connect_drawing_areas, DetailsWidgets,
};
#[cfg(feature = "ui")]
use crate::ui::habit_dialog::show_habit_dialog;
#[cfg(feature = "ui")]
use crate::ui::portal::{trigger_export_dialog, trigger_import_dialog};
#[cfg(feature = "ui")]
use crate::ui::sidebar::{build_sidebar, SidebarWidgets};
#[cfg(feature = "ui")]
use crate::ui::state::{default_database_path, refresh, DrawCaches, WindowInner};

/// Connects user interface controllers, including the add habit dialog and detail view actions.
#[cfg(feature = "ui")]
fn connect_controllers(
    inner_rc: &Rc<RefCell<WindowInner>>,
    caches: &DrawCaches,
    root_window: &gtk4::Window,
    root_overlay: &gtk4::Overlay,
    details: &DetailsWidgets,
    add_btn: &gtk4::Button,
) {
    let inner_add = Rc::clone(inner_rc);
    let caches_add = caches.clone();
    let overlay_add = root_overlay.clone();
    add_btn.connect_clicked(move |_| {
        let inner_saved = Rc::clone(&inner_add);
        let caches_saved = caches_add.clone();
        show_habit_dialog(&overlay_add, None, move |new_habit| {
            let db = {
                let inner = inner_saved.borrow();
                Rc::clone(&inner.db)
            };
            let create_res = db.borrow().create_habit(&new_habit);
            if let Ok(id) = create_res {
                let (nav_view, details_page) = {
                    let mut inner = inner_saved.borrow_mut();
                    inner.selected_habit_id = Some(id);
                    (inner.nav_view.clone(), inner.details_page.clone())
                };
                refresh(&inner_saved, &caches_saved);
                nav_view.push(&details_page);
            } else {
                refresh(&inner_saved, &caches_saved);
            }
        });
    });

    connect_detail_actions(inner_rc, caches, root_window, root_overlay, details);
}

/// Configures the primary window menu with backup export, import, and about actions.
#[cfg(feature = "ui")]
fn setup_primary_menu(
    window: &libadwaita::ApplicationWindow,
    root_window: &gtk4::Window,
    inner_rc: &Rc<RefCell<WindowInner>>,
    caches: &DrawCaches,
    menu_btn: &gtk4::MenuButton,
    toast_overlay: &libadwaita::ToastOverlay,
) {
    let menu_model = gtk4::gio::Menu::new();
    menu_model.append(Some("Export Backup..."), Some("win.export-backup"));
    menu_model.append(Some("Import Backup..."), Some("win.import-backup"));
    menu_model.append(Some("About Habit"), Some("win.about"));
    menu_btn.set_menu_model(Some(&menu_model));

    // Export action
    let action_export = gtk4::gio::SimpleAction::new("export-backup", None);
    let inner_exp = Rc::clone(inner_rc);
    let root_exp = root_window.clone();
    let toast_exp = toast_overlay.clone();
    action_export.connect_activate(move |_, _| {
        let db = {
            let inner = inner_exp.borrow();
            Rc::clone(&inner.db)
        };
        let toast = toast_exp.clone();
        trigger_export_dialog(&root_exp, db, move |res| match res {
            Ok(path) => {
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("backup.db");
                toast.add_toast(libadwaita::Toast::new(&format!(
                    "Backup exported to {name}"
                )));
            }
            Err(err) => {
                if !err.contains("cancelled") && !err.contains("Dismissed") {
                    toast.add_toast(libadwaita::Toast::new(&format!("Export failed: {err}")));
                }
            }
        });
    });
    window.add_action(&action_export);

    // Import action
    let action_import = gtk4::gio::SimpleAction::new("import-backup", None);
    let inner_imp = Rc::clone(inner_rc);
    let caches_imp = caches.clone();
    let root_imp = root_window.clone();
    let toast_imp = toast_overlay.clone();
    action_import.connect_activate(move |_, _| {
        let db = {
            let inner = inner_imp.borrow();
            Rc::clone(&inner.db)
        };
        let toast = toast_imp.clone();
        let inner_done = Rc::clone(&inner_imp);
        let caches_done = caches_imp.clone();
        trigger_import_dialog(&root_imp, db, move |res| match res {
            Ok(_path) => {
                toast.add_toast(libadwaita::Toast::new("Database restored successfully"));
                {
                    let mut inner = inner_done.borrow_mut();
                    inner.selected_habit_id = None;
                }
                refresh(&inner_done, &caches_done);
            }
            Err(err) => {
                if !err.contains("cancelled") && !err.contains("Dismissed") {
                    toast.add_toast(libadwaita::Toast::new(&format!("Import failed: {err}")));
                }
            }
        });
    });
    window.add_action(&action_import);

    // About action
    let action_about = gtk4::gio::SimpleAction::new("about", None);
    let root_about = root_window.clone();
    action_about.connect_activate(move |_, _| {
        let about = libadwaita::AboutDialog::builder()
            .application_name("Habit")
            .application_icon(crate::APP_ID)
            .developer_name("Habit Contributors")
            .version("0.1.0")
            .license_type(gtk4::License::Gpl30)
            .comments("A native, local-only Linux habit tracker app for GNOME")
            .website("https://github.com/habit/habit")
            .issue_url("https://github.com/habit/habit/issues")
            .build();
        about.present(Some(&root_about));
    });
    window.add_action(&action_about);
}

/// Main application window providing Fast-Check List and push-navigation for details.
#[cfg(feature = "ui")]
#[derive(Debug, Clone)]
pub struct HabitWindow {
    window: libadwaita::ApplicationWindow,
    nav_view: libadwaita::NavigationView,
    toast_overlay: libadwaita::ToastOverlay,
}

#[cfg(feature = "ui")]
impl HabitWindow {
    /// Opens the application window using the default on-disk SQLite database path.
    ///
    /// # Panics
    /// Panics if opening an in-memory database fallback fails.
    #[must_use]
    pub fn open_default(app: &libadwaita::Application) -> Self {
        let db_path = default_database_path();
        let db = Database::open(&db_path).unwrap_or_else(|_| {
            Database::open_in_memory().expect("In-memory database fallback should always succeed")
        });
        Self::new(app, db)
    }

    /// Creates and initializes the `HabitWindow` with the provided database instance.
    #[must_use]
    pub fn new(app: &libadwaita::Application, db: Database) -> Self {
        let window = libadwaita::ApplicationWindow::builder()
            .application(app)
            .title("Habits")
            .default_width(520)
            .default_height(700)
            .resizable(false)
            .build();

        let nav_view = libadwaita::NavigationView::new();
        nav_view.set_animate_transitions(false);

        let sidebar: SidebarWidgets = build_sidebar();
        nav_view.add(&sidebar.page);

        let details: DetailsWidgets = build_details();
        let details_clone = details.clone();

        let toast_overlay = libadwaita::ToastOverlay::new();
        toast_overlay.set_child(Some(&nav_view));

        let root_overlay = gtk4::Overlay::new();
        root_overlay.set_child(Some(&toast_overlay));
        window.set_content(Some(&root_overlay));

        let now = chrono::Local::now();
        let today = now.date_naive();

        let caches = DrawCaches::new(today);

        connect_drawing_areas(&details, &caches);

        let habit_css_provider = gtk4::CssProvider::new();
        if let Some(display) = gtk4::gdk::Display::default() {
            gtk4::style_context_add_provider_for_display(
                &display,
                &habit_css_provider,
                gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }

        let db_rc = Rc::new(RefCell::new(db));
        let inner = WindowInner {
            nav_view: nav_view.clone(),
            details_page: details.page.clone(),
            db: Rc::clone(&db_rc),
            selected_habit_id: None,
            heatmap_week_offset: 0,
            habits_list_box: sidebar.habits_list_box,
            habit_score_label: sidebar.habit_score_label,
            header_day_labels: sidebar.header_day_labels,
            habit_css_provider,
            title_label: details.title_label,
            schedule_label: details.schedule_label,
            target_label: details.target_label,
            edit_btn: details.edit_btn.clone(),
            delete_btn: details.delete_btn.clone(),
            ring_drawing_area: details.ring_drawing_area,
            score_card_value: details.score_card_value,
            month_diff_card_value: details.month_diff_card_value,
            year_diff_card_value: details.year_diff_card_value,
            total_count_card_value: details.total_count_card_value,
            curve_drawing_area: details.curve_drawing_area,
            today_week_btn: details.today_week_btn,
            next_week_btn: details.next_week_btn,
            heatmap_drawing_area: details.heatmap_drawing_area,
        };

        let inner_rc = Rc::new(RefCell::new(inner));
        let root_window: gtk4::Window = window.clone().upcast();

        connect_controllers(
            &inner_rc,
            &caches,
            &root_window,
            &root_overlay,
            &details_clone,
            &sidebar.add_btn,
        );

        setup_primary_menu(
            &window,
            &root_window,
            &inner_rc,
            &caches,
            &sidebar.menu_btn,
            &toast_overlay,
        );

        // Initial populate and render
        refresh(&inner_rc, &caches);

        Self {
            window,
            nav_view,
            toast_overlay,
        }
    }

    /// Returns a reference to the wrapped `libadwaita::ApplicationWindow`.
    #[must_use]
    pub const fn window(&self) -> &libadwaita::ApplicationWindow {
        &self.window
    }

    /// Returns a reference to the `libadwaita::NavigationView`.
    #[must_use]
    pub const fn nav_view(&self) -> &libadwaita::NavigationView {
        &self.nav_view
    }

    /// Returns a reference to the `libadwaita::ToastOverlay`.
    #[must_use]
    pub const fn toast_overlay(&self) -> &libadwaita::ToastOverlay {
        &self.toast_overlay
    }

    /// Presents the application window to the user.
    pub fn present(&self) {
        self.window.present();
    }
}

#[cfg(feature = "ui")]
impl std::ops::Deref for HabitWindow {
    type Target = libadwaita::ApplicationWindow;

    fn deref(&self) -> &Self::Target {
        &self.window
    }
}

#[cfg(feature = "ui")]
impl AsRef<gtk4::Widget> for HabitWindow {
    fn as_ref(&self) -> &gtk4::Widget {
        self.window.upcast_ref()
    }
}

/// Application entry point: initializes libadwaita and launches `HabitWindow`.
#[cfg(feature = "ui")]
pub fn run_app() {
    let app = libadwaita::Application::builder()
        .application_id(crate::APP_ID)
        .build();

    app.connect_activate(|app| {
        let window = HabitWindow::open_default(app);
        window.present();
    });

    app.run();
}
