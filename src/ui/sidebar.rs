//! Habit list sidebar component for the application window.

use gtk4::prelude::*;
use libadwaita::prelude::*;

use crate::ui::habit_row::{calculate_quick_check_dates, QUICK_CHECK_DAYS};

/// Holds UI widget handles comprising the habit list sidebar page.
#[derive(Debug, Clone)]
pub struct SidebarWidgets {
    /// Navigation page holding the sidebar content.
    pub page: libadwaita::NavigationPage,
    /// Box container containing habit row widgets.
    pub habits_list_box: gtk4::Box,
    /// Header label displaying "HABIT & SCORE".
    pub habit_score_label: gtk4::Label,
    /// Header labels for the 5 quick-check dates.
    pub header_day_labels: Vec<gtk4::Label>,
    /// Header bar button to lock/unlock habit reordering.
    pub lock_btn: gtk4::Button,
    /// Header bar button to add a new habit.
    pub add_btn: gtk4::Button,
    /// Header bar button to open the primary menu.
    pub menu_btn: gtk4::MenuButton,
}

fn build_sidebar_header() -> (
    libadwaita::HeaderBar,
    gtk4::Button,
    gtk4::Button,
    gtk4::MenuButton,
) {
    let sidebar_header = libadwaita::HeaderBar::new();
    let sidebar_title = libadwaita::WindowTitle::new("Habits", "");
    sidebar_header.set_title_widget(Some(&sidebar_title));

    let lock_btn = gtk4::Button::from_icon_name("changes-prevent-symbolic");
    lock_btn.set_tooltip_text(Some("Unlock to Reorder Habits"));
    lock_btn.add_css_class("flat");
    sidebar_header.pack_start(&lock_btn);

    let menu_btn = gtk4::MenuButton::new();
    menu_btn.set_icon_name("open-menu-symbolic");
    menu_btn.add_css_class("flat");
    sidebar_header.pack_end(&menu_btn);

    let add_btn = gtk4::Button::from_icon_name("list-add-symbolic");
    add_btn.set_tooltip_text(Some("New Habit"));
    add_btn.add_css_class("flat");
    sidebar_header.pack_end(&add_btn);

    (sidebar_header, lock_btn, add_btn, menu_btn)
}

fn build_quick_check_header() -> (gtk4::Box, gtk4::Label, Vec<gtk4::Label>) {
    let quick_check_header = gtk4::Box::new(gtk4::Orientation::Horizontal, 4);
    quick_check_header.set_margin_start(12);
    quick_check_header.set_margin_end(12);
    quick_check_header.set_margin_top(8);
    quick_check_header.set_margin_bottom(8);

    let habit_score_label = gtk4::Label::new(Some("HABIT & SCORE"));
    habit_score_label.set_halign(gtk4::Align::Start);
    habit_score_label.set_hexpand(true);
    habit_score_label.add_css_class("dim-label");
    habit_score_label.add_css_class("caption");
    quick_check_header.append(&habit_score_label);

    let days_header_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 4);
    days_header_box.set_halign(gtk4::Align::End);
    let today = chrono::Local::now().date_naive();
    let dates = calculate_quick_check_dates(today);
    let mut header_day_labels = Vec::with_capacity(QUICK_CHECK_DAYS);
    for date in dates {
        let date_str = date.format("%m/%d").to_string();
        let lbl = gtk4::Label::new(Some(&date_str));
        lbl.add_css_class("dim-label");
        lbl.add_css_class("caption");
        lbl.set_size_request(42, -1);
        lbl.set_halign(gtk4::Align::Center);
        days_header_box.append(&lbl);
        header_day_labels.push(lbl);
    }
    quick_check_header.append(&days_header_box);

    (quick_check_header, habit_score_label, header_day_labels)
}

/// Constructs the habit list sidebar page and returns handles to its interactive widgets.
#[must_use]
pub fn build_sidebar() -> SidebarWidgets {
    let sidebar_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);

    let (sidebar_header, lock_btn, add_btn, menu_btn) = build_sidebar_header();
    sidebar_box.append(&sidebar_header);

    let sidebar_scrolled = gtk4::ScrolledWindow::new();
    sidebar_scrolled.set_vexpand(true);
    sidebar_scrolled.set_hscrollbar_policy(gtk4::PolicyType::Never);

    let content_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    let (quick_check_header, habit_score_label, header_day_labels) = build_quick_check_header();
    content_box.append(&quick_check_header);
    content_box.append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));

    let habits_list_box = gtk4::Box::new(gtk4::Orientation::Vertical, 6);
    habits_list_box.set_margin_start(12);
    habits_list_box.set_margin_end(12);
    habits_list_box.set_margin_top(8);
    habits_list_box.set_margin_bottom(8);
    content_box.append(&habits_list_box);

    sidebar_scrolled.set_child(Some(&content_box));
    sidebar_box.append(&sidebar_scrolled);

    let page = libadwaita::NavigationPage::new(&sidebar_box, "Habits");
    page.set_tag(Some("habits"));

    SidebarWidgets {
        page,
        habits_list_box,
        habit_score_label,
        header_day_labels,
        lock_btn,
        add_btn,
        menu_btn,
    }
}
