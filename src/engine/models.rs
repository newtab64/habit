use std::fmt;
use std::str::FromStr;

/// Schedule bitmask constant for Monday.
pub const SCHEDULE_MON: u8 = 1;
/// Schedule bitmask constant for Tuesday.
pub const SCHEDULE_TUE: u8 = 2;
/// Schedule bitmask constant for Wednesday.
pub const SCHEDULE_WED: u8 = 4;
/// Schedule bitmask constant for Thursday.
pub const SCHEDULE_THU: u8 = 8;
/// Schedule bitmask constant for Friday.
pub const SCHEDULE_FRI: u8 = 16;
/// Schedule bitmask constant for Saturday.
pub const SCHEDULE_SAT: u8 = 32;
/// Schedule bitmask constant for Sunday.
pub const SCHEDULE_SUN: u8 = 64;

/// Bitmask for all 7 days of the week.
pub const SCHEDULE_ALL_DAYS: u8 = SCHEDULE_MON
    | SCHEDULE_TUE
    | SCHEDULE_WED
    | SCHEDULE_THU
    | SCHEDULE_FRI
    | SCHEDULE_SAT
    | SCHEDULE_SUN;

/// Bitmask for weekdays (Monday through Friday).
pub const SCHEDULE_WORKDAYS: u8 =
    SCHEDULE_MON | SCHEDULE_TUE | SCHEDULE_WED | SCHEDULE_THU | SCHEDULE_FRI;

/// Bitmask for weekend days (Saturday and Sunday).
pub const SCHEDULE_WEEKENDS: u8 = SCHEDULE_SAT | SCHEDULE_SUN;

/// Default GNOME accent color (Blue).
pub const DEFAULT_ACCENT_COLOR: &str = "#3584E4";

/// Standard GNOME palette preset hex colors.
pub const GNOME_PALETTE: [&str; 8] = [
    "#3584E4", // Blue
    "#33D17A", // Green
    "#F6D32D", // Yellow
    "#FF7800", // Orange
    "#E01B24", // Red
    "#9141AC", // Purple
    "#986A44", // Brown
    "#5E5C64", // Dark
];

/// Error when validating or normalizing a hex color string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ColorError {
    /// String is empty or whitespace only.
    Empty,
    /// Invalid length (must be 6 hex digits, optional leading `#`).
    InvalidFormat,
    /// Encountered a non-hexadecimal character.
    InvalidHexDigit(char),
}

impl fmt::Display for ColorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "color string cannot be empty"),
            Self::InvalidFormat => {
                write!(f, "hex color must be 6 hex digits (e.g. #RRGGBB or RRGGBB)")
            }
            Self::InvalidHexDigit(c) => write!(f, "invalid hex character: '{c}'"),
        }
    }
}

impl std::error::Error for ColorError {}

/// Normalizes a hex color string into standard `#RRGGBB` format (uppercase).
///
/// Accepts strings like `"#ff7800"`, `"ff7800"`, `"#FF7800"`.
///
/// # Errors
/// Returns [`ColorError::Empty`] if input is empty, [`ColorError::InvalidFormat`]
/// if length is not 6 hex characters (ignoring leading `#`), or
/// [`ColorError::InvalidHexDigit`] if a character is not an ASCII hex digit.
pub fn normalize_hex_color(input: &str) -> Result<String, ColorError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(ColorError::Empty);
    }
    let hex_part = trimmed.strip_prefix('#').unwrap_or(trimmed);
    if hex_part.len() != 6 {
        return Err(ColorError::InvalidFormat);
    }
    let mut normalized = String::with_capacity(7);
    normalized.push('#');
    for c in hex_part.chars() {
        if c.is_ascii_hexdigit() {
            normalized.push(c.to_ascii_uppercase());
        } else {
            return Err(ColorError::InvalidHexDigit(c));
        }
    }
    Ok(normalized)
}

/// The status of a habit entry on a specific date.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntryStatus {
    /// The habit was completed or met its measurable target.
    Completed,
    /// The habit was skipped (e.g. pre-defined rest day, excused).
    Skipped,
    /// The habit was scheduled but not completed.
    Missed,
}

impl EntryStatus {
    /// Returns the database and serialized string representation.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Skipped => "skipped",
            Self::Missed => "missed",
        }
    }

    /// Parses a status from its string representation.
    #[must_use]
    pub fn parse_status(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "completed" => Some(Self::Completed),
            "skipped" => Some(Self::Skipped),
            "missed" => Some(Self::Missed),
            _ => None,
        }
    }
}

