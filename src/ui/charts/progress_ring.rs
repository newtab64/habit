#[cfg(feature = "ui")]
use super::geometry::Rgba;

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
    if width <= 0.0 || height <= 0.0 || !width.is_finite() || !height.is_finite() {
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
    let clamped = if percentage.is_finite() {
        percentage.clamp(0.0, 1.0)
    } else {
        0.0
    };
    if clamped > 0.001 {
        let start_angle = -std::f64::consts::FRAC_PI_2;
        let end_angle = (clamped * 2.0).mul_add(std::f64::consts::PI, start_angle);
        cr.set_source_rgba(color.r, color.g, color.b, 1.0);
        cr.arc(center_x, center_y, radius, start_angle, end_angle);
        let _ = cr.stroke();
    }
}
