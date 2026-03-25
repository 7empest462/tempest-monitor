// Unified error type for the monitor
// (Currently using standard Box/Anyhow errors, keeping this for future use)

/*
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TempestError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Config error: {0}")]
    Config(String),
}

pub type Result<T> = std::result::Result<T, TempestError>;
*/
