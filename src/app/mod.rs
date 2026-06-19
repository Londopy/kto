//! Application core: shared state, configuration, and persistent achievements.

pub mod achievements;
pub mod config;
pub mod state;

pub use config::Config;
pub use state::{AppState, Shared};
