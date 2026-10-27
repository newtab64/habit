//! Habit details view displaying statistics, streak metrics, history curve, and rolling heatmap.

#[cfg(feature = "ui")]
use std::cell::RefCell;
#[cfg(feature = "ui")]
use std::rc::Rc;

#[cfg(feature = "ui")]
use gtk4::prelude::*;
#[cfg(feature = "ui")]
use libadwaita::prelude::*;

#[cfg(feature = "ui")]
use crate::ui::charts::{draw_loop_heatmap, draw_progress_ring, draw_strength_curve};
#[cfg(feature = "ui")]
use crate::ui::habit_dialog::show_habit_dialog;
#[cfg(feature = "ui")]
use crate::ui::history_editor::{build_history_editor, HistoryEditorWidgets};
#[cfg(feature = "ui")]
use crate::ui::state::{refresh, DrawCaches, WindowInner};

/// Widgets comprising the habit details page and history editor overlay.
#[cfg(feature = "ui")]
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

/// Creates a vertical stat column containing a value label and a caption title.
#[cfg(feature = "ui")]
#[must_use]
pub fn create_stat_column(value_label: &gtk4::Label, title: &str) -> gtk4::Box {
    let col = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
    col.set_hexpand(true);
    col.set_halign(gtk4::Align::Fill);
    col.set_valign(gtk4::Align::Center);

    value_label.add_css_class("title-2");
    value_label.set_halign(gtk4::Align::Center);
    value_label.set_single_line_mode(true);
    col.append(value_label);

    let title_lbl = gtk4::Label::new(Some(title));
    title_lbl.add_css_class("dim-label");
    title_lbl.add_css_class("caption");
    title_lbl.set_halign(gtk4::Align::Center);
    title_lbl.set_single_line_mode(true);
    col.append(&title_lbl);

    col
}

/// Builds the overview stat card section with progress ring and statistics columns.
#[cfg(feature = "ui")]
#[must_use]
pub fn build_overview_section() -> (
    gtk4::Box,
    gtk4::DrawingArea,
    gtk4::Label,
    gtk4::Label,
    gtk4::Label,
    gtk4::Label,
) {
    let card = gtk4::Box::new(gtk4::Orientation::Vertical, 10);
    card.add_css_class("card");
    card.set_margin_start(2);
    card.set_margin_end(2);
    card.set_margin_top(2);
    card.set_margin_bottom(2);

    let overview_title = gtk4::Label::new(Some("Overview"));
    overview_title.add_css_class("title-3");
    overview_title.add_css_class("overview-accent");
    overview_title.set_halign(gtk4::Align::Start);
    overview_title.set_margin_start(14);
    overview_title.set_margin_top(12);
    card.append(&overview_title);

    let stats_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    stats_box.set_homogeneous(true);
    stats_box.set_margin_start(8);
    stats_box.set_margin_end(8);
    stats_box.set_margin_bottom(12);
    stats_box.set_margin_top(2);

    let ring_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    ring_box.set_valign(gtk4::Align::Center);
    ring_box.set_halign(gtk4::Align::Fill);
    ring_box.set_hexpand(true);

    let ring_drawing_area = gtk4::DrawingArea::new();
    ring_drawing_area.set_content_width(38);
    ring_drawing_area.set_content_height(38);
    ring_drawing_area.set_halign(gtk4::Align::Center);
    ring_drawing_area.set_valign(gtk4::Align::Center);
    ring_box.append(&ring_drawing_area);
    stats_box.append(&ring_box);

    let score_card_value = gtk4::Label::new(Some("0%"));
    score_card_value.add_css_class("overview-accent");

    let month_diff_card_value = gtk4::Label::new(Some("+0%"));
    let year_diff_card_value = gtk4::Label::new(Some("+0%"));

    let total_count_card_value = gtk4::Label::new(Some("0"));
    total_count_card_value.add_css_class("overview-accent");

    let col_score = create_stat_column(&score_card_value, "Score");
    let col_month = create_stat_column(&month_diff_card_value, "Month");
    let col_year = create_stat_column(&year_diff_card_value, "Year");
    let col_total = create_stat_column(&total_count_card_value, "Total");

    stats_box.append(&col_score);
    stats_box.append(&col_month);
    stats_box.append(&col_year);
    stats_box.append(&col_total);

    card.append(&stats_box);

    (
        card,
        ring_drawing_area,
        score_card_value,
        month_diff_card_value,
        year_diff_card_value,
        total_count_card_value,
    )
}

