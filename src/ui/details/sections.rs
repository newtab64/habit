use gtk4::prelude::*;

/// Creates a vertical stat column containing a value label and a caption title.
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

fn build_ring_box() -> (gtk4::Box, gtk4::DrawingArea) {
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

    (ring_box, ring_drawing_area)
}

/// Builds the overview stat card section with progress ring and statistics columns.
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

    let (ring_box, ring_drawing_area) = build_ring_box();
    stats_box.append(&ring_box);

    let score_card_value = gtk4::Label::new(Some("0%"));
    score_card_value.add_css_class("overview-accent");

    let month_diff_card_value = gtk4::Label::new(Some("+0%"));
    let year_diff_card_value = gtk4::Label::new(Some("+0%"));

    let total_count_card_value = gtk4::Label::new(Some("0"));
    total_count_card_value.add_css_class("overview-accent");

    stats_box.append(&create_stat_column(&score_card_value, "Score"));
    stats_box.append(&create_stat_column(&month_diff_card_value, "Month"));
    stats_box.append(&create_stat_column(&year_diff_card_value, "Year"));
    stats_box.append(&create_stat_column(&total_count_card_value, "Total"));

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

pub(super) fn build_details_header() -> (libadwaita::HeaderBar, gtk4::Button, gtk4::Button) {
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

    (content_header, edit_btn, delete_btn)
}

pub(super) fn build_info_section() -> (gtk4::Box, gtk4::Label, gtk4::Label, gtk4::Label) {
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

    (info_box, title_label, schedule_label, target_label)
}

pub(super) fn build_curve_section() -> (gtk4::Label, gtk4::DrawingArea) {
    let curve_header = gtk4::Label::new(Some("Score History (Last 30 Days)"));
    curve_header.add_css_class("heading");
    curve_header.set_halign(gtk4::Align::Start);

    let curve_drawing_area = gtk4::DrawingArea::new();
    curve_drawing_area.set_content_height(140);
    curve_drawing_area.set_hexpand(true);

    (curve_header, curve_drawing_area)
}
