use chrono::Datelike;

#[cfg(feature = "ui")]
use crate::engine::models::EntryStatus;

/// Number of columns in the monthly calendar heatmap (Monday through Sunday).
pub const HEATMAP_COLS: usize = 7;
/// Maximum number of rows required to display any month in a 7-column layout.
pub const HEATMAP_ROWS: usize = 6;
/// Outer padding around the heatmap canvas (in pixels).
pub const HEATMAP_PADDING: f64 = 8.0;
/// Gap spacing between heatmap cells (in pixels).
pub const HEATMAP_SPACING: f64 = 5.0;
/// Top header height reserved for weekday label letters (M, T, W, T, F, S, S).
pub const HEATMAP_HEADER_HEIGHT: f64 = 20.0;

/// A 2D floating-point point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    /// X coordinate in pixels.
    pub x: f64,
    /// Y coordinate in pixels.
    pub y: f64,
}

impl Point {
    /// Constructs a new `Point`.
    #[must_use]
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// A 2D floating-point rectangle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    /// X coordinate of the top-left corner.
    pub x: f64,
    /// Y coordinate of the top-left corner.
    pub y: f64,
    /// Width of the rectangle.
    pub width: f64,
    /// Height of the rectangle.
    pub height: f64,
}

impl Rect {
    /// Constructs a new `Rect`.
    #[must_use]
    pub const fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Checks if a 2D point `(px, py)` falls inside this rectangle (inclusive of edges).
    #[must_use]
    pub fn contains(&self, px: f64, py: f64) -> bool {
        px >= self.x && px <= self.x + self.width && py >= self.y && py <= self.y + self.height
    }
}

/// Normalized RGBA color with channel values in the range `0.0..=1.0`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgba {
    /// Red channel in `[0.0, 1.0]`.
    pub r: f64,
    /// Green channel in `[0.0, 1.0]`.
    pub g: f64,
    /// Blue channel in `[0.0, 1.0]`.
    pub b: f64,
    /// Alpha channel in `[0.0, 1.0]`.
    pub a: f64,
}

impl Rgba {
    /// Constructs a new `Rgba` value.
    #[must_use]
    pub const fn new(r: f64, g: f64, b: f64, a: f64) -> Self {
        Self { r, g, b, a }
    }

    /// Parses an RGBA color from a hex string (`#RRGGBB` or `#RRGGBBAA`),
    /// with or without leading `#`.
    #[must_use]
    pub fn from_hex(hex: &str) -> Option<Self> {
        let trimmed = hex.trim();
        let body = trimmed.strip_prefix('#').unwrap_or(trimmed);
        if !body.is_ascii() {
            return None;
        }
        match body.len() {
            6 => {
                let r = u8::from_str_radix(&body[0..2], 16).ok()?;
                let g = u8::from_str_radix(&body[2..4], 16).ok()?;
                let b = u8::from_str_radix(&body[4..6], 16).ok()?;
                Some(Self {
                    r: f64::from(r) / 255.0,
                    g: f64::from(g) / 255.0,
                    b: f64::from(b) / 255.0,
                    a: 1.0,
                })
            }
            8 => {
                let r = u8::from_str_radix(&body[0..2], 16).ok()?;
                let g = u8::from_str_radix(&body[2..4], 16).ok()?;
                let b = u8::from_str_radix(&body[4..6], 16).ok()?;
                let a = u8::from_str_radix(&body[6..8], 16).ok()?;
                Some(Self {
                    r: f64::from(r) / 255.0,
                    g: f64::from(g) / 255.0,
                    b: f64::from(b) / 255.0,
                    a: f64::from(a) / 255.0,
                })
            }
            _ => None,
        }
    }
}

#[cfg(feature = "ui")]
impl From<gtk4::gdk::RGBA> for Rgba {
    fn from(rgba: gtk4::gdk::RGBA) -> Self {
        Self {
            r: f64::from(rgba.red()),
            g: f64::from(rgba.green()),
            b: f64::from(rgba.blue()),
            a: f64::from(rgba.alpha()),
        }
    }
}

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

