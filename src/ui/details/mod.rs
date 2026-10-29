//! Habit details view displaying statistics, streak metrics, history curve, and rolling heatmap.

mod sections;

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use libadwaita::prelude::*;

use crate::ui::charts::{draw_loop_heatmap, draw_progress_ring, draw_strength_curve};
use crate::ui::habit_dialog::show_habit_dialog;
use crate::ui::history_editor::{build_history_editor, HistoryEditorWidgets};
use crate::ui::state::{refresh, DrawCaches, WindowInner};

use sections::{build_curve_section, build_details_header, build_info_section};
pub use sections::{build_heatmap_section, build_overview_section, create_stat_column};

/// Widgets comprising the habit details page and history editor overlay.
#[derive(Clone)]
pub struct DetailsWidgets {
    /// Navigation page wrapper for push/pop navigation.
    pub page: libadwaita::NavigationPage,
    /// Habit name title label.
    pub title_label: gtk4::Label,
    /// Schedule days subtitle label.
    pub schedule_label: gtk4::Label,
    /// Target description label.
    pub target_label: gtk4::Label,
    /// Button triggering edit habit dialog.
    pub edit_btn: gtk4::Button,
    /// Button triggering delete habit confirmation dialog.
    pub delete_btn: gtk4::Button,
    /// Drawing area rendering the overall progress ring.
    pub ring_drawing_area: gtk4::DrawingArea,
    /// Stat card displaying current score percentage.
    pub score_card_value: gtk4::Label,
    /// Stat card displaying month-over-month score diff.
    pub month_diff_card_value: gtk4::Label,
    /// Stat card displaying year-over-year score diff.
    pub year_diff_card_value: gtk4::Label,
    /// Stat card displaying total completion count.
    pub total_count_card_value: gtk4::Label,
    /// Drawing area rendering 30-day strength score history curve.
    pub curve_drawing_area: gtk4::DrawingArea,
    /// Button to pan history heatmap to older weeks.
    pub prev_week_btn: gtk4::Button,
    /// Button to jump history heatmap to current week.
    pub today_week_btn: gtk4::Button,
    /// Button to pan history heatmap to newer weeks.
    pub next_week_btn: gtk4::Button,
    /// Button to open the full history editor overlay.
    pub edit_mode_btn: gtk4::Button,
    /// Drawing area rendering the rolling habit heatmap.
    pub heatmap_drawing_area: gtk4::DrawingArea,
    /// History editor bottom sheet overlay widgets.
    pub history_editor: HistoryEditorWidgets,
}

fn wrap_details_page(
    content_header: &libadwaita::HeaderBar,
    details_vbox: &gtk4::Box,
) -> (libadwaita::NavigationPage, HistoryEditorWidgets) {
    let details_vbox_margins = 18;
    details_vbox.set_margin_start(details_vbox_margins);
    details_vbox.set_margin_end(details_vbox_margins);
    details_vbox.set_margin_top(14);
    details_vbox.set_margin_bottom(details_vbox_margins);

    let content_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    content_box.append(content_header);

    let details_scrolled = gtk4::ScrolledWindow::new();
    details_scrolled.set_vexpand(true);
    details_scrolled.set_hscrollbar_policy(gtk4::PolicyType::Never);
    details_scrolled.set_child(Some(details_vbox));
    content_box.append(&details_scrolled);

    let overlay = gtk4::Overlay::new();
    overlay.set_child(Some(&content_box));

    let history_editor = build_history_editor();
    overlay.add_overlay(&history_editor.container);

    let page = libadwaita::NavigationPage::new(&overlay, "Details");
    page.set_tag(Some("details"));

    (page, history_editor)
}

