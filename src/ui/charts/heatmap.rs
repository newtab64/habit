use chrono::Datelike;

use super::geometry::Rect;

/// Gap spacing between heatmap cells (in pixels).
pub const HEATMAP_SPACING: f64 = 4.0;

/// Layout metrics and geometry for the Loop Habit Tracker / GitHub commit style rolling heatmap.
#[derive(Debug, Clone, PartialEq)]
pub struct LoopHeatmapLayout {
    /// Padding around the canvas in pixels.
    pub padding: f64,
    /// Square slot size including gap in pixels.
    pub square_size: f64,
    /// Gap between adjacent squares in pixels.
    pub square_spacing: f64,
    /// Usable inner cell width and height in pixels.
    pub cell_size: f64,
    /// Number of weekly columns that fit on the canvas.
    pub n_cols: usize,
    /// Width reserved for the right-hand weekday labels.
    pub weekday_col_width: f64,
    /// Date of Monday in the earliest (leftmost) column.
    pub top_left_monday: chrono::NaiveDate,
    /// Today's reference date.
    pub today: chrono::NaiveDate,
    /// Week offset backward from today.
    pub week_offset: usize,
}

/// Computes the dynamic layout geometry for a Loop Habit Tracker style continuous heatmap.
#[must_use]
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::suboptimal_flops
)]
pub fn calculate_loop_heatmap_layout(
    width: f64,
    height: f64,
    today: chrono::NaiveDate,
    week_offset: usize,
) -> LoopHeatmapLayout {
    let padding = 8.0;
    let square_spacing = HEATMAP_SPACING;
    let weekday_col_width = 18.0;

    let avail_h = (height - 2.0 * padding).max(24.0);
    let square_size = (avail_h / 8.0).floor().clamp(12.0, 30.0);
    let cell_size = (square_size - square_spacing).max(2.0);

    let avail_w = (width - 2.0 * padding - weekday_col_width).max(square_size);
    let n_cols = ((avail_w / square_size).floor() as usize).max(4);

    let days_from_monday = i64::from(today.weekday().num_days_from_monday());
    let current_week_monday = today - chrono::Duration::days(days_from_monday);
    let rightmost_monday = current_week_monday - chrono::Duration::weeks(week_offset as i64);
    let top_left_monday = rightmost_monday - chrono::Duration::weeks((n_cols - 1) as i64);

    LoopHeatmapLayout {
        padding,
        square_size,
        square_spacing,
        cell_size,
        n_cols,
        weekday_col_width,
        top_left_monday,
        today,
        week_offset,
    }
}

/// Computes the bounding rectangle for a day cell in the Loop heatmap at column `col`
/// (`0..n_cols`) and row `row` (0..=6, where 0=Monday..6=Sunday).
#[must_use]
#[allow(clippy::cast_precision_loss, clippy::suboptimal_flops)]
pub fn loop_heatmap_cell_rect(col: usize, row: usize, layout: &LoopHeatmapLayout) -> Rect {
    let x = layout.padding + (col as f64) * layout.square_size;
    let y = layout.padding + ((row + 1) as f64) * layout.square_size;
    Rect {
        x,
        y,
        width: layout.cell_size,
        height: layout.cell_size,
    }
}

/// Performs hit testing on the Loop Habit Tracker style continuous heatmap.
#[must_use]
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]
pub fn loop_heatmap_hit_test(
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    today: chrono::NaiveDate,
    week_offset: usize,
) -> Option<chrono::NaiveDate> {
    if x < 0.0 || x > width || y < 0.0 || y > height {
        return None;
    }
    let layout = calculate_loop_heatmap_layout(width, height, today, week_offset);
    let top_y = layout.padding + layout.square_size;
    if x < layout.padding || y < top_y {
        return None;
    }

    let col = ((x - layout.padding) / layout.square_size).floor() as usize;
    let row = ((y - top_y) / layout.square_size).floor() as usize;
    if col >= layout.n_cols || row >= 7 {
        return None;
    }

    let rect = loop_heatmap_cell_rect(col, row, &layout);
    if !rect.contains(x, y) {
        return None;
    }

    let col_monday = layout.top_left_monday + chrono::Duration::weeks(col as i64);
    let cell_date = col_monday + chrono::Duration::days(row as i64);
    if cell_date > today {
        return None;
    }

    Some(cell_date)
}

#[cfg(feature = "ui")]
fn draw_month_headers(cr: &cairo::Context, layout: &LoopHeatmapLayout) {
    cr.set_font_size((layout.square_size * 0.55).clamp(9.0, 13.0));
    cr.set_source_rgba(0.65, 0.65, 0.68, 0.85);

    let mut last_month: Option<u32> = None;
    let mut last_year: Option<i32> = None;

    for col in 0..layout.n_cols {
        let week_idx = i64::try_from(col).unwrap_or(0);
        let col_monday = layout.top_left_monday + chrono::Duration::weeks(week_idx);
        let cur_month = col_monday.month();
        let cur_year = col_monday.year();

        let header_text = if last_year.is_some_and(|y| y != cur_year) {
            last_year = Some(cur_year);
            last_month = Some(cur_month);
            format!("{cur_year}")
        } else if last_month.is_none_or(|m| m != cur_month) {
            last_month = Some(cur_month);
            last_year = Some(cur_year);
            col_monday.format("%b").to_string()
        } else {
            String::new()
        };

        if !header_text.is_empty() {
            let col_f64 = u32::try_from(col).map_or(0.0, f64::from);
            let hx = col_f64.mul_add(layout.square_size, layout.padding);
            let hy = layout.square_size.mul_add(0.72, layout.padding);
            cr.move_to(hx, hy);
            let _ = cr.show_text(&header_text);
        }
    }
}

