use crate::engine::models::{EntryStatus, Habit, HabitEntry, HabitType};
use chrono::{Datelike, NaiveDate};

#[cfg(feature = "ui")]
use gtk4::prelude::*;

/// Number of days displayed in the quick-check row (today plus previous 6 days).
pub const QUICK_CHECK_DAYS: usize = 7;

/// Visual display attributes for a single day column in a habit row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaySymbolDisplay {
    /// Character or formatted string to display (e.g. "✓", "—", "✕", or "30").
    pub text: String,
    /// Scoped CSS class to apply for color and styling.
    pub css_class: &'static str,
    /// Whether the symbol should be highlighted with the habit's accent color.
    pub is_accent: bool,
}

/// Calculates the array of 7 consecutive dates ending on `today` (chronological left-to-right: today-6 to today).
#[must_use]
pub fn calculate_7_day_dates(today: NaiveDate) -> [NaiveDate; 7] {
    [
        today
            .checked_sub_signed(chrono::Duration::days(6))
            .unwrap_or(today),
        today
            .checked_sub_signed(chrono::Duration::days(5))
            .unwrap_or(today),
        today
            .checked_sub_signed(chrono::Duration::days(4))
            .unwrap_or(today),
        today
            .checked_sub_signed(chrono::Duration::days(3))
            .unwrap_or(today),
        today
            .checked_sub_signed(chrono::Duration::days(2))
            .unwrap_or(today),
        today
            .checked_sub_signed(chrono::Duration::days(1))
            .unwrap_or(today),
        today,
    ]
}

/// Cycles to the next entry status in the sequence:
/// `None / Missed` -> `Completed` -> `Skipped` -> `Missed`.
#[must_use]
pub const fn next_toggle_status(current: Option<EntryStatus>) -> EntryStatus {
    match current {
        None | Some(EntryStatus::Missed) => EntryStatus::Completed,
        Some(EntryStatus::Completed) => EntryStatus::Skipped,
        Some(EntryStatus::Skipped) => EntryStatus::Missed,
    }
}

/// Formats a floating-point measurable value without unnecessary trailing `.0` or decimals.
///
/// For example, `30.0` becomes `"30"`, and `1.5` becomes `"1.5"`.
#[must_use]
pub fn format_measurable_value(val: f64) -> String {
    if val.is_nan() || val.is_infinite() {
        return "0".to_string();
    }
    if (val - val.round()).abs() < 1e-6 {
        format!("{:.0}", val.round())
    } else {
        let s = format!("{val:.2}");
        let trimmed = s.trim_end_matches('0').trim_end_matches('.');
        trimmed.to_string()
    }
}

/// Formats the bare symbol display attributes for a habit on a specific date.
///
/// Rules:
/// - Explicit manual skip or pre-defined rest day without completion: `"—"`, `symbol-rest`, accent.
/// - Binary completed: `"✓"`, `symbol-completed`, accent.
/// - Measurable completed ($v \ge \text{target}$): formatted number string, `symbol-completed`, accent.
/// - Measurable partial ($0 < v < \text{target}$): formatted number string, `symbol-partial`, muted.
/// - Missed or unlogged past scheduled day: `"✕"`, `symbol-missed`, muted.
#[must_use]
pub fn format_day_symbol(
    habit: &Habit,
    entry: Option<&HabitEntry>,
    date: NaiveDate,
) -> DaySymbolDisplay {
    let is_scheduled = habit.is_scheduled_on(date.weekday());

    // Explicit manual skip always displays as rest pause symbol
    if let Some(e) = entry {
        if e.status == EntryStatus::Skipped {
            return DaySymbolDisplay {
                text: "⏸".to_string(),
                css_class: "symbol-rest",
                is_accent: true,
            };
        }
    }

    match &habit.habit_type {
        HabitType::Binary => {
            if let Some(e) = entry {
                if e.status == EntryStatus::Completed {
                    return DaySymbolDisplay {
                        text: "✓".to_string(),
                        css_class: "symbol-completed",
                        is_accent: true,
                    };
                }
            }
        }
        HabitType::Measurable { target, .. } => {
            if let Some(e) = entry {
                if e.value >= *target && (e.status == EntryStatus::Completed || e.value > 0.0) {
                    return DaySymbolDisplay {
                        text: format_measurable_value(e.value),
                        css_class: "symbol-completed",
                        is_accent: true,
                    };
                }
                if e.value > 0.0 {
                    return DaySymbolDisplay {
                        text: format_measurable_value(e.value),
                        css_class: "symbol-partial",
                        is_accent: false,
                    };
                }
            }
        }
    }

    if is_scheduled {
        DaySymbolDisplay {
            text: "✕".to_string(),
            css_class: "symbol-missed",
            is_accent: false,
        }
    } else {
        DaySymbolDisplay {
            text: "—".to_string(),
            css_class: "symbol-rest",
            is_accent: true,
        }
    }
}