impl fmt::Display for EntryStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for EntryStatus {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse_status(s).ok_or(())
    }
}

/// Defines the completion mode and target for a habit.
#[derive(Debug, Clone, PartialEq)]
pub enum HabitType {
    /// Binary habit (done or not done).
    Binary,
    /// Measurable habit with a numerical daily target and unit label.
    Measurable {
        /// Target numerical value to achieve completion.
        target: f64,
        /// Unit description (e.g. "pages", "minutes", "ml").
        unit: String,
    },
}

impl HabitType {
    /// Returns true if this is a measurable habit.
    #[must_use]
    pub const fn is_measurable(&self) -> bool {
        matches!(self, Self::Measurable { .. })
    }

    /// Returns the measurable target if applicable.
    #[must_use]
    pub const fn target(&self) -> Option<f64> {
        match self {
            Self::Binary => None,
            Self::Measurable { target, .. } => Some(*target),
        }
    }

    /// Returns the measurable unit if applicable.
    #[must_use]
    pub const fn unit(&self) -> Option<&str> {
        match self {
            Self::Binary => None,
            Self::Measurable { unit, .. } => Some(unit.as_str()),
        }
    }
}

/// Represents a single logged entry for a habit on a calendar date.
#[derive(Debug, Clone, PartialEq)]
pub struct HabitEntry {
    /// Foreign key referencing the parent habit.
    pub habit_id: i64,
    /// Calendar date of this entry.
    pub date: chrono::NaiveDate,
    /// Status on that date.
    pub status: EntryStatus,
    /// Numerical value recorded (1.0 for completed binary, or arbitrary float).
    pub value: f64,
}

impl HabitEntry {
    /// Creates a new habit entry.
    #[must_use]
    pub const fn new(
        habit_id: i64,
        date: chrono::NaiveDate,
        status: EntryStatus,
        value: f64,
    ) -> Self {
        Self {
            habit_id,
            date,
            status,
            value,
        }
    }
}

/// A habit tracking definition and schedule configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct Habit {
    /// Primary key in SQLite (`None` before insertion).
    pub id: Option<i64>,
    /// Display name of the habit.
    pub name: String,
    /// Completion type (binary or measurable).
    pub habit_type: HabitType,
    /// Accent color in standard hex `#RRGGBB` format.
    pub accent_color: String,
    /// Weekday schedule bitmask ($1 = \text{Mon} \dots 64 = \text{Sun}$).
    pub scheduled_days: u8,
    /// Whether this habit is archived.
    pub archived: bool,
    /// Creation timestamp (Unix epoch in seconds).
    pub created_at: i64,
}

impl Habit {
    /// Creates a new habit with custom parameters.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        habit_type: HabitType,
        scheduled_days: u8,
        accent_color: impl Into<String>,
    ) -> Self {
        Self {
            id: None,
            name: name.into(),
            habit_type,
            accent_color: accent_color.into(),
            scheduled_days,
            archived: false,
            created_at: 0,
        }
    }

    /// Returns the bitmask bit corresponding to a weekday ($1 = \text{Mon} \dots 64 = \text{Sun}$).
    #[must_use]
    pub const fn weekday_bit(weekday: chrono::Weekday) -> u8 {
        match weekday {
            chrono::Weekday::Mon => SCHEDULE_MON,
            chrono::Weekday::Tue => SCHEDULE_TUE,
            chrono::Weekday::Wed => SCHEDULE_WED,
            chrono::Weekday::Thu => SCHEDULE_THU,
            chrono::Weekday::Fri => SCHEDULE_FRI,
            chrono::Weekday::Sat => SCHEDULE_SAT,
            chrono::Weekday::Sun => SCHEDULE_SUN,
        }
    }

    /// Checks if this habit is scheduled to occur on the specified weekday.
    #[must_use]
    pub const fn is_scheduled_on(&self, weekday: chrono::Weekday) -> bool {
        (self.scheduled_days & Self::weekday_bit(weekday)) != 0
    }

    /// Parses the habit's hex accent color into an [`Rgba`](crate::engine::color::Rgba),
    /// falling back to [`DEFAULT_ACCENT`](crate::engine::color::DEFAULT_ACCENT).
    #[must_use]
    pub fn accent_rgba(&self) -> crate::engine::color::Rgba {
        crate::engine::color::Rgba::from_hex(&self.accent_color)
            .unwrap_or(crate::engine::color::DEFAULT_ACCENT)
    }
}
