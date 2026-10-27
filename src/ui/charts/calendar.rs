use chrono::Datelike;

#[cfg(feature = "ui")]
use crate::engine::models::EntryStatus;
use super::geometry::Rect;
#[cfg(feature = "ui")]
use super::geometry::Rgba;
#[cfg(feature = "ui")]
use super::geometry::draw_rounded_rect;
use super::heatmap::HEATMAP_SPACING;

/// Number of columns in the monthly calendar heatmap (Monday through Sunday).
pub const HEATMAP_COLS: usize = 7;
/// Maximum number of rows required to display any month in a 7-column layout.
pub const HEATMAP_ROWS: usize = 6;
/// Outer padding around the heatmap canvas (in pixels).
pub const HEATMAP_PADDING: f64 = 8.0;
/// Top header height reserved for weekday label letters (M, T, W, T, F, S, S).
pub const HEATMAP_HEADER_HEIGHT: f64 = 20.0;

/// Returns the number of days in the specified month for a given year,
/// accounting for Gregorian leap years.
///
/// Returns `0` if `month` is not in `1..=12`.
#[must_use]
pub const fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

/// Returns the English name of the month (1 = January, 12 = December).
///
/// Returns an empty string if `month` is not in `1..=12`.
#[must_use]
pub const fn month_name(month: u32) -> &'static str {
    match month {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => "",
    }
}

/// Computes the bounding rectangle for a calendar cell at column `col` (0..=6)
/// and row `row` (0..=5) on a canvas of dimension `(width, height)`.
///
/// Guaranteed to produce geometrically square cells centered horizontally within the canvas.
#[must_use]
#[allow(clippy::cast_precision_loss, clippy::suboptimal_flops)]
pub fn heatmap_cell_rect(col: usize, row: usize, width: f64, height: f64) -> Rect {
    let total_spacing_x = HEATMAP_SPACING * 6.0;
    let total_spacing_y = HEATMAP_SPACING * 5.0;
    let avail_w = (width - 2.0 * HEATMAP_PADDING - total_spacing_x).max(0.0);
    let avail_h =
        (height - 2.0 * HEATMAP_PADDING - HEATMAP_HEADER_HEIGHT - total_spacing_y).max(0.0);
    let cell_w = avail_w / 7.0;
    let cell_h = avail_h / 6.0;
    let size = cell_w.min(cell_h).clamp(16.0, 36.0);

    let grid_w = 7.0 * size + total_spacing_x;
    let offset_x = ((width - grid_w) / 2.0).max(HEATMAP_PADDING);
    let offset_y = HEATMAP_PADDING + HEATMAP_HEADER_HEIGHT;

    let x = offset_x + (col as f64) * (size + HEATMAP_SPACING);
    let y = offset_y + (row as f64) * (size + HEATMAP_SPACING);

    Rect {
        x,
        y,
        width: size,
        height: size,
    }
}

/// Performs hit testing on the monthly calendar heatmap.
///
/// Maps `(x, y)` to the corresponding `chrono::NaiveDate` for the given month.
/// Returns `None` if `(x, y)` is outside the grid bounds, in the cell spacing gap,
/// before the 1st of the month, or after the last day of the month.
#[must_use]
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
pub fn heatmap_hit_test(
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    year: i32,
    month: u32,
) -> Option<chrono::NaiveDate> {
    if month == 0 || month > 12 || x < 0.0 || x > width || y < 0.0 || y > height {
        return None;
    }

    let first_date = chrono::NaiveDate::from_ymd_opt(year, month, 1)?;
    let start_col = first_date.weekday().num_days_from_monday() as usize;
    let days = days_in_month(year, month) as usize;

    for row in 0..HEATMAP_ROWS {
        for col in 0..HEATMAP_COLS {
            let rect = heatmap_cell_rect(col, row, width, height);
            if rect.contains(x, y) {
                let cell_index = row * HEATMAP_COLS + col;
                if cell_index < start_col {
                    return None;
                }
                let day_num = (cell_index - start_col) + 1;
                if day_num > days {
                    return None;
                }
                return chrono::NaiveDate::from_ymd_opt(year, month, day_num as u32);
            }
        }
    }

    None
}