/// Builds the habit details page and embeds the history editor overlay.
#[must_use]
pub fn build_details() -> DetailsWidgets {
    let (content_header, edit_btn, delete_btn) = build_details_header();
    let details_vbox = gtk4::Box::new(gtk4::Orientation::Vertical, 14);

    let (info_box, title_label, schedule_label, target_label) = build_info_section();
    details_vbox.append(&info_box);
    details_vbox.append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));

    let (
        overview_card,
        ring_drawing_area,
        score_card_value,
        month_diff_card_value,
        year_diff_card_value,
        total_count_card_value,
    ) = build_overview_section();
    details_vbox.append(&overview_card);
    details_vbox.append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));

    let (curve_header, curve_drawing_area) = build_curve_section();
    details_vbox.append(&curve_header);
    details_vbox.append(&curve_drawing_area);
    details_vbox.append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));

    let (
        heatmap_nav_box,
        prev_week_btn,
        today_week_btn,
        next_week_btn,
        edit_mode_btn,
        heatmap_drawing_area,
    ) = build_heatmap_section();
    details_vbox.append(&heatmap_nav_box);
    details_vbox.append(&heatmap_drawing_area);

    let (page, history_editor) = wrap_details_page(&content_header, &details_vbox);

    DetailsWidgets {
        page,
        title_label,
        schedule_label,
        target_label,
        edit_btn,
        delete_btn,
        ring_drawing_area,
        score_card_value,
        month_diff_card_value,
        year_diff_card_value,
        total_count_card_value,
        curve_drawing_area,
        prev_week_btn,
        today_week_btn,
        next_week_btn,
        edit_mode_btn,
        heatmap_drawing_area,
        history_editor,
    }
}

fn connect_heatmap_navigation(
    details: &DetailsWidgets,
    inner_rc: &Rc<RefCell<WindowInner>>,
    caches: &DrawCaches,
) {
    let inner_prev = Rc::clone(inner_rc);
    let caches_prev = caches.clone();
    details.prev_week_btn.connect_clicked(move |_| {
        caches_prev
            .heatmap_week_offset
            .set(caches_prev.heatmap_week_offset.get() + 12);
        refresh(&inner_prev, &caches_prev);
    });

    let inner_today_btn = Rc::clone(inner_rc);
    let caches_today_btn = caches.clone();
    details.today_week_btn.connect_clicked(move |_| {
        caches_today_btn.heatmap_week_offset.set(0);
        refresh(&inner_today_btn, &caches_today_btn);
    });

    let inner_next = Rc::clone(inner_rc);
    let caches_next = caches.clone();
    details.next_week_btn.connect_clicked(move |_| {
        caches_next
            .heatmap_week_offset
            .set(caches_next.heatmap_week_offset.get().saturating_sub(12));
        refresh(&inner_next, &caches_next);
    });

    let inner_edit_mode = Rc::clone(inner_rc);
    let caches_edit_mode = caches.clone();
    let history_editor = details.history_editor.clone();
    details.edit_mode_btn.connect_clicked(move |_| {
        history_editor.open(&inner_edit_mode, &caches_edit_mode);
    });
}

fn connect_edit_action(
    details: &DetailsWidgets,
    inner_rc: &Rc<RefCell<WindowInner>>,
    caches: &DrawCaches,
    root_overlay: &gtk4::Overlay,
) {
    let inner_edit = Rc::clone(inner_rc);
    let caches_edit = caches.clone();
    let overlay_edit = root_overlay.clone();
    details.edit_btn.connect_clicked(move |_| {
        let (selected_id, db) = {
            let inner = inner_edit.borrow();
            (inner.selected_habit_id, Rc::clone(&inner.db))
        };
        let habit_opt = selected_id.and_then(|id| db.get_habit(id).ok().flatten());
        if let Some(habit) = habit_opt {
            let inner_saved = Rc::clone(&inner_edit);
            let caches_saved = caches_edit.clone();
            show_habit_dialog(&overlay_edit, Some(&habit), move |updated_habit| {
                let id_opt = updated_habit.id;
                let db = {
                    let inner = inner_saved.borrow();
                    Rc::clone(&inner.db)
                };
                if let Err(e) = db.update_habit(&updated_habit) {
                    eprintln!("Failed to update habit: {e}");
                    let toast_overlay = inner_saved.borrow().toast_overlay.clone();
                    toast_overlay.add_toast(libadwaita::Toast::new(&format!(
                        "Failed to update habit: {e}"
                    )));
                }
                {
                    let mut inner = inner_saved.borrow_mut();
                    inner.selected_habit_id = id_opt;
                }
                refresh(&inner_saved, &caches_saved);
            });
        }
    });
}

fn build_delete_alert(habit_name: &str) -> libadwaita::AlertDialog {
    let alert = libadwaita::AlertDialog::new(
        Some(&format!("Delete \"{habit_name}\"?")),
        Some(
            "All completion history and progress data for this habit will be permanently deleted. This action cannot be undone.",
        ),
    );
    alert.add_response("cancel", "Cancel");
    alert.add_response("delete", "Delete");
    alert.set_response_appearance("delete", libadwaita::ResponseAppearance::Destructive);
    alert.set_default_response(Some("cancel"));
    alert
}