/// Generates scoped CSS for `.habit-row-{habit_id}` tinting symbols and progress bar fill.
#[must_use]
pub fn generate_row_css(habit_id: i64, hex_color: &str) -> String {
    let color = if hex_color.starts_with('#') {
        hex_color.to_string()
    } else {
        format!("#{hex_color}")
    };

    format!(
        ".habit-row-{habit_id} .symbol-completed,\n\
         .habit-row-{habit_id} .symbol-rest {{\n  \
             color: {color};\n\
         }}\n\
         .habit-row-{habit_id} .symbol-missed,\n\
         .habit-row-{habit_id} .symbol-partial {{\n  \
             color: #77767b;\n  \
             opacity: 0.55;\n\
         }}\n\
         .habit-row-{habit_id} .habit-progress progress {{\n  \
             background-color: {color};\n  \
             min-height: 5px;\n  \
             height: 5px;\n  \
             border-radius: 3px;\n\
         }}\n\
         .habit-row-{habit_id} .habit-progress trough {{\n  \
             min-height: 5px;\n  \
             height: 5px;\n  \
             border-radius: 3px;\n\
         }}\n\
         .habit-row-{habit_id} .day-symbol-button {{\n  \
             border: none;\n  \
             background: none;\n  \
             box-shadow: none;\n  \
             min-width: 36px;\n  \
             width: 36px;\n  \
             height: 36px;\n  \
             padding: 0;\n\
         }}\n"
    )
}

#[cfg(feature = "ui")]
/// GTK4 widget representing a single habit row in the fast-check list.
#[derive(Debug, Clone)]
pub struct HabitRowWidget {
    /// The root horizontal box containing title, progress, and 7-day checklist.
    pub container: gtk4::Box,
    /// Left box containing title and progress bar.
    pub left_box: gtk4::Box,
    /// The habit title label.
    pub title_label: gtk4::Label,
    /// Slim 5px progress bar indicating current habit score.
    pub progress_bar: gtk4::ProgressBar,
    /// Horizontal box containing the 7 day bare symbol action buttons.
    pub days_box: gtk4::Box,
    /// The 7 day action buttons in chronological order.
    pub day_buttons: Vec<gtk4::Button>,
}

#[cfg(feature = "ui")]
fn create_left_pane<F: Fn(i64) + 'static>(
    name: &str,
    strength: f64,
    habit_id: i64,
    on_select: F,
) -> (gtk4::Box, gtk4::Label, gtk4::ProgressBar) {
    let left_box = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
    left_box.set_hexpand(true);
    left_box.set_valign(gtk4::Align::Center);

    let title_label = gtk4::Label::new(Some(name));
    title_label.set_halign(gtk4::Align::Start);
    title_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    title_label.add_css_class("habit-title");
    left_box.append(&title_label);

    let progress_bar = gtk4::ProgressBar::new();
    progress_bar.set_fraction(strength.clamp(0.0, 1.0));
    progress_bar.set_show_text(false);
    progress_bar.add_css_class("habit-progress");
    left_box.append(&progress_bar);

    let select_gesture = gtk4::GestureClick::new();
    select_gesture.connect_pressed(move |_, _, _, _| {
        on_select(habit_id);
    });
    left_box.add_controller(select_gesture);

    (left_box, title_label, progress_bar)
}

#[cfg(feature = "ui")]
fn attach_measurable_popover<F: Fn(i64, NaiveDate, Option<f64>) + 'static>(
    parent: &gtk4::Button,
    habit: &Habit,
    entry: Option<&HabitEntry>,
    habit_id: i64,
    date: NaiveDate,
    on_toggle: std::rc::Rc<F>,
) {
    let popover = gtk4::Popover::new();
    popover.set_parent(parent);

    let pop_box = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
    pop_box.set_margin_start(10);
    pop_box.set_margin_end(10);
    pop_box.set_margin_top(10);
    pop_box.set_margin_bottom(10);

    let current_val = entry.map_or(0.0, |e| e.value);
    let target = habit.habit_type.target().unwrap_or(1.0);
    let unit = habit.habit_type.unit().unwrap_or("");

    let prompt_label = gtk4::Label::new(Some(&format!(
        "Target: {} {}",
        format_measurable_value(target),
        unit
    )));
    prompt_label.add_css_class("dim-label");
    pop_box.append(&prompt_label);

    let adjustment = gtk4::Adjustment::new(current_val, 0.0, 100_000.0, 1.0, 5.0, 0.0);
    let spin = gtk4::SpinButton::new(Some(&adjustment), 1.0, 1);
    pop_box.append(&spin);

    let log_btn = gtk4::Button::with_label("Log");
    log_btn.add_css_class("suggested-action");

    let pop_close = popover.clone();
    log_btn.connect_clicked(move |_| {
        let val = spin.value();
        pop_close.popdown();
        on_toggle(habit_id, date, Some(val));
    });
    pop_box.append(&log_btn);

    popover.set_child(Some(&pop_box));

    parent.connect_clicked(move |_| {
        popover.popup();
    });
}

