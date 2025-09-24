use thiserror::Error;

/// Font management errors with comprehensive categorization
#[derive(Error, Debug, Clone)]
pub enum FontError {
    /// I/O error when reading font files
    #[error("I/O error: {0}")]
    IoError(String),

    /// Font parsing error
    #[error("Font parsing error: {0}")]
    ParseError(String),

    /// Network error when loading web fonts
    #[error("Network error: {0}")]
    NetworkError(String),

    /// Font not found error
    #[error("Font not found: {0}")]
    NotFound(String),

    /// Font loading timeout
    #[error("Font loading timeout")]
    LoadTimeout,

    /// Font load failed with reason
    #[error("Font load failed: {0}")]
    LoadFailed(String),

    /// Invalid font format
    #[error("Invalid font format: {0}")]
    InvalidFormat(String),

    /// Lock error when accessing shared resources
    #[error("Lock error: failed to acquire lock")]
    LockError,

    /// Font system error
    #[error("Font system error: {0}")]
    FontSystemError(String),

    /// Unsupported operation
    #[error("Unsupported operation: {0}")]
    UnsupportedOperation(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Cache error
    #[error("Cache error: {0}")]
    CacheError(String),

    /// Invalid URL format or parsing error
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    /// Cache manager initialization failed
    #[error("Cache initialization failed: {0}")]
    CacheInitializationError(String),
}

/// Result type alias for font operations
pub type FontResult<T> = Result<T, FontError>;

/// Font error severity levels for categorizing error impact
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FontErrorSeverity {
    /// Information - not an error, just informational
    Info,
    /// Warning - non-fatal issue that may impact performance
    Warning,
    /// Error - operation failed but system can continue
    Error,
    /// Critical - system-level failure requiring immediate attention
    Critical,
}

impl std::fmt::Display for FontErrorSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FontErrorSeverity::Info => write!(f, "INFO"),
            FontErrorSeverity::Warning => write!(f, "WARN"),
            FontErrorSeverity::Error => write!(f, "ERROR"),
            FontErrorSeverity::Critical => write!(f, "CRITICAL"),
        }
    }
}

impl FontError {
    /// Get the severity level of this error
    pub fn severity(&self) -> FontErrorSeverity {
        match self {
            FontError::NotFound(_) => FontErrorSeverity::Warning,
            FontError::LoadTimeout => FontErrorSeverity::Warning,
            FontError::ParseError(_) => FontErrorSeverity::Error,
            FontError::InvalidFormat(_) => FontErrorSeverity::Error,
            FontError::NetworkError(_) => FontErrorSeverity::Error,
            FontError::LoadFailed(_) => FontErrorSeverity::Error,
            FontError::ConfigError(_) => FontErrorSeverity::Error,
            FontError::CacheError(_) => FontErrorSeverity::Error,
            FontError::UnsupportedOperation(_) => FontErrorSeverity::Error,
            FontError::InvalidUrl(_) => FontErrorSeverity::Error,
            FontError::IoError(_) => FontErrorSeverity::Critical,
            FontError::LockError => FontErrorSeverity::Critical,
            FontError::FontSystemError(_) => FontErrorSeverity::Critical,
            FontError::CacheInitializationError(_) => FontErrorSeverity::Critical,
        }
    }

    /// Check if this error is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            FontError::NotFound(_) => true,
            FontError::LoadTimeout => true,
            FontError::NetworkError(_) => true,
            FontError::LoadFailed(_) => true,
            FontError::ParseError(_) => false,
            FontError::InvalidFormat(_) => false,
            FontError::IoError(_) => false,
            FontError::LockError => false,
            FontError::FontSystemError(_) => false,
            FontError::UnsupportedOperation(_) => false,
            FontError::ConfigError(_) => false,
            FontError::CacheError(_) => true,
            FontError::InvalidUrl(_) => false, // URL parsing errors are not recoverable
            FontError::CacheInitializationError(_) => false, // System-level failures
        }
    }

    /// Get error category as string
    pub fn category(&self) -> &'static str {
        match self {
            FontError::IoError(_) => "io",
            FontError::ParseError(_) => "parse",
            FontError::NetworkError(_) => "network",
            FontError::NotFound(_) => "not_found",
            FontError::LoadTimeout => "timeout",
            FontError::LoadFailed(_) => "load_failed",
            FontError::InvalidFormat(_) => "invalid_format",
            FontError::LockError => "lock",
            FontError::FontSystemError(_) => "font_system",
            FontError::UnsupportedOperation(_) => "unsupported",
            FontError::ConfigError(_) => "config",
            FontError::CacheError(_) => "cache",
            FontError::InvalidUrl(_) => "invalid_url",
            FontError::CacheInitializationError(_) => "cache_init",
        }
    }

    /// Get recommended retry strategy
    pub fn retry_strategy(&self) -> RetryStrategy {
        match self {
            FontError::NetworkError(_) => RetryStrategy::ExponentialBackoff { max_attempts: 3 },
            FontError::LoadTimeout => RetryStrategy::LinearBackoff { max_attempts: 2 },
            FontError::LoadFailed(_) => RetryStrategy::SingleRetry,
            FontError::CacheError(_) => RetryStrategy::SingleRetry,
            _ => RetryStrategy::NoRetry,
        }
    }
}

/// Retry strategy recommendations for different error types
#[derive(Debug, Clone, PartialEq)]
pub enum RetryStrategy {
    /// Do not retry the operation
    NoRetry,
    /// Retry once immediately
    SingleRetry,
    /// Linear backoff with fixed delay
    LinearBackoff { max_attempts: u8 },
    /// Exponential backoff with increasing delay
    ExponentialBackoff { max_attempts: u8 },
}

/// Font warning types for non-fatal issues
#[derive(Debug, Clone)]
pub enum FontWarning {
    /// Font file is corrupted but partially usable
    CorruptedFont { path: String, details: String },
    /// Font is missing glyphs for requested characters
    MissingGlyphs { font: String, characters: Vec<char> },
    /// Font fallback chain is being used
    FallbackUsed { requested: String, fallback: String },
    /// Cache is near capacity
    CacheNearCapacity { current: usize, max: usize },
    /// Slow font loading operation
    SlowLoading { url: String, duration_ms: u64 },
    /// Deprecated font format
    DeprecatedFormat { format: String, recommended: String },
}

impl std::fmt::Display for FontWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FontWarning::CorruptedFont { path, details } => {
                write!(f, "Corrupted font '{}': {}", path, details)
            }
            FontWarning::MissingGlyphs { font, characters } => {
                write!(
                    f,
                    "Font '{}' missing glyphs for characters: {:?}",
                    font, characters
                )
            }
            FontWarning::FallbackUsed {
                requested,
                fallback,
            } => {
                write!(
                    f,
                    "Using fallback font '{}' for requested font '{}'",
                    fallback, requested
                )
            }
            FontWarning::CacheNearCapacity { current, max } => {
                write!(f, "Font cache near capacity: {}/{} entries", current, max)
            }
            FontWarning::SlowLoading { url, duration_ms } => {
                write!(f, "Slow font loading from '{}': {}ms", url, duration_ms)
            }
            FontWarning::DeprecatedFormat {
                format,
                recommended,
            } => {
                write!(
                    f,
                    "Deprecated font format '{}', recommend '{}'",
                    format, recommended
                )
            }
        }
    }
}
