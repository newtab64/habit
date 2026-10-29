//! History editor bottom sheet overlay for viewing and modifying habit completion history.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use chrono::Datelike;
use gtk4::prelude::*;

use crate::engine::models::EntryStatus;
use crate::ui::charts::{draw_loop_heatmap, loop_heatmap_hit_test};
use crate::ui::state::{refresh, DrawCaches, WindowInner};

/// Widgets comprising the history editor bottom sheet overlay.
#[derive(Clone)]
pub struct HistoryEditorWidgets {
    /// Outer container overlaying the details view with a dimming backdrop.
    pub container: gtk4::Box,
    /// Dimming background area responding to clicks to dismiss the editor.
    pub dimming: gtk4::Box,
    /// Bottom sheet slide-up revealer.
    pub sheet_revealer: gtk4::Revealer,
    /// Window title widget displaying the habit name.
    pub title: libadwaita::WindowTitle,
    /// Button to close/dismiss the history editor.
    pub close_btn: gtk4::Button,
    /// Drawing area rendering the interactive 104-week loop heatmap.
    pub drawing_area: gtk4::DrawingArea,
}

fn build_editor_header() -> (gtk4::CenterBox, libadwaita::WindowTitle, gtk4::Button) {
    let sheet_header = gtk4::CenterBox::new();
    sheet_header.add_css_class("editor-sheet-header");
    sheet_header.set_margin_start(16);
    sheet_header.set_margin_end(10);
    sheet_header.set_margin_top(12);
    sheet_header.set_margin_bottom(10);

    let editor_title = libadwaita::WindowTitle::new("Edit History", "");
    sheet_header.set_center_widget(Some(&editor_title));

    let editor_close_btn = gtk4::Button::from_icon_name("window-close-symbolic");
    editor_close_btn.add_css_class("flat");
    editor_close_btn.add_css_class("circular");
    editor_close_btn.set_tooltip_text(Some("Close"));
    sheet_header.set_end_widget(Some(&editor_close_btn));

    (sheet_header, editor_title, editor_close_btn)
}

fn build_editor_sheet(sheet_header: &gtk4::CenterBox) -> (gtk4::Box, gtk4::DrawingArea) {
    let sheet_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    sheet_box.set_valign(gtk4::Align::End);
    sheet_box.set_hexpand(true);
    sheet_box.add_css_class("editor-bottom-sheet");
    sheet_box.append(sheet_header);

    let editor_sep = gtk4::Separator::new(gtk4::Orientation::Horizontal);
    editor_sep.set_margin_bottom(4);
    sheet_box.append(&editor_sep);

    let sheet_content = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    sheet_content.set_margin_start(12);
    sheet_content.set_margin_end(12);
    sheet_content.set_margin_top(8);
    sheet_content.set_margin_bottom(12);

    let editor_drawing_area = gtk4::DrawingArea::new();
    editor_drawing_area.set_content_height(280);
    editor_drawing_area.set_hexpand(true);
    sheet_content.append(&editor_drawing_area);
    sheet_box.append(&sheet_content);

    (sheet_box, editor_drawing_area)
}

/// Builds the history editor bottom sheet overlay widgets.
#[must_use]
pub fn build_history_editor() -> HistoryEditorWidgets {
    let editor_container = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    editor_container.set_visible(false);
    editor_container.add_css_class("editor-dimming-backdrop");

    let editor_dimming = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    editor_dimming.set_vexpand(true);
    editor_dimming.set_hexpand(true);
    editor_container.append(&editor_dimming);

    let (sheet_header, editor_title, editor_close_btn) = build_editor_header();
    let (sheet_box, editor_drawing_area) = build_editor_sheet(&sheet_header);

    let sheet_revealer = gtk4::Revealer::new();
    sheet_revealer.set_transition_type(gtk4::RevealerTransitionType::SlideUp);
    sheet_revealer.set_transition_duration(180);
    sheet_revealer.set_child(Some(&sheet_box));
    sheet_revealer.set_valign(gtk4::Align::End);
    sheet_revealer.set_hexpand(true);
    sheet_revealer.set_reveal_child(false);

    let cont_on_hide = editor_container.clone();
    sheet_revealer.connect_child_revealed_notify(move |rev| {
        if !rev.is_child_revealed() {
            cont_on_hide.set_visible(false);
        }
    });

    editor_container.append(&sheet_revealer);

    HistoryEditorWidgets {
        container: editor_container,
        dimming: editor_dimming,
        sheet_revealer,
        title: editor_title,
        close_btn: editor_close_btn,
        drawing_area: editor_drawing_area,
    }
}

