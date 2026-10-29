use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use libadwaita::prelude::*;

use crate::engine::db::Database;
use crate::ui::details::{
    build_details, connect_detail_actions, connect_drawing_areas, DetailsWidgets,
};
use crate::ui::habit_dialog::show_habit_dialog;
use crate::ui::portal::{trigger_export_dialog, trigger_import_dialog, DialogOutcome};
use crate::ui::sidebar::{build_sidebar, SidebarWidgets};
use crate::ui::state::{default_database_path, refresh, DrawCaches, WindowInner, BASE_CSS};

fn connect_lock_button(
    inner_rc: &Rc<RefCell<WindowInner>>,
    caches: &DrawCaches,
    lock_btn: &gtk4::Button,
) {
    let inner_lock = Rc::clone(inner_rc);
    let caches_lock = caches.clone();
    lock_btn.connect_clicked(move |_| {
        let is_reorder = caches_lock.reorder_mode.get();
        if is_reorder {
            let staged = caches_lock.staged_habit_order.borrow().clone();
            let db = {
                let inner = inner_lock.borrow();
                Rc::clone(&inner.db)
            };
            if let Err(e) = db.reorder_habits(&staged) {
                eprintln!("Failed to reorder habits: {e}");
                let inner = inner_lock.borrow();
                inner
                    .toast_overlay
                    .add_toast(libadwaita::Toast::new(&format!(
                        "Failed to save habit order: {e}"
                    )));
            }
            caches_lock.reorder_mode.set(false);
            caches_lock.staged_habit_order.borrow_mut().clear();
        } else {
            let ids: Vec<i64> = {
                let inner = inner_lock.borrow();
                inner
                    .db
                    .get_habits()
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(|h| h.id)
                    .collect()
            };
            *caches_lock.staged_habit_order.borrow_mut() = ids;
            caches_lock.reorder_mode.set(true);
        }
        refresh(&inner_lock, &caches_lock);
    });
}

fn connect_controllers(
    inner_rc: &Rc<RefCell<WindowInner>>,
    caches: &DrawCaches,
    root_window: &gtk4::Window,
    root_overlay: &gtk4::Overlay,
    details: &DetailsWidgets,
    lock_btn: &gtk4::Button,
    add_btn: &gtk4::Button,
) {
    connect_lock_button(inner_rc, caches, lock_btn);

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
            match db.create_habit(&new_habit) {
                Ok(id) => {
                    let (nav_view, details_page) = {
                        let mut inner = inner_saved.borrow_mut();
                        inner.selected_habit_id = Some(id);
                        (inner.nav_view.clone(), inner.details_page.clone())
                    };
                    refresh(&inner_saved, &caches_saved);
                    nav_view.push(&details_page);
                }
                Err(e) => {
                    eprintln!("Failed to create habit: {e}");
                    let toast_overlay = inner_saved.borrow().toast_overlay.clone();
                    toast_overlay.add_toast(libadwaita::Toast::new(&format!(
                        "Failed to create habit: {e}"
                    )));
                    refresh(&inner_saved, &caches_saved);
                }
            }
        });
    });

    connect_detail_actions(inner_rc, caches, root_window, root_overlay, details);
}

fn register_export_action(
    window: &libadwaita::ApplicationWindow,
    root_window: &gtk4::Window,
    inner_rc: &Rc<RefCell<WindowInner>>,
    toast_overlay: &libadwaita::ToastOverlay,
) {
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
        trigger_export_dialog(&root_exp, db, move |outcome| match outcome {
            DialogOutcome::Done(path) => {
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("backup.db");
                toast.add_toast(libadwaita::Toast::new(&format!(
                    "Backup exported to {name}"
                )));
            }
            DialogOutcome::Cancelled => {}
            DialogOutcome::Failed(err) => {
                toast.add_toast(libadwaita::Toast::new(&format!("Export failed: {err}")));
            }
        });
    });
    window.add_action(&action_export);
}

