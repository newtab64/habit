/// Default GNOME blue accent color (`#3584E4`).
pub const DEFAULT_ACCENT: Rgba = Rgba::new(0.21, 0.52, 0.89, 1.0);

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