/// Renders a monthly calendar heatmap using Cairo in Loop Habit Tracker style.
///
/// Draws a 7-column calendar matrix (Monday through Sunday) for the given month:
/// - Weekday headers row: `M`, `T`, `W`, `T`, `F`, `S`, `S`.
/// - Square cells with 4px rounded corners and day numbers (`1..31`).
/// - Completed days: filled rounded square in accent color with white text.
/// - Pre-defined rest / skipped days: muted accent fill with horizontal dash.
/// - Missed days: soft neutral card fill with muted text.
/// - Unreached future days: faint subtle background and dim text.
#[cfg(feature = "ui")]
#[allow(clippy::cast_possible_truncation, clippy::similar_names)]
pub fn draw_monthly_heatmap<S: std::hash::BuildHasher>(
    cr: &cairo::Context,
    width: f64,
    height: f64,
    year: i32,
    month: u32,
    entries: &std::collections::HashMap<chrono::NaiveDate, EntryStatus, S>,
    color: &Rgba,
) {
    if width <= 0.0 || height <= 0.0 || month == 0 || month > 12 {
        return;
    }

    let Some(first_date) = chrono::NaiveDate::from_ymd_opt(year, month, 1) else {
        return;
    };
    let start_col = first_date.weekday().num_days_from_monday() as usize;
    let days = days_in_month(year, month);
    let today = chrono::Local::now().date_naive();

    // 1. Draw Weekday column headers (M, T, W, T, F, S, S)
    let weekday_labels = ["M", "T", "W", "T", "F", "S", "S"];
    cr.set_font_size(10.0);
    cr.set_source_rgba(0.55, 0.55, 0.58, 0.85);

    for (col, label) in weekday_labels.iter().enumerate() {
        let header_rect = heatmap_cell_rect(col, 0, width, height);
        if let Ok(extents) = cr.text_extents(label) {
            let tx =
                header_rect.x + (header_rect.width - extents.width()) / 2.0 - extents.x_bearing();
            let ty = HEATMAP_PADDING + 13.0;
            cr.move_to(tx, ty);
            let _ = cr.show_text(label);
        }
    }

    // 2. Draw calendar day cells
    let cell_radius = 4.0;

    for d in 1..=days {
        let Some(date) = chrono::NaiveDate::from_ymd_opt(year, month, d) else {
            continue;
        };
        let offset = start_col + (d as usize - 1);
        let col = offset % HEATMAP_COLS;
        let row = offset / HEATMAP_COLS;
        let rect = heatmap_cell_rect(col, row, width, height);

        let text_color = match entries.get(&date) {
            Some(EntryStatus::Completed) => {
                // Completed day: solid accent fill
                cr.set_source_rgba(color.r, color.g, color.b, color.a);
                draw_rounded_rect(cr, rect.x, rect.y, rect.width, rect.height, cell_radius);
                let _ = cr.fill();
                Rgba::new(1.0, 1.0, 1.0, 1.0)
            }
            Some(EntryStatus::Skipped) => {
                // Rest day: muted accent fill without underline
                cr.set_source_rgba(color.r, color.g, color.b, 0.25);
                draw_rounded_rect(cr, rect.x, rect.y, rect.width, rect.height, cell_radius);
                let _ = cr.fill();
                Rgba::new(color.r, color.g, color.b, 0.95)
            }
            Some(EntryStatus::Missed) => {
                // Missed day: soft neutral card fill
                cr.set_source_rgba(0.22, 0.22, 0.24, 0.55);
                draw_rounded_rect(cr, rect.x, rect.y, rect.width, rect.height, cell_radius);
                let _ = cr.fill();
                Rgba::new(0.65, 0.65, 0.68, 0.8)
            }
            None => {
                if date > today {
                    // Future day: faint outline
                    cr.set_source_rgba(0.45, 0.45, 0.48, 0.12);
                    draw_rounded_rect(cr, rect.x, rect.y, rect.width, rect.height, cell_radius);
                    let _ = cr.fill();
                    Rgba::new(0.5, 0.5, 0.52, 0.35)
                } else {
                    // Past unlogged day: soft neutral fill
                    cr.set_source_rgba(0.22, 0.22, 0.24, 0.45);
                    draw_rounded_rect(cr, rect.x, rect.y, rect.width, rect.height, cell_radius);
                    let _ = cr.fill();
                    Rgba::new(0.55, 0.55, 0.58, 0.7)
                }
            }
        };

        // Draw centered day number text (1..31)
        let day_str = d.to_string();
        cr.set_font_size(10.5);
        cr.set_source_rgba(text_color.r, text_color.g, text_color.b, text_color.a);
        if let Ok(extents) = cr.text_extents(&day_str) {
            let target_y = rect.y + (rect.height - extents.height()) / 2.0 - extents.y_bearing();
            let tx = rect.x + (rect.width - extents.width()) / 2.0 - extents.x_bearing();
            cr.move_to(tx, target_y);
            let _ = cr.show_text(&day_str);
        }
    }
}
