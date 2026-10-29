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

pub use crate::engine::color::{Rgba, DEFAULT_ACCENT};

/// Helper function to create a closed rounded rectangle sub-path in a Cairo context.
#[cfg(feature = "ui")]
pub fn draw_rounded_rect(
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