/// Builds the history heatmap header navigation bar and drawing area.
#[cfg(feature = "ui")]
#[must_use]
pub fn build_heatmap_section() -> (
    gtk4::Box,
    gtk4::Button,
    gtk4::Button,
    gtk4::Button,
    gtk4::Button,
    gtk4::DrawingArea,
) {
    let heatmap_nav_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    heatmap_nav_box.set_margin_top(4);
    heatmap_nav_box.set_margin_bottom(4);

    let heatmap_title = gtk4::Label::new(Some("History Heatmap"));
    heatmap_title.add_css_class("heading");
    heatmap_title.set_halign(gtk4::Align::Start);
    heatmap_title.set_hexpand(true);
    heatmap_nav_box.append(&heatmap_title);

    let edit_mode_btn = gtk4::Button::from_icon_name("document-edit-symbolic");
    edit_mode_btn.set_tooltip_text(Some("Edit Today's Status"));
    heatmap_nav_box.append(&edit_mode_btn);

    let nav_buttons = gtk4::Box::new(gtk4::Orientation::Horizontal, 4);
    nav_buttons.add_css_class("linked");

    let prev_week_btn = gtk4::Button::from_icon_name("go-previous-symbolic");
    prev_week_btn.set_tooltip_text(Some("Older Weeks"));
    nav_buttons.append(&prev_week_btn);

    let today_week_btn = gtk4::Button::with_label("Today");
    today_week_btn.set_tooltip_text(Some("Jump to Current"));
    nav_buttons.append(&today_week_btn);

    let next_week_btn = gtk4::Button::from_icon_name("go-next-symbolic");
    next_week_btn.set_tooltip_text(Some("Newer Weeks"));
    nav_buttons.append(&next_week_btn);

    heatmap_nav_box.append(&nav_buttons);

    let heatmap_drawing_area = gtk4::DrawingArea::new();
    heatmap_drawing_area.set_content_height(220);
    heatmap_drawing_area.set_hexpand(true);

    (
        heatmap_nav_box,
        prev_week_btn,
        today_week_btn,
        next_week_btn,
        edit_mode_btn,
        heatmap_drawing_area,
    )
}

/// Builds the habit details page and embeds the history editor overlay.
#[cfg(feature = "ui")]
#[allow(clippy::too_many_lines)]
#[must_use]
pub fn build_details() -> DetailsWidgets {
    let content_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);

    let content_header = libadwaita::HeaderBar::new();
    let content_title = libadwaita::WindowTitle::new("Habit Details", "");
    content_header.set_title_widget(Some(&content_title));

    let edit_btn = gtk4::Button::from_icon_name("document-edit-symbolic");
    edit_btn.set_tooltip_text(Some("Edit Habit"));
    edit_btn.add_css_class("flat");
    content_header.pack_end(&edit_btn);

    let delete_btn = gtk4::Button::from_icon_name("user-trash-symbolic");
    delete_btn.set_tooltip_text(Some("Delete Habit"));
    delete_btn.add_css_class("flat");
    delete_btn.add_css_class("destructive-action");
    content_header.pack_end(&delete_btn);

    content_box.append(&content_header);

    let details_vbox = gtk4::Box::new(gtk4::Orientation::Vertical, 14);
    details_vbox.set_margin_start(18);
    details_vbox.set_margin_end(18);
    details_vbox.set_margin_top(14);
    details_vbox.set_margin_bottom(18);

    let info_box = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
    let title_label = gtk4::Label::new(None);
    title_label.add_css_class("title-1");
    title_label.set_halign(gtk4::Align::Start);
    info_box.append(&title_label);

    let schedule_label = gtk4::Label::new(None);
    schedule_label.add_css_class("dim-label");
    schedule_label.set_halign(gtk4::Align::Start);
    info_box.append(&schedule_label);

    let target_label = gtk4::Label::new(None);
    target_label.add_css_class("dim-label");
    target_label.set_halign(gtk4::Align::Start);
    info_box.append(&target_label);

    details_vbox.append(&info_box);

    let sep2 = gtk4::Separator::new(gtk4::Orientation::Horizontal);
    details_vbox.append(&sep2);

    let (
        overview_card,
        ring_drawing_area,
        score_card_value,
        month_diff_card_value,
        year_diff_card_value,
        total_count_card_value,
    ) = build_overview_section();
    details_vbox.append(&overview_card);

    let sep3 = gtk4::Separator::new(gtk4::Orientation::Horizontal);
    details_vbox.append(&sep3);

    let curve_header = gtk4::Label::new(Some("Score History (Last 30 Days)"));
    curve_header.add_css_class("heading");
    curve_header.set_halign(gtk4::Align::Start);
    details_vbox.append(&curve_header);

    let curve_drawing_area = gtk4::DrawingArea::new();
    curve_drawing_area.set_content_height(140);
    curve_drawing_area.set_hexpand(true);
    details_vbox.append(&curve_drawing_area);

    let sep4 = gtk4::Separator::new(gtk4::Orientation::Horizontal);
    details_vbox.append(&sep4);

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

    let details_scrolled = gtk4::ScrolledWindow::new();
    details_scrolled.set_vexpand(true);
    details_scrolled.set_hscrollbar_policy(gtk4::PolicyType::Never);
    details_scrolled.set_child(Some(&details_vbox));
    content_box.append(&details_scrolled);

    let overlay = gtk4::Overlay::new();
    overlay.set_child(Some(&content_box));

    let history_editor = build_history_editor();
    overlay.add_overlay(&history_editor.container);

    let page = libadwaita::NavigationPage::new(&overlay, "Details");
    page.set_tag(Some("details"));

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

