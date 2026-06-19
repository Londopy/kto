//! Crate-wide error type for the binary's top-level `Result`.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum KtoError {
    #[error("configuration error: {0}")]
    Config(String),

    #[error("engine error: {0}")]
    Engine(#[from] crate::engine::EngineError),

    #[error("session/export error: {0}")]
    Session(String),

    #[error("update check failed: {0}")]
    Update(String),

    #[error("UI error: {0}")]
    Ui(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, KtoError>;
