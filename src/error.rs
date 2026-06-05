use thiserror::Error;

#[derive(Error, Debug)]
pub enum TempestError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Config parsing error: {0}")]
    Config(String),

    #[cfg(feature = "database")]
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Platform not supported: {0}")]
    UnsupportedPlatform(String),

    #[error("Process not found (PID: {0})")]
    ProcessNotFound(sysinfo::Pid),

    #[error("Telemetry error: {0}")]
    Telemetry(String),
}

pub type Result<T> = std::result::Result<T, TempestError>;