impl HistoryEditorWidgets {
    /// Opens the history editor bottom sheet for the currently selected habit.
    pub fn open(&self, inner_rc: &Rc<RefCell<WindowInner>>, caches: &DrawCaches) {
        let (habit_opt, initial_entries) = {
            let inner = inner_rc.borrow();
            let db = &inner.db;
            let habit_id = inner.selected_habit_id;
            let h = habit_id.and_then(|id| db.get_habit(id).ok().flatten());
            let today = chrono::Local::now().date_naive();
            let start_date = today - chrono::Duration::weeks(104);
            let e = habit_id
                .and_then(|id| db.get_entries(id, start_date, today).ok())
                .unwrap_or_default();
            (h, e)
        };
        if let Some(habit) = habit_opt {
            let Some(habit_id) = habit.id else { return };
            caches.editor_habit_id.set(Some(habit_id));
            caches.editor_week_offset.set(0);
            caches.editor_color.set(habit.accent_rgba());

            let today = chrono::Local::now().date_naive();
            let start_date = today - chrono::Duration::weeks(104);
            let mut map = HashMap::with_capacity(initial_entries.len() + 104 * 7);
            for e in initial_entries {
                map.insert(e.date, e.status);
            }
            let mut cur = start_date;
            while cur <= today {
                if !map.contains_key(&cur) && !habit.is_scheduled_on(cur.weekday()) {
                    map.insert(cur, EntryStatus::Skipped);
                }
                cur += chrono::Duration::days(1);
            }
            *caches.editor_entries.borrow_mut() = map;

            self.title.set_title(&habit.name);
            self.drawing_area.queue_draw();
            self.container.set_visible(true);
            self.sheet_revealer.set_reveal_child(true);
        }
    }

    /// Closes the history editor bottom sheet and triggers state refresh.
    pub fn close(&self, inner_rc: &Rc<RefCell<WindowInner>>, caches: &DrawCaches) {
        if self.container.is_visible() {
            self.sheet_revealer.set_reveal_child(false);
            if !self.sheet_revealer.is_child_revealed() {
                self.container.set_visible(false);
            }
            refresh(inner_rc, caches);
        }
    }