#[cfg(feature = "ui")]
fn draw_weekday_labels(cr: &cairo::Context, layout: &LoopHeatmapLayout) {
    let weekday_labels = ["M", "T", "W", "T", "F", "S", "S"];
    cr.set_font_size((layout.square_size * 0.5).clamp(8.5, 11.5));
    cr.set_source_rgba(0.55, 0.55, 0.58, 0.75);

    let n_cols_f64 = u32::try_from(layout.n_cols).map_or(0.0, f64::from);
    let right_x = n_cols_f64.mul_add(layout.square_size, layout.padding) + 4.0;
    for (row, lbl) in weekday_labels.iter().enumerate() {
        let row_f64 = u32::try_from(row + 1).map_or(0.0, f64::from);
        let ry = layout
            .square_size
            .mul_add(0.72, row_f64.mul_add(layout.square_size, layout.padding));
        cr.move_to(right_x, ry);
        let _ = cr.show_text(lbl);
    }
}

#[cfg(feature = "ui")]
fn draw_day_cell(
    cr: &cairo::Context,
    layout: &LoopHeatmapLayout,
    rect: Rect,
    cell_date: chrono::NaiveDate,
    status: Option<crate::engine::models::EntryStatus>,
    color: &super::geometry::Rgba,
) {
    use super::geometry::{draw_rounded_rect, Rgba};
    use crate::engine::models::EntryStatus;

    let cell_radius = (layout.cell_size * 0.18).clamp(2.5, 5.0);
    let (bg_color, text_color) = if cell_date > layout.today {
        (
            Rgba::new(0.45, 0.45, 0.48, 0.08),
            Rgba::new(0.5, 0.5, 0.52, 0.25),
        )
    } else {
        match status {
            Some(EntryStatus::Completed) => (*color, Rgba::new(1.0, 1.0, 1.0, 1.0)),
            Some(EntryStatus::Skipped) => (
                Rgba::new(color.r, color.g, color.b, 0.28),
                Rgba::new(color.r, color.g, color.b, 0.95),
            ),
            Some(EntryStatus::Missed) => (
                Rgba::new(0.22, 0.22, 0.24, 0.55),
                Rgba::new(0.65, 0.65, 0.68, 0.75),
            ),
            None => (
                Rgba::new(0.22, 0.22, 0.24, 0.35),
                Rgba::new(0.55, 0.55, 0.58, 0.55),
            ),
        }
    };

    cr.set_source_rgba(bg_color.r, bg_color.g, bg_color.b, bg_color.a);
    draw_rounded_rect(cr, rect.x, rect.y, rect.width, rect.height, cell_radius);
    let _ = cr.fill();

    let day_str = cell_date.day().to_string();
    cr.set_font_size((layout.cell_size * 0.48).clamp(8.5, 12.0));
    cr.set_source_rgba(text_color.r, text_color.g, text_color.b, text_color.a);
    if let Ok(extents) = cr.text_extents(&day_str) {
        let target_y = rect.y + (rect.height - extents.height()) / 2.0 - extents.y_bearing();
        let tx = rect.x + (rect.width - extents.width()) / 2.0 - extents.x_bearing();
        cr.move_to(tx, target_y);
        let _ = cr.show_text(&day_str);
    }
}

/// Renders a continuous rolling-weeks heatmap in the Loop Habit Tracker & GitHub style.
///
/// Columns represent weeks scrolling horizontally, while rows represent the 7 days of the week.
/// Row 0 at the top displays month and year labels when they roll over.
/// Right-hand side displays weekday letter labels (`M`, `T`, `W`, `T`, `F`, `S`, `S`).
/// Inside each rounded square cell, the day-of-month number (`1..31`) is centered.
#[cfg(feature = "ui")]
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::suboptimal_flops,
    clippy::similar_names
)]
pub fn draw_loop_heatmap<S: std::hash::BuildHasher>(
    cr: &cairo::Context,
    width: f64,
    height: f64,
    today: chrono::NaiveDate,
    week_offset: usize,
    entries: &std::collections::HashMap<chrono::NaiveDate, crate::engine::models::EntryStatus, S>,
    color: &super::geometry::Rgba,
) {
    if width <= 0.0 || height <= 0.0 {
        return;
    }

    let layout = calculate_loop_heatmap_layout(width, height, today, week_offset);
    draw_month_headers(cr, &layout);
    draw_weekday_labels(cr, &layout);

    for col in 0..layout.n_cols {
        let col_monday = layout.top_left_monday + chrono::Duration::weeks(col as i64);
        for row in 0..7 {
            let cell_date = col_monday + chrono::Duration::days(row as i64);
            let rect = loop_heatmap_cell_rect(col, row, &layout);
            draw_day_cell(
                cr,
                &layout,
                rect,
                cell_date,
                entries.get(&cell_date).copied(),
                color,
            );
        }
    }
}