fn connect_delete_action(
    details: &DetailsWidgets,
    inner_rc: &Rc<RefCell<WindowInner>>,
    caches: &DrawCaches,
    root_window: &gtk4::Window,
) {
    let inner_delete = Rc::clone(inner_rc);
    let caches_delete = caches.clone();
    let win_delete = root_window.clone();
    details.delete_btn.connect_clicked(move |_| {
        let (selected_id, db) = {
            let inner = inner_delete.borrow();
            (inner.selected_habit_id, Rc::clone(&inner.db))
        };
        if let Some(id) = selected_id {
            let habit_name = db
                .get_habit(id)
                .ok()
                .flatten()
                .map_or_else(|| "this habit".to_string(), |h| h.name);
            let alert = build_delete_alert(&habit_name);

            let inner_cb = Rc::clone(&inner_delete);
            let caches_cb = caches_delete.clone();
            alert.choose(&win_delete, gtk4::gio::Cancellable::NONE, move |response| {
                if response == "delete" {
                    let db = {
                        let inner = inner_cb.borrow();
                        Rc::clone(&inner.db)
                    };
                    if let Err(e) = db.delete_habit(id) {
                        eprintln!("Failed to delete habit: {e}");
                        let toast_overlay = inner_cb.borrow().toast_overlay.clone();
                        toast_overlay.add_toast(libadwaita::Toast::new(&format!(
                            "Failed to delete habit: {e}"
                        )));
                        return;
                    }
                    gtk4::glib::idle_add_local_once(move || {
                        let nav_view = {
                            let mut inner = inner_cb.borrow_mut();
                            inner.selected_habit_id = None;
                            inner.nav_view.clone()
                        };
                        nav_view.pop();
                        refresh(&inner_cb, &caches_cb);
                    });
                }
            });
        }
    });
}

/// Connects event handlers and dialog actions for the details view.
pub fn connect_detail_actions(
    inner_rc: &Rc<RefCell<WindowInner>>,
    caches: &DrawCaches,
    root_window: &gtk4::Window,
    root_overlay: &gtk4::Overlay,
    details: &DetailsWidgets,
) {
    connect_heatmap_navigation(details, inner_rc, caches);
    details
        .history_editor
        .connect_controllers(inner_rc, caches, root_window);
    connect_edit_action(details, inner_rc, caches, root_overlay);
    connect_delete_action(details, inner_rc, caches, root_window);
}

/// Connects render callbacks to the details view drawing areas (ring, curve, heatmap, and history editor).
pub fn connect_drawing_areas(details: &DetailsWidgets, caches: &DrawCaches) {
    let overview_score_draw = Rc::clone(&caches.overview_score);
    let overview_color_draw = Rc::clone(&caches.overview_color);
    details
        .ring_drawing_area
        .set_draw_func(move |_area, cr, width, height| {
            let score = overview_score_draw.get();
            let color = overview_color_draw.get();
            draw_progress_ring(cr, f64::from(width), f64::from(height), score, &color);
        });

    let curve_data_draw = Rc::clone(&caches.curve_data);
    let curve_color_draw = Rc::clone(&caches.curve_color);
    details
        .curve_drawing_area
        .set_draw_func(move |_area, cr, width, height| {
            let data = curve_data_draw.borrow();
            let color = curve_color_draw.get();
            draw_strength_curve(cr, f64::from(width), f64::from(height), &data, &color);
        });

    let heatmap_entries_draw = Rc::clone(&caches.heatmap_entries);
    let heatmap_color_draw = Rc::clone(&caches.heatmap_color);
    let heatmap_offset_draw = Rc::clone(&caches.heatmap_week_offset);
    let heatmap_today_draw = Rc::clone(&caches.heatmap_today);
    details
        .heatmap_drawing_area
        .set_draw_func(move |_area, cr, width, height| {
            let entries = heatmap_entries_draw.borrow();
            let color = heatmap_color_draw.get();
            let week_offset = heatmap_offset_draw.get();
            let today = heatmap_today_draw.get();
            draw_loop_heatmap(
                cr,
                f64::from(width),
                f64::from(height),
                today,
                week_offset,
                &entries,
                &color,
            );
        });

    details.history_editor.connect_drawing_area(caches);
}