fn register_import_action(
    window: &libadwaita::ApplicationWindow,
    root_window: &gtk4::Window,
    inner_rc: &Rc<RefCell<WindowInner>>,
    caches: &DrawCaches,
    toast_overlay: &libadwaita::ToastOverlay,
) {
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
        trigger_import_dialog(&root_imp, db, move |outcome| match outcome {
            DialogOutcome::Done(_path) => {
                let nav_view = {
                    let mut inner = inner_done.borrow_mut();
                    inner.selected_habit_id = None;
                    inner.nav_view.clone()
                };
                let _ = nav_view.pop_to_tag("habits");
                refresh(&inner_done, &caches_done);
                toast.add_toast(libadwaita::Toast::new("Database restored successfully"));
            }
            DialogOutcome::Cancelled => {}
            DialogOutcome::Failed(err) => {
                toast.add_toast(libadwaita::Toast::new(&format!("Import failed: {err}")));
            }
        });
    });
    window.add_action(&action_import);
}

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

    register_export_action(window, root_window, inner_rc, toast_overlay);
    register_import_action(window, root_window, inner_rc, caches, toast_overlay);

    let action_about = gtk4::gio::SimpleAction::new("about", None);
    let root_about = root_window.clone();
    action_about.connect_activate(move |_, _| {
        let about = libadwaita::AboutDialog::builder()
            .application_name("Habit")
            .application_icon(crate::APP_ID)
            .developer_name("Habit Contributors")
            .version(env!("CARGO_PKG_VERSION"))
            .license_type(gtk4::License::Gpl30)
            .comments("A native, local-only Linux habit tracker app for GNOME")
            .website("https://github.com/habit/habit")
            .issue_url("https://github.com/habit/habit/issues")
            .build();
        about.present(Some(&root_about));
    });
    window.add_action(&action_about);
}