/// Connects event handlers and dialog actions for the details view.
#[cfg(feature = "ui")]
#[allow(clippy::too_many_lines)]
pub fn connect_detail_actions(
    inner_rc: &Rc<RefCell<WindowInner>>,
    caches: &DrawCaches,
    root_window: &gtk4::Window,
    root_overlay: &gtk4::Overlay,
    details: &DetailsWidgets,
) {
    let inner_prev = Rc::clone(inner_rc);
    let caches_prev = caches.clone();
    details.prev_week_btn.connect_clicked(move |_| {
        {
            let mut inner = inner_prev.borrow_mut();
            inner.heatmap_week_offset += 12;
        }
        refresh(&inner_prev, &caches_prev);
    });

    let inner_today_btn = Rc::clone(inner_rc);
    let caches_today_btn = caches.clone();
    details.today_week_btn.connect_clicked(move |_| {
        {
            let mut inner = inner_today_btn.borrow_mut();
            inner.heatmap_week_offset = 0;
        }
        refresh(&inner_today_btn, &caches_today_btn);
    });

    let inner_next = Rc::clone(inner_rc);
    let caches_next = caches.clone();
    details.next_week_btn.connect_clicked(move |_| {
        {
            let mut inner = inner_next.borrow_mut();
            inner.heatmap_week_offset = inner.heatmap_week_offset.saturating_sub(12);
        }
        refresh(&inner_next, &caches_next);
    });

    let inner_edit_mode = Rc::clone(inner_rc);
    let caches_edit_mode = caches.clone();
    let history_editor = details.history_editor.clone();
    details.edit_mode_btn.connect_clicked(move |_| {
        history_editor.open(&inner_edit_mode, &caches_edit_mode);
    });

    details
        .history_editor
        .connect_controllers(inner_rc, caches, root_window);

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
                }
                {
                    let mut inner = inner_saved.borrow_mut();
                    inner.selected_habit_id = id_opt;
                }
                refresh(&inner_saved, &caches_saved);
            });
        }
    });

    let inner_delete = Rc::clone(inner_rc);
    let caches_delete = caches.clone();
    let win_delete = root_window.clone();
    details.delete_btn.connect_clicked(move |_| {
        let (selected_id, db) = {
            let inner = inner_delete.borrow();
            (inner.selected_habit_id, Rc::clone(&inner.db))
        };
        if let Some(id) = selected_id {
            let habit_name = db.get_habit(id).ok().flatten().map_or_else(|| "this habit".to_string(), |h| h.name);
            let alert = libadwaita::AlertDialog::new(
                Some(&format!("Delete \"{habit_name}\"?")),
                Some("All completion history and progress data for this habit will be permanently deleted. This action cannot be undone."),
            );
            alert.add_response("cancel", "Cancel");
            alert.add_response("delete", "Delete");
            alert.set_response_appearance("delete", libadwaita::ResponseAppearance::Destructive);
            alert.set_default_response(Some("cancel"));

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
                    }
                    gtk4::glib::idle_add_local_once(move || {
                        {
                            let mut inner = inner_cb.borrow_mut();
                            inner.selected_habit_id = None;
                            inner.nav_view.pop();
                        }
                        refresh(&inner_cb, &caches_cb);
                    });
                }
            });
        }
    });
}

/// Connects render callbacks to the details view drawing areas (ring, curve, heatmap, and history editor).
#[cfg(feature = "ui")]
pub fn connect_drawing_areas(details: &DetailsWidgets, caches: &DrawCaches) {
    let overview_score_draw = Rc::clone(&caches.overview_score);
    let overview_color_draw = Rc::clone(&caches.overview_color);
    details
        .ring_drawing_area
        .set_draw_func(move |_area, cr, width, height| {
            let score = *overview_score_draw.borrow();
            let color = overview_color_draw.borrow();
            draw_progress_ring(cr, f64::from(width), f64::from(height), score, &color);
        });

    let curve_data_draw = Rc::clone(&caches.curve_data);
    let curve_color_draw = Rc::clone(&caches.curve_color);
    details
        .curve_drawing_area
        .set_draw_func(move |_area, cr, width, height| {
            let data = curve_data_draw.borrow();
            let color = curve_color_draw.borrow();
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
            let color = heatmap_color_draw.borrow();
            let week_offset = *heatmap_offset_draw.borrow();
            let today = *heatmap_today_draw.borrow();
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
