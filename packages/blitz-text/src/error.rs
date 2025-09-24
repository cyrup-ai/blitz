//! Error handling for text shaping operations

/// Comprehensive error handling for text shaping
#[derive(Debug, thiserror::Error)]
pub enum ShapingError {
    #[error("Font system access failed")]
    FontSystemError,

    #[error("Bidirectional text processing failed: {0}")]
    BidiProcessingError(String),

    #[error("Script analysis failed: {0}")]
    ScriptAnalysisError(String),

    #[error("Text shaping failed: {0}")]
    ShapingFailed(String),

    #[error("Memory allocation failed")]
    MemoryError,

    #[error("Invalid text input: {0}")]
    InvalidInput(String),

    #[error("Cache operation failed: {0}")]
    CacheError(String),

    #[error("Cache initialization failed: {0}")]
    CacheInitializationError(String),

    #[error("Cache operation error: {0}")]
    CacheOperationError(String),

    #[error("Invalid cache size: {0}")]
    InvalidCacheSize(usize),

    #[error("Unicode processing error: {0}")]
    UnicodeError(String),

    #[error("Font not found for provided attributes")]
    FontNotFound,

    #[error("Failed to load font from system")]
    FontLoadError,

    #[error("Font database transfer failed")]
    FontTransferError,

    #[error("Font loading failed: {0}")]
    FontLoadErrorWithMessage(String),

    #[error("Feature configuration error: {0}")]
    FeatureError(String),

    #[error("IO operation failed: {0}")]
    Io(String),

    #[error("Lock acquisition failed")]
    LockError,

    #[error("Data corruption detected: {0}")]
    Corruption(String),

    #[error("Serialization failed: {0}")]
    Serialization(String),

    #[error("Invalid range: start {start}, end {end}, length {length}")]
    InvalidRange {
        start: usize,
        end: usize,
        length: usize,
    },
}

impl From<std::fmt::Error> for ShapingError {
    fn from(err: std::fmt::Error) -> Self {
        ShapingError::ShapingFailed(err.to_string())
    }
}

impl From<std::collections::TryReserveError> for ShapingError {
    fn from(_: std::collections::TryReserveError) -> Self {
        ShapingError::MemoryError
    }
}

impl From<Box<dyn std::error::Error + Send + Sync>> for ShapingError {
    fn from(err: Box<dyn std::error::Error + Send + Sync>) -> Self {
        ShapingError::ShapingFailed(err.to_string())
    }
}

impl From<Box<dyn std::error::Error>> for ShapingError {
    fn from(err: Box<dyn std::error::Error>) -> Self {
        ShapingError::CacheOperationError(err.to_string())
    }
}