/// Computes drawing points for a habit strength curve.
///
/// Maps normalized series data `[0.0, 1.0]` into canvas coordinates `(x, y)`
/// within `[padding, width - padding]` and `[padding, height - padding]`.
/// Y values are inverted for screen coordinates (1.0 = top, 0.0 = bottom).
#[must_use]
#[allow(clippy::cast_precision_loss, clippy::suboptimal_flops)]
pub fn calculate_curve_points(data: &[f64], width: f64, height: f64, padding: f64) -> Vec<Point> {
    if data.is_empty() {
        return Vec::new();
    }

    let usable_w = (width - 2.0 * padding).max(0.0);
    let usable_h = (height - 2.0 * padding).max(0.0);

    if data.len() == 1 {
        let val = data[0].clamp(0.0, 1.0);
        let x = padding + usable_w / 2.0;
        let y = padding + (1.0 - val) * usable_h;
        return vec![Point { x, y }];
    }

    let n = data.len();
    let step_x = if usable_w > 0.0 {
        usable_w / (n - 1) as f64
    } else {
        0.0
    };

    data.iter()
        .enumerate()
        .map(|(i, &val)| {
            let clamped = if val.is_nan() {
                0.0
            } else {
                val.clamp(0.0, 1.0)
            };
            let x = padding + (i as f64) * step_x;
            let y = padding + (1.0 - clamped) * usable_h;
            Point { x, y }
        })
        .collect()
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
    let square_spacing = 2.0;
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

/// Renders a circular progress ring (e.g. for the Overview card).
///
/// Draws a subtle background track ring and a rounded foreground progress arc
/// starting from the 12 o'clock position (-PI/2) clockwise.
#[cfg(feature = "ui")]
pub fn draw_progress_ring(
    cr: &cairo::Context,
    width: f64,
    height: f64,
    percentage: f64,
    color: &Rgba,
) {
    if width <= 0.0 || height <= 0.0 {
        return;
    }

    let size = width.min(height);
    let center_x = width / 2.0;
    let center_y = height / 2.0;
    let line_width = (size * 0.16).clamp(3.0, 6.0);
    let radius = (size - line_width) / 2.0;
    if radius <= 0.0 {
        return;
    }

    // 1. Background inactive track ring
    cr.set_line_width(line_width);
    cr.set_line_cap(cairo::LineCap::Round);
    cr.set_source_rgba(color.r, color.g, color.b, 0.20);
    cr.arc(center_x, center_y, radius, 0.0, 2.0 * std::f64::consts::PI);
    let _ = cr.stroke();

    // 2. Foreground active progress arc
    let clamped = percentage.clamp(0.0, 1.0);
    if clamped > 0.001 {
        let start_angle = -std::f64::consts::FRAC_PI_2;
        let end_angle = start_angle + clamped * 2.0 * std::f64::consts::PI;
        cr.set_source_rgba(color.r, color.g, color.b, 1.0);
        cr.arc(center_x, center_y, radius, start_angle, end_angle);
        let _ = cr.stroke();
    }
}

/// Renders a smooth habit strength curve with gradient fill using Cairo.
///
/// Draws a smooth cubic bezier line with a 2.5px stroke in the specified color,
/// and fills the area beneath the curve with a gradient decaying from 0.35 alpha
/// at the top to 0.0 alpha at the bottom.
#[cfg(feature = "ui")]
#[allow(clippy::similar_names)]
pub fn draw_strength_curve(
    cr: &cairo::Context,
    width: f64,
    height: f64,
    data: &[f64],
    color: &Rgba,
) {
    if data.is_empty() || width <= 0.0 || height <= 0.0 {
        return;
    }

    let padding = 12.0;
    let pts = calculate_curve_points(data, width, height, padding);
    if pts.is_empty() {
        return;
    }

    let bottom_y = (height - padding).max(padding);

    // 1. Draw gradient fill beneath the curve
    cr.new_path();
    cr.move_to(pts[0].x, pts[0].y);

    if pts.len() == 1 {
        cr.line_to(pts[0].x + 1.0, pts[0].y);
        cr.line_to(pts[0].x + 1.0, bottom_y);
    } else {
        for window in pts.windows(2) {
            let p0 = window[0];
            let p1 = window[1];
            let cp1_x = f64::midpoint(p0.x, p1.x);
            let cp1_y = p0.y;
            let cp2_x = f64::midpoint(p0.x, p1.x);
            let cp2_y = p1.y;
            cr.curve_to(cp1_x, cp1_y, cp2_x, cp2_y, p1.x, p1.y);
        }
        if let Some(last) = pts.last() {
            cr.line_to(last.x, bottom_y);
        }
    }
    cr.line_to(pts[0].x, bottom_y);
    cr.close_path();

    let gradient = cairo::LinearGradient::new(0.0, padding, 0.0, bottom_y);
    gradient.add_color_stop_rgba(0.0, color.r, color.g, color.b, color.a * 0.35);
    gradient.add_color_stop_rgba(1.0, color.r, color.g, color.b, 0.0);
    let _ = cr.set_source(&gradient);
    let _ = cr.fill();

    // 2. Draw smooth stroke curve (2.5px width)
    cr.new_path();
    cr.move_to(pts[0].x, pts[0].y);
    if pts.len() == 1 {
        cr.line_to(pts[0].x + 1.0, pts[0].y);
    } else {
        for window in pts.windows(2) {
            let p0 = window[0];
            let p1 = window[1];
            let cp1_x = f64::midpoint(p0.x, p1.x);
            let cp1_y = p0.y;
            let cp2_x = f64::midpoint(p0.x, p1.x);
            let cp2_y = p1.y;
            cr.curve_to(cp1_x, cp1_y, cp2_x, cp2_y, p1.x, p1.y);
        }
    }
    cr.set_source_rgba(color.r, color.g, color.b, color.a);
    cr.set_line_width(2.5);
    let _ = cr.stroke();
}

#[cfg(feature = "ui")]
fn draw_rounded_rect(
    cr: &cairo::Context,
    pos_x: f64,
    pos_y: f64,
    width: f64,
    height: f64,
    corner_radius: f64,
) {
    let radius = corner_radius.min(width / 2.0).min(height / 2.0);
    cr.new_sub_path();
    cr.arc(
        pos_x + width - radius,
        pos_y + radius,
        radius,
        -std::f64::consts::FRAC_PI_2,
        0.0,
    );
    cr.arc(
        pos_x + width - radius,
        pos_y + height - radius,
        radius,
        0.0,
        std::f64::consts::FRAC_PI_2,
    );
    cr.arc(
        pos_x + radius,
        pos_y + height - radius,
        radius,
        std::f64::consts::FRAC_PI_2,
        std::f64::consts::PI,
    );
    cr.arc(
        pos_x + radius,
        pos_y + radius,
        radius,
        std::f64::consts::PI,
        3.0 * std::f64::consts::FRAC_PI_2,
    );
    cr.close_path();
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
    clippy::too_many_lines,
    clippy::similar_names
)]
pub fn draw_loop_heatmap<S: std::hash::BuildHasher>(
    cr: &cairo::Context,
    width: f64,
    height: f64,
    today: chrono::NaiveDate,
    week_offset: usize,
    entries: &std::collections::HashMap<chrono::NaiveDate, EntryStatus, S>,
    color: &Rgba,
) {
    if width <= 0.0 || height <= 0.0 {
        return;
    }

    let layout = calculate_loop_heatmap_layout(width, height, today, week_offset);
    let cell_radius = (layout.cell_size * 0.18).clamp(2.5, 5.0);

    // 1. Draw Month / Year column headers along row 0
    cr.set_font_size((layout.square_size * 0.55).clamp(9.0, 13.0));
    cr.set_source_rgba(0.65, 0.65, 0.68, 0.85);

    let mut last_month: Option<u32> = None;
    let mut last_year: Option<i32> = None;

    for col in 0..layout.n_cols {
        let col_monday = layout.top_left_monday + chrono::Duration::weeks(col as i64);
        let cur_month = col_monday.month();
        let cur_year = col_monday.year();

        let header_text = if last_year.is_some_and(|y| y != cur_year) {
            last_year = Some(cur_year);
            last_month = Some(cur_month);
            format!("{cur_year}")
        } else if last_month.is_none_or(|m| m != cur_month) {
            last_month = Some(cur_month);
            last_year = Some(cur_year);
            match cur_month {
                1 => "Jan",
                2 => "Feb",
                3 => "Mar",
                4 => "Apr",
                5 => "May",
                6 => "Jun",
                7 => "Jul",
                8 => "Aug",
                9 => "Sep",
                10 => "Oct",
                11 => "Nov",
                12 => "Dec",
                _ => "",
            }
            .to_string()
        } else {
            String::new()
        };

        if !header_text.is_empty() {
            let hx = layout.padding + (col as f64) * layout.square_size;
            let hy = layout.padding + layout.square_size * 0.72;
            cr.move_to(hx, hy);
            let _ = cr.show_text(&header_text);
        }
    }

    // 2. Draw Weekday labels on the right side
    let weekday_labels = ["M", "T", "W", "T", "F", "S", "S"];
    cr.set_font_size((layout.square_size * 0.5).clamp(8.5, 11.5));
    cr.set_source_rgba(0.55, 0.55, 0.58, 0.75);

    let right_x = layout.padding + (layout.n_cols as f64) * layout.square_size + 4.0;
    for (row, lbl) in weekday_labels.iter().enumerate() {
        let ry =
            layout.padding + ((row + 1) as f64) * layout.square_size + layout.square_size * 0.72;
        cr.move_to(right_x, ry);
        let _ = cr.show_text(lbl);
    }

    // 3. Draw day cells
    for col in 0..layout.n_cols {
        let col_monday = layout.top_left_monday + chrono::Duration::weeks(col as i64);
        for row in 0..7 {
            let cell_date = col_monday + chrono::Duration::days(row as i64);
            let rect = loop_heatmap_cell_rect(col, row, &layout);

            let text_color = if cell_date > today {
                // Future day
                cr.set_source_rgba(0.45, 0.45, 0.48, 0.08);
                draw_rounded_rect(cr, rect.x, rect.y, rect.width, rect.height, cell_radius);
                let _ = cr.fill();
                Rgba::new(0.5, 0.5, 0.52, 0.25)
            } else {
                match entries.get(&cell_date) {
                    Some(EntryStatus::Completed) => {
                        cr.set_source_rgba(color.r, color.g, color.b, color.a);
                        draw_rounded_rect(cr, rect.x, rect.y, rect.width, rect.height, cell_radius);
                        let _ = cr.fill();
                        Rgba::new(1.0, 1.0, 1.0, 1.0)
                    }
                    Some(EntryStatus::Skipped) => {
                        // Rest day: gentle tint of habit color without underline
                        cr.set_source_rgba(color.r, color.g, color.b, 0.28);
                        draw_rounded_rect(cr, rect.x, rect.y, rect.width, rect.height, cell_radius);
                        let _ = cr.fill();
                        Rgba::new(color.r, color.g, color.b, 0.95)
                    }
                    Some(EntryStatus::Missed) => {
                        cr.set_source_rgba(0.22, 0.22, 0.24, 0.55);
                        draw_rounded_rect(cr, rect.x, rect.y, rect.width, rect.height, cell_radius);
                        let _ = cr.fill();
                        Rgba::new(0.65, 0.65, 0.68, 0.75)
                    }
                    None => {
                        cr.set_source_rgba(0.22, 0.22, 0.24, 0.35);
                        draw_rounded_rect(cr, rect.x, rect.y, rect.width, rect.height, cell_radius);
                        let _ = cr.fill();
                        Rgba::new(0.55, 0.55, 0.58, 0.55)
                    }
                }
            };

            // Draw centered day number text (1..31)
            let day_str = cell_date.day().to_string();
            cr.set_font_size((layout.cell_size * 0.48).clamp(8.5, 12.0));
            cr.set_source_rgba(text_color.r, text_color.g, text_color.b, text_color.a);
            if let Ok(extents) = cr.text_extents(&day_str) {
                let target_y =
                    rect.y + (rect.height - extents.height()) / 2.0 - extents.y_bearing();
                let tx = rect.x + (rect.width - extents.width()) / 2.0 - extents.x_bearing();
                cr.move_to(tx, target_y);
                let _ = cr.show_text(&day_str);
            }
        }
    }
}