#[cfg(feature = "ui")]
impl HabitRowWidget {
    /// Creates and configures a new `HabitRowWidget`.
    ///
    /// # Arguments
    /// * `habit` - The habit definition.
    /// * `score` - Current habit score in range $[0.0, 1.0]$.
    /// * `entries` - Historical entries for this habit.
    /// * `today` - The reference calendar date.
    /// * `on_toggle` - Callback invoked when a day symbol is clicked / logged: `(habit_id, date, Option<value>)`.
    /// * `on_select` - Callback invoked when the habit title or row is selected: `(habit_id)`.
    #[allow(deprecated)]
    pub fn new<F1, F2>(
        habit: &Habit,
        score: f64,
        entries: &[HabitEntry],
        today: NaiveDate,
        on_toggle: F1,
        on_select: F2,
    ) -> Self
    where
        F1: Fn(i64, NaiveDate, Option<f64>) + 'static,
        F2: Fn(i64) + 'static,
    {
        let habit_id = habit.id.unwrap_or(0);
        let on_toggle = std::rc::Rc::new(on_toggle);

        // Main horizontal container
        let container = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
        container.add_css_class("habit-row");
        let scoped_class = format!("habit-row-{habit_id}");
        container.add_css_class(&scoped_class);

        // Apply custom scoped CSS provider to this row widget
        let css = generate_row_css(habit_id, &habit.accent_color);
        let provider = gtk4::CssProvider::new();
        provider.load_from_data(&css);
        container
            .style_context()
            .add_provider(&provider, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION);

        let (left_box, title_label, progress_bar) =
            create_left_pane(&habit.name, score, habit_id, on_select);
        container.append(&left_box);

        // Right box: 7 day symbol columns
        let days_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        days_box.set_halign(gtk4::Align::End);
        days_box.set_valign(gtk4::Align::Center);

        let dates = calculate_7_day_dates(today);
        let is_measurable = habit.habit_type.is_measurable();
        let mut day_buttons = Vec::with_capacity(7);

        for date in dates {
            let entry = entries.iter().find(|e| e.date == date);
            let display = format_day_symbol(habit, entry, date);

            let day_btn = gtk4::Button::new();
            day_btn.add_css_class("flat");
            day_btn.add_css_class("day-symbol-button");
            day_btn.set_size_request(36, 36);

            let day_label = gtk4::Label::new(Some(&display.text));
            day_label.add_css_class("day-symbol");
            day_label.add_css_class(display.css_class);
            if !display.is_accent {
                day_label.add_css_class("dim-label");
            }
            day_label.set_halign(gtk4::Align::Center);
            day_label.set_valign(gtk4::Align::Center);
            day_btn.set_child(Some(&day_label));

            if is_measurable {
                attach_measurable_popover(
                    &day_btn,
                    habit,
                    entry,
                    habit_id,
                    date,
                    on_toggle.clone(),
                );
            } else {
                let on_toggle_clone = on_toggle.clone();
                day_btn.connect_clicked(move |_| {
                    on_toggle_clone(habit_id, date, None);
                });

                let on_toggle_secondary = on_toggle.clone();
                let right_click = gtk4::GestureClick::new();
                right_click.set_button(3);
                right_click.connect_pressed(move |_, _, _, _| {
                    on_toggle_secondary(habit_id, date, Some(-1.0));
                });
                day_btn.add_controller(right_click);
            }

            day_buttons.push(day_btn.clone());
            days_box.append(&day_btn);
        }

        container.append(&days_box);

        Self {
            container,
            left_box,
            title_label,
            progress_bar,
            days_box,
            day_buttons,
        }
    }

    /// Returns a reference to the root `gtk4::Box` widget.
    #[must_use]
    pub const fn widget(&self) -> &gtk4::Box {
        &self.container
    }
}

#[cfg(feature = "ui")]
impl std::ops::Deref for HabitRowWidget {
    type Target = gtk4::Box;

    fn deref(&self) -> &Self::Target {
        &self.container
    }
}

#[cfg(feature = "ui")]
impl AsRef<gtk4::Widget> for HabitRowWidget {
    fn as_ref(&self) -> &gtk4::Widget {
        self.container.upcast_ref()
    }
}
