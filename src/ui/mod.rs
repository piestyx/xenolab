pub mod app;
pub mod event;
pub mod terminal;
pub mod view_console;
pub mod view_log;
pub mod view_status;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum UiError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("simulation error: {0}")]
    Sim(#[from] crate::engine::sim::SimError),
}

pub fn run(seed: u64) -> Result<(), UiError> {
    terminal::run_app(app::App::new(seed))
}
