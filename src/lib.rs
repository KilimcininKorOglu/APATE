pub mod auth;
pub mod config;
pub mod noise;
pub mod telemetry;
pub mod transport;
pub mod util;

use thiserror::Error;

pub const APATE_NAME: &str = "apate";

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ConfigError {
    #[error("missing required configuration key: {key}")]
    MissingRequiredKey { key: String },
    #[error("invalid configuration value for key: {key}")]
    InvalidValue { key: String },
    #[error("unsupported configuration key: {key}")]
    UnsupportedKey { key: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RuntimeError {
    #[error("runtime backend unavailable: {backend}")]
    BackendUnavailable { backend: String },
    #[error("event loop start failed")]
    EventLoopStartFailed,
    #[error("runtime shutdown timed out")]
    ShutdownTimeout,
}

#[derive(Debug, Error)]
pub enum ApateError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    Auth(#[from] auth::AuthError),
    #[error(transparent)]
    Frame(#[from] transport::FrameError),
    #[error(transparent)]
    Transport(#[from] transport::TransportError),
    #[error(transparent)]
    Security(#[from] noise::SecurityError),
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
}
