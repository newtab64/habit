use super::geometry::Point;

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
    color: &super::geometry::Rgba,
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
