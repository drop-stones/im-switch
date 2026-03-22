/// Error type for input method operations.
#[derive(Debug, thiserror::Error)]
pub enum ImSwitchError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("unsupported platform")]
    UnsupportedPlatform,
    #[error("input method not found: {0}")]
    InputMethodNotFound(String),
    #[error("{0}")]
    Platform(String),
}
