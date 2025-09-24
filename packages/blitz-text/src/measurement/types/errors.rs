//! Error type definitions for measurement system
//!
//! This module contains error types and result types used throughout
//! the text measurement system for proper error handling.

/// Measurement system errors
#[derive(Debug, thiserror::Error)]
pub enum MeasurementError {
    #[error("Font system error during measurement")]
    FontSystemError,

    #[error("Invalid text for measurement")]
    InvalidText,

    #[error("No lines found in measured text")]
    NoLinesFound,

    #[error("Measurement cache error")]
    CacheError,

    #[error("Buffer error during measurement")]
    BufferError,

    #[error("Font metrics extraction failed: {0}")]
    FontMetricsError(String),
}

impl From<Box<dyn std::error::Error + Send + Sync>> for MeasurementError {
    fn from(err: Box<dyn std::error::Error + Send + Sync>) -> Self {
        MeasurementError::FontMetricsError(err.to_string())
    }
}

/// Result type for measurement operations
pub type MeasurementResult<T> = Result<T, MeasurementError>;
