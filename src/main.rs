#![forbid(unsafe_code)]
#![warn(clippy::pedantic, clippy::nursery)]

fn main() {
    #[cfg(feature = "ui")]
    {
        habit::ui::window::run_app();
    }

    #[cfg(not(feature = "ui"))]
    {
        println!("Starting Habit ({}) [Headless / CLI Mode]", habit::APP_ID);
    }
}
