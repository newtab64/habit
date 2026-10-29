#![forbid(unsafe_code)]
#![warn(clippy::pedantic, clippy::nursery)]

#[cfg(not(feature = "ui"))]
compile_error!("build with --features ui");

fn main() {
    #[cfg(feature = "ui")]
    habit::ui::window::run_app();
}
