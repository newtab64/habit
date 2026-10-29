use habit::engine::models::{
    Habit, HabitType, DEFAULT_ACCENT_COLOR, SCHEDULE_ALL_DAYS, SCHEDULE_FRI, SCHEDULE_MON,
    SCHEDULE_SAT, SCHEDULE_WED,
};

pub trait HabitTestExt {
    #[must_use]
    fn new_binary(name: impl Into<String>) -> Habit;
    #[must_use]
    fn new_measurable(name: impl Into<String>, target: f64, unit: impl Into<String>) -> Habit;
    #[must_use]
    fn new_gym_example() -> Habit;
    #[must_use]
    fn test_fixture() -> Habit;
    #[must_use]
    fn with_id(self, id: i64) -> Habit;
    #[must_use]
    fn with_schedule(self, scheduled_days: u8) -> Habit;
    #[must_use]
    fn with_color(self, color: impl Into<String>) -> Habit;
}

impl HabitTestExt for Habit {
    fn new_binary(name: impl Into<String>) -> Self {
        Self::new(
            name,
            HabitType::Binary,
            SCHEDULE_ALL_DAYS,
            DEFAULT_ACCENT_COLOR,
        )
    }

    fn new_measurable(name: impl Into<String>, target: f64, unit: impl Into<String>) -> Self {
        Self::new(
            name,
            HabitType::Measurable {
                target,
                unit: unit.into(),
            },
            SCHEDULE_ALL_DAYS,
            DEFAULT_ACCENT_COLOR,
        )
    }

    fn new_gym_example() -> Self {
        Self::new(
            "Gym Workout",
            HabitType::Binary,
            SCHEDULE_MON | SCHEDULE_WED | SCHEDULE_FRI | SCHEDULE_SAT,
            "#E01B24",
        )
    }

    fn test_fixture() -> Self {
        <Self as HabitTestExt>::new_binary("Test Fixture Habit")
    }

    fn with_id(mut self, id: i64) -> Self {
        self.id = Some(id);
        self
    }

    fn with_schedule(mut self, scheduled_days: u8) -> Self {
        self.scheduled_days = scheduled_days;
        self
    }

    fn with_color(mut self, color: impl Into<String>) -> Self {
        self.accent_color = color.into();
        self
    }
}
