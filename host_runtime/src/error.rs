use thiserror::Error;

#[allow(dead_code)]
#[derive(Debug, Error)]
pub enum MOError {
    #[error("memory export not found")]
    MemoryNotFound,

    #[error("invalid address or overflow")]
    InvalidAddress,

    #[error("size exceeds limit: {size} > {max}")]
    SizeExceedsLimit { size: usize, max: usize },

    #[error("out of bounds: {start}..{end}")]
    OutOfBounds { start: usize, end: usize },

    #[error("invalid UTF-8: {0}")]
    InvalidUtf8(#[from] std::str::Utf8Error),

    // --- для будущих модулей ---
    #[error("file not found: {0}")]
    FileNotFound(String),

    #[error("I/O error: {0}")]
    Io(String),

    #[error("host call failed: {0}")]
    HostCall(String),

    #[error("image decode failed")]
    ImageDecodeFailed,

    #[error("image encode failed")]
    ImageEncodeFailed,
}

pub type MOResult<T> = Result<T, MOError>;