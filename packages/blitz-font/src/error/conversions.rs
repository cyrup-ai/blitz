use crate::error::types::FontError;

/// From implementations for converting various error types to FontError
impl From<std::io::Error> for FontError {
    fn from(error: std::io::Error) -> Self {
        FontError::IoError(error.to_string())
    }
}

impl From<ttf_parser::FaceParsingError> for FontError {
    fn from(error: ttf_parser::FaceParsingError) -> Self {
        FontError::ParseError(format!("TTF parsing failed: {:?}", error))
    }
}

#[cfg(feature = "web-fonts")]
impl From<reqwest::Error> for FontError {
    fn from(error: reqwest::Error) -> Self {
        FontError::NetworkError(error.to_string())
    }
}

impl From<url::ParseError> for FontError {
    fn from(err: url::ParseError) -> Self {
        FontError::InvalidUrl(match err {
            url::ParseError::EmptyHost => "URL contains empty host".to_string(),
            url::ParseError::IdnaError => "Invalid international domain name".to_string(),
            url::ParseError::InvalidPort => "Invalid port number in URL".to_string(),
            url::ParseError::InvalidIpv4Address => "Invalid IPv4 address in URL".to_string(),
            url::ParseError::InvalidIpv6Address => "Invalid IPv6 address in URL".to_string(),
            url::ParseError::InvalidDomainCharacter => {
                "Invalid domain character in URL".to_string()
            }
            url::ParseError::RelativeUrlWithoutBase => {
                "Relative URL provided without base URL".to_string()
            }
            url::ParseError::RelativeUrlWithCannotBeABaseBase => {
                "Invalid base URL for relative resolution".to_string()
            }
            url::ParseError::SetHostOnCannotBeABaseUrl => {
                "Cannot set host on cannot-be-a-base URL".to_string()
            }
            url::ParseError::Overflow => "URL exceeds maximum supported length (4GB)".to_string(),
            _ => format!("URL parsing error: {:?}", err),
        })
    }
}

#[allow(unexpected_cfgs)]
#[cfg(feature = "serde")]
impl From<serde_json::Error> for FontError {
    fn from(error: serde_json::Error) -> Self {
        FontError::ParseError(format!("JSON parsing failed: {}", error))
    }
}

impl From<std::string::FromUtf8Error> for FontError {
    fn from(error: std::string::FromUtf8Error) -> Self {
        FontError::InvalidFormat(format!("Invalid UTF-8: {}", error))
    }
}

impl From<std::num::ParseIntError> for FontError {
    fn from(error: std::num::ParseIntError) -> Self {
        FontError::ParseError(format!("Integer parsing failed: {}", error))
    }
}

impl From<std::num::ParseFloatError> for FontError {
    fn from(error: std::num::ParseFloatError) -> Self {
        FontError::ParseError(format!("Float parsing failed: {}", error))
    }
}

#[cfg(feature = "web-fonts")]
impl From<tokio::time::error::Elapsed> for FontError {
    fn from(_: tokio::time::error::Elapsed) -> Self {
        FontError::LoadTimeout
    }
}

impl From<Box<dyn std::error::Error + Send + Sync>> for FontError {
    fn from(error: Box<dyn std::error::Error + Send + Sync>) -> Self {
        FontError::FontSystemError(error.to_string())
    }
}

/// Conversion utilities for error handling
impl FontError {
    /// Create an IoError from a file path and error message
    pub fn io_error_with_path(path: &std::path::Path, message: impl Into<String>) -> Self {
        FontError::IoError(format!("{}: {}", path.display(), message.into()))
    }

    /// Create a ParseError with file context
    pub fn parse_error_with_context(file: &str, message: impl Into<String>) -> Self {
        FontError::ParseError(format!("{}: {}", file, message.into()))
    }

    /// Create a NetworkError with URL context
    pub fn network_error_with_url(url: &url::Url, message: impl Into<String>) -> Self {
        FontError::NetworkError(format!("{}: {}", url, message.into()))
    }

    /// Create a NotFound error with detailed context
    pub fn not_found_with_context(resource_type: &str, identifier: impl Into<String>) -> Self {
        FontError::NotFound(format!(
            "{} not found: {}",
            resource_type,
            identifier.into()
        ))
    }

    /// Create a LoadFailed error with detailed context
    pub fn load_failed_with_reason(resource: impl Into<String>, reason: impl Into<String>) -> Self {
        FontError::LoadFailed(format!(
            "Failed to load {}: {}",
            resource.into(),
            reason.into()
        ))
    }

    /// Create an InvalidFormat error with format details
    pub fn invalid_format_with_details(format: &str, details: impl Into<String>) -> Self {
        FontError::InvalidFormat(format!("Invalid {} format: {}", format, details.into()))
    }

    /// Create a ConfigError with configuration context
    pub fn config_error_with_field(field: &str, message: impl Into<String>) -> Self {
        FontError::ConfigError(format!(
            "Configuration error in '{}': {}",
            field,
            message.into()
        ))
    }

    /// Create a CacheError with operation context
    pub fn cache_error_with_operation(operation: &str, message: impl Into<String>) -> Self {
        FontError::CacheError(format!("Cache {} failed: {}", operation, message.into()))
    }
}