    /// Connects the drawing callback to render the 104-week heatmap.
    pub fn connect_drawing_area(&self, caches: &DrawCaches) {
        let editor_entries_draw = Rc::clone(&caches.editor_entries);
        let editor_color_draw = Rc::clone(&caches.editor_color);
        let editor_offset_draw = Rc::clone(&caches.editor_week_offset);
        self.drawing_area
            .set_draw_func(move |_area, cr, width, height| {
                let entries = editor_entries_draw.borrow();
                let color = editor_color_draw.get();
                let week_offset = editor_offset_draw.get();
                let today = chrono::Local::now().date_naive();
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
    }

    /// Connects user interaction controllers: close button, dimming tap, scroll gesture,
    /// heatmap date hit testing, and the Escape key on the root window.
    pub fn connect_controllers(
        &self,
        inner_rc: &Rc<RefCell<WindowInner>>,
        caches: &DrawCaches,
        root_window: &gtk4::Window,
    ) {
        let editor_close = self.clone();
        let inner_close = Rc::clone(inner_rc);
        let caches_close = caches.clone();
        self.close_btn.connect_clicked(move |_| {
            editor_close.close(&inner_close, &caches_close);
        });

        let editor_dim = self.clone();
        let inner_dim = Rc::clone(inner_rc);
        let caches_dim = caches.clone();
        let dimming_gesture = gtk4::GestureClick::new();
        dimming_gesture.connect_pressed(move |_, _, _, _| {
            editor_dim.close(&inner_dim, &caches_dim);
        });
        self.dimming.add_controller(dimming_gesture);

        let scroll = gtk4::EventControllerScroll::new(
            gtk4::EventControllerScrollFlags::VERTICAL
                | gtk4::EventControllerScrollFlags::HORIZONTAL,
        );
        let offset_scroll = Rc::clone(&caches.editor_week_offset);
        let area_scroll = self.drawing_area.clone();
        scroll.connect_scroll(move |_, dx, dy| {
            let delta = if dy.abs() > dx.abs() { dy } else { dx };
            let off = offset_scroll.get();
            if delta > 0.0 {
                offset_scroll.set((off + 2).min(104));
            } else if delta < 0.0 {
                offset_scroll.set(off.saturating_sub(2));
            }
            area_scroll.queue_draw();
            gtk4::glib::Propagation::Stop
        });
        self.drawing_area.add_controller(scroll);

        self.connect_hit_test_controller(inner_rc, caches);

        let key_controller = gtk4::EventControllerKey::new();
        let editor_esc = self.clone();
        let inner_esc = Rc::clone(inner_rc);
        let caches_esc = caches.clone();
        key_controller.connect_key_pressed(move |_, key, _, _| {
            if key == gtk4::gdk::Key::Escape && editor_esc.container.is_visible() {
                editor_esc.close(&inner_esc, &caches_esc);
                gtk4::glib::Propagation::Stop
            } else {
                gtk4::glib::Propagation::Proceed
            }
        });
        root_window.add_controller(key_controller);
    }

    fn connect_hit_test_controller(
        &self,
        inner_rc: &Rc<RefCell<WindowInner>>,
        caches: &DrawCaches,
    ) {
        let click_gesture = gtk4::GestureClick::new();
        let offset_click = Rc::clone(&caches.editor_week_offset);
        let entries_click = Rc::clone(&caches.editor_entries);
        let habit_id_click = Rc::clone(&caches.editor_habit_id);
        let area_click = self.drawing_area.clone();
        let inner_click = Rc::clone(inner_rc);
        let caches_click = caches.clone();
        click_gesture.connect_pressed(move |_, _, x, y| {
            let Some(habit_id) = habit_id_click.get() else {
                return;
            };
            let habit = {
                let inner = inner_click.borrow();
                inner.db.get_habit(habit_id).ok().flatten()
            };
            let Some(habit) = habit else { return };

            let width = f64::from(area_click.width());
            let height = f64::from(area_click.height());
            let today = chrono::Local::now().date_naive();
            let offset = offset_click.get();
            if let Some(clicked_date) = loop_heatmap_hit_test(x, y, width, height, today, offset) {
                if clicked_date <= today {
                    handle_history_click(
                        &habit,
                        clicked_date,
                        &entries_click,
                        &area_click,
                        &inner_click,
                        &caches_click,
                    );
                }
            }
        });
        self.drawing_area.add_controller(click_gesture);
    }
}

const fn next_entry_state(
    is_scheduled: bool,
    cur_status: Option<EntryStatus>,
    target_val: f64,
) -> (Option<EntryStatus>, f64, bool) {
    if is_scheduled {
        match cur_status {
            None | Some(EntryStatus::Missed) => (Some(EntryStatus::Completed), target_val, false),
            Some(EntryStatus::Completed) => (Some(EntryStatus::Skipped), 0.0, false),
            Some(EntryStatus::Skipped) => (None, 0.0, true),
        }
    } else {
        match cur_status {
            Some(EntryStatus::Skipped) | None => (Some(EntryStatus::Completed), target_val, false),
            Some(EntryStatus::Completed) => (Some(EntryStatus::Missed), 0.0, false),
            Some(EntryStatus::Missed) => (Some(EntryStatus::Skipped), 0.0, true),
        }
    }
}

fn handle_history_click(
    habit: &crate::engine::models::Habit,
    clicked_date: chrono::NaiveDate,
    entries_click: &RefCell<HashMap<chrono::NaiveDate, EntryStatus>>,
    area_click: &gtk4::DrawingArea,
    inner_click: &Rc<RefCell<WindowInner>>,
    caches_click: &DrawCaches,
) {
    let Some(habit_id) = habit.id else { return };
    let is_scheduled = habit.is_scheduled_on(clicked_date.weekday());
    let cur_status = entries_click.borrow().get(&clicked_date).copied();
    let target_val = habit.habit_type.target().unwrap_or(1.0);

    let (new_status, new_val, delete_db) = next_entry_state(is_scheduled, cur_status, target_val);

    let write_res = {
        let inner = inner_click.borrow();
        let db = &inner.db;
        if delete_db {
            db.delete_entry(habit_id, clicked_date)
        } else if let Some(status) = new_status {
            db.log_entry(habit_id, clicked_date, status, new_val)
        } else {
            Ok(())
        }
    };

    match write_res {
        Ok(()) => {
            if let Some(status) = new_status {
                entries_click.borrow_mut().insert(clicked_date, status);
            } else {
                entries_click.borrow_mut().remove(&clicked_date);
            }

            area_click.queue_draw();
            refresh(inner_click, caches_click);
        }
        Err(e) => {
            eprintln!("Failed to update habit entry: {e}");
            let inner = inner_click.borrow();
            inner
                .toast_overlay
                .add_toast(libadwaita::Toast::new(&format!(
                    "Failed to save history: {e}"
                )));
        }
    }
}