fn create_window_inner(
    nav_view: &libadwaita::NavigationView,
    toast_overlay: &libadwaita::ToastOverlay,
    db: Database,
    sidebar: SidebarWidgets,
    details: DetailsWidgets,
) -> (WindowInner, gtk4::Button, gtk4::Button, gtk4::MenuButton) {
    let habit_css_provider = gtk4::CssProvider::new();
    if let Some(display) = gtk4::gdk::Display::default() {
        let base_css_provider = gtk4::CssProvider::new();
        base_css_provider.load_from_data(BASE_CSS);
        gtk4::style_context_add_provider_for_display(
            &display,
            &base_css_provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
        // Dynamic per-habit rules override base rules regardless of registration order.
        gtk4::style_context_add_provider_for_display(
            &display,
            &habit_css_provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION + 1,
        );
    }

    let lock_btn = sidebar.lock_btn.clone();
    let add_btn = sidebar.add_btn.clone();
    let menu_btn = sidebar.menu_btn;

    let inner = WindowInner {
        nav_view: nav_view.clone(),
        details_page: details.page,
        toast_overlay: toast_overlay.clone(),
        db: Rc::new(db),
        selected_habit_id: None,
        lock_btn: sidebar.lock_btn,
        add_btn: sidebar.add_btn,
        habits_list_box: sidebar.habits_list_box,
        habit_score_label: sidebar.habit_score_label,
        header_day_labels: sidebar.header_day_labels,
        habit_css_provider,
        title_label: details.title_label,
        schedule_label: details.schedule_label,
        target_label: details.target_label,
        edit_btn: details.edit_btn,
        delete_btn: details.delete_btn,
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

    (inner, lock_btn, add_btn, menu_btn)
}

/// Main application window providing Fast-Check List and push-navigation for details.
#[derive(Debug, Clone)]
pub struct HabitWindow {
    window: libadwaita::ApplicationWindow,
    nav_view: libadwaita::NavigationView,
    toast_overlay: libadwaita::ToastOverlay,
}

impl HabitWindow {
    /// Opens the application window using the default on-disk SQLite database path.
    ///
    /// # Panics
    /// Panics if opening an in-memory database fallback fails.
    #[must_use]
    pub fn open_default(app: &libadwaita::Application) -> Self {
        let db_path = default_database_path();
        match Database::open(&db_path) {
            Ok(db) => Self::new(app, db),
            Err(err) => {
                eprintln!("Failed to open database at '{}': {err}", db_path.display());
                let fallback_db = Database::open_in_memory()
                    .expect("In-memory database fallback should always succeed");
                let habit_window = Self::new(app, fallback_db);

                let alert = libadwaita::AlertDialog::builder()
                    .heading("Failed to Open Database")
                    .body(format!(
                        "Could not open habit database at:\n{}\n\nError: {err}\n\nDo you want to quit, or continue with a temporary in-memory database? Changes will NOT be saved to disk.",
                        db_path.display()
                    ))
                    .build();
                alert.add_response("quit", "Quit");
                alert.add_response("temporary", "Use Temporary In-Memory DB");
                alert.set_response_appearance(
                    "temporary",
                    libadwaita::ResponseAppearance::Destructive,
                );
                alert.set_default_response(Some("quit"));
                alert.set_close_response("quit");

                let win_clone = habit_window.window.clone();
                let toast_clone = habit_window.toast_overlay.clone();
                alert.choose(
                    &habit_window.window,
                    None::<&gtk4::gio::Cancellable>,
                    move |response| {
                        if response == "temporary" {
                            let toast = libadwaita::Toast::new(
                                "Using temporary in-memory database — data will NOT be saved",
                            );
                            toast.set_timeout(0);
                            toast_clone.add_toast(toast);
                        } else {
                            if let Some(app) = win_clone.application() {
                                app.quit();
                            }
                            win_clone.close();
                        }
                    },
                );

                habit_window
            }
        }
    }

    /// Creates and initializes the `HabitWindow` with the provided database instance.
    #[must_use]
    pub fn new(app: &libadwaita::Application, db: Database) -> Self {
        let window = libadwaita::ApplicationWindow::builder()
            .application(app)
            .title("Habits")
            .default_width(430)
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

        let today = chrono::Local::now().date_naive();
        let caches = DrawCaches::new(today);
        connect_drawing_areas(&details, &caches);

        let (inner, lock_btn, add_btn, menu_btn) =
            create_window_inner(&nav_view, &toast_overlay, db, sidebar, details);
        let inner_rc = Rc::new(RefCell::new(inner));
        let root_window: gtk4::Window = window.clone().upcast();

        connect_controllers(
            &inner_rc,
            &caches,
            &root_window,
            &root_overlay,
            &details_clone,
            &lock_btn,
            &add_btn,
        );

        setup_primary_menu(
            &window,
            &root_window,
            &inner_rc,
            &caches,
            &menu_btn,
            &toast_overlay,
        );

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

impl std::ops::Deref for HabitWindow {
    type Target = libadwaita::ApplicationWindow;

    fn deref(&self) -> &Self::Target {
        &self.window
    }
}

impl AsRef<gtk4::Widget> for HabitWindow {
    fn as_ref(&self) -> &gtk4::Widget {
        self.window.upcast_ref()
    }
}

fn acquire_instance_lock() -> Option<std::fs::File> {
    let lock_path = default_database_path().with_file_name("habit.lock");
    if let Some(parent) = lock_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .open(lock_path)
        .ok()?;
    file.try_lock().ok()?;
    Some(file)
}

/// Application entry point: initializes libadwaita and launches `HabitWindow`.
///
/// Configured to run exclusively on Wayland sessions with no X11/Xorg fallback,
/// and enforces a strict single-instance lock so only one window can ever run.
pub fn run_app() {
    std::env::set_var("GDK_BACKEND", "wayland");

    let app = libadwaita::Application::builder()
        .application_id(crate::APP_ID)
        .flags(gtk4::gio::ApplicationFlags::FLAGS_NONE)
        .build();

    if app.register(gtk4::gio::Cancellable::NONE).is_ok() && app.is_remote() {
        app.activate();
        return;
    }

    let Some(_instance_lock) = acquire_instance_lock() else {
        eprintln!("Habit is already running.");
        return;
    };

    app.connect_activate(|app| {
        if let Some(window) = app
            .active_window()
            .or_else(|| app.windows().into_iter().next())
        {
            window.present();
            return;
        }
        let window = HabitWindow::open_default(app);
        window.present();
    });

    app.run();
}
