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
pub const HEATMAP_SPACING: f64 = 4.0;

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
            let clamped = if val.is_nan() { 0.0 } else { val.clamp(0.0, 1.0) };
            let x = padding + (i as f64) * step_x;
            let y = padding + (1.0 - clamped) * usable_h;
            Point { x, y }
        })
        .collect()
}

/// Computes the bounding rectangle for a calendar cell at column `col` (0..=6)
/// and row `row` (0..=5) on a canvas of dimension `(width, height)`.
#[must_use]
#[allow(clippy::cast_precision_loss, clippy::suboptimal_flops)]
pub fn heatmap_cell_rect(col: usize, row: usize, width: f64, height: f64) -> Rect {
    let total_spacing_x = HEATMAP_SPACING * 6.0;
    let total_spacing_y = HEATMAP_SPACING * 5.0;
    let avail_w = (width - 2.0 * HEATMAP_PADDING - total_spacing_x).max(0.0);
    let avail_h = (height - 2.0 * HEATMAP_PADDING - total_spacing_y).max(0.0);
    let cell_w = avail_w / 7.0;
    let cell_h = avail_h / 6.0;

    let x = HEATMAP_PADDING + (col as f64) * (cell_w + HEATMAP_SPACING);
    let y = HEATMAP_PADDING + (row as f64) * (cell_h + HEATMAP_SPACING);

    Rect {
        x,
        y,
        width: cell_w,
        height: cell_h,
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

/// Renders a monthly calendar heatmap using Cairo.
///
/// Draws a 7-column calendar matrix (Monday through Sunday) for the given month:
/// - Completed days: filled square in accent color.
/// - Pre-defined rest / skipped days: subtle horizontal dash and muted fill.
/// - Missed days: dark/translucent border.
/// - Unreached future days: faint gray outline.
#[cfg(feature = "ui")]
#[allow(clippy::cast_possible_truncation)]
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

    for d in 1..=days {
        let Some(date) = chrono::NaiveDate::from_ymd_opt(year, month, d) else {
            continue;
        };
        let offset = start_col + (d as usize - 1);
        let col = offset % HEATMAP_COLS;
        let row = offset / HEATMAP_COLS;
        let rect = heatmap_cell_rect(col, row, width, height);

        match entries.get(&date) {
            Some(EntryStatus::Completed) => {
                // Completed days: filled square in accent color
                cr.set_source_rgba(color.r, color.g, color.b, color.a);
                cr.rectangle(rect.x, rect.y, rect.width, rect.height);
                let _ = cr.fill();
            }
            Some(EntryStatus::Skipped) => {
                // Pre-defined rest / skipped days: subtle horizontal dash or muted square
                cr.set_source_rgba(color.r, color.g, color.b, 0.25);
                cr.rectangle(rect.x, rect.y, rect.width, rect.height);
                let _ = cr.fill();

                let dash_w = rect.width * 0.45;
                let dash_x = rect.x + (rect.width - dash_w) / 2.0;
                let dash_y = rect.y + rect.height / 2.0;
                cr.set_source_rgba(color.r, color.g, color.b, 0.85);
                cr.set_line_width(2.0);
                cr.move_to(dash_x, dash_y);
                cr.line_to(dash_x + dash_w, dash_y);
                let _ = cr.stroke();
            }
            Some(EntryStatus::Missed) => {
                // Missed days: dark/translucent border
                cr.set_source_rgba(0.85, 0.2, 0.2, 0.6);
                cr.set_line_width(1.5);
                cr.rectangle(rect.x, rect.y, rect.width, rect.height);
                let _ = cr.stroke();
            }
            None => {
                if date > today {
                    // Unreached future days: faint gray outline
                    cr.set_source_rgba(0.6, 0.6, 0.6, 0.25);
                } else {
                    // Past day with no entry logged: muted outline
                    cr.set_source_rgba(0.5, 0.5, 0.5, 0.45);
                }
                cr.set_line_width(1.0);
                cr.rectangle(rect.x, rect.y, rect.width, rect.height);
                let _ = cr.stroke();
            }
        }
    }
}
