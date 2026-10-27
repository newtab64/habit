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
