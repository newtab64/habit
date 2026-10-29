use std::fmt;

use crate::engine::models::{normalize_hex_color, ColorError, Habit, HabitType};

#[cfg(feature = "ui")]
mod widget;
#[cfg(feature = "ui")]
pub use widget::show_habit_dialog;

/// Error variants encountered when validating a habit form submission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HabitValidationError {
    /// Habit display name was empty or whitespace only.
    EmptyName,
    /// Measurable habit target was less than or equal to zero, or not finite.
    InvalidTarget,
    /// No active scheduled days were selected.
    NoScheduledDays,
    /// Hex color format is invalid or malformed.
    InvalidColor(String),
}

impl fmt::Display for HabitValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyName => write!(f, "habit name cannot be empty"),
            Self::InvalidTarget => write!(f, "measurable target must be greater than zero"),
            Self::NoScheduledDays => write!(f, "at least one scheduled day must be selected"),
            Self::InvalidColor(err) => write!(f, "invalid accent color: {err}"),
        }
    }
}

impl std::error::Error for HabitValidationError {}

impl From<ColorError> for HabitValidationError {
    fn from(err: ColorError) -> Self {
        Self::InvalidColor(err.to_string())
    }
}

/// Pure validation logic for habit creation and edit forms.
///
/// Ensures the name is non-empty, the scheduled days bitmask contains at least one day,
/// any measurable target is strictly greater than zero and finite, and the accent color
/// is normalized to standard 6-digit `#RRGGBB` format.
///
/// # Errors
/// Returns [`HabitValidationError::EmptyName`] if `name` is empty or only whitespace,
/// [`HabitValidationError::InvalidTarget`] if `is_measurable` is true and `target <= 0.0` or non-finite,
/// [`HabitValidationError::NoScheduledDays`] if `scheduled_days == 0`, or
/// [`HabitValidationError::InvalidColor`] if `hex_color` cannot be parsed into a 6-digit hex string.
pub fn validate_habit_form(
    name: &str,
    is_measurable: bool,
    target: f64,
    unit: &str,
    scheduled_days: u8,
    hex_color: &str,
) -> Result<Habit, HabitValidationError> {
    let trimmed_name = name.trim();
    if trimmed_name.is_empty() {
        return Err(HabitValidationError::EmptyName);
    }

    let habit_type = if is_measurable {
        if target <= 0.0 || !target.is_finite() {
            return Err(HabitValidationError::InvalidTarget);
        }
        HabitType::Measurable {
            target,
            unit: unit.trim().to_string(),
        }
    } else {
        HabitType::Binary
    };

    if scheduled_days == 0 {
        return Err(HabitValidationError::NoScheduledDays);
    }

    let normalized_color = normalize_hex_color(hex_color)?;

    Ok(Habit {
        id: None,
        name: trimmed_name.to_string(),
        habit_type,
        accent_color: normalized_color,
        scheduled_days,
        archived: false,
        created_at: 0,
    })
}
