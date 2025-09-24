use std::path::PathBuf;

use url::Url;

use crate::error::types::FontError;

/// Context information for font operations to aid in debugging
#[derive(Debug, Clone)]
pub struct FontErrorContext {
    pub operation: String,
    pub font_family: Option<String>,
    pub font_url: Option<Url>,
    pub file_path: Option<PathBuf>,
    pub additional_info: Vec<String>,
}

impl FontErrorContext {
    /// Create a new error context
    pub fn new(operation: impl Into<String>) -> Self {
        Self {
            operation: operation.into(),
            font_family: None,
            font_url: None,
            file_path: None,
            additional_info: Vec::new(),
        }
    }

    /// Set font family context
    pub fn with_font_family(mut self, family: impl Into<String>) -> Self {
        self.font_family = Some(family.into());
        self
    }

    /// Set font URL context
    pub fn with_url(mut self, url: Url) -> Self {
        self.font_url = Some(url);
        self
    }

    /// Set file path context
    pub fn with_path(mut self, path: PathBuf) -> Self {
        self.file_path = Some(path);
        self
    }

    /// Add additional context information
    pub fn with_info(mut self, info: impl Into<String>) -> Self {
        self.additional_info.push(info.into());
        self
    }

    /// Create a contextualized error
    pub fn error(self, error: FontError) -> ContextualizedFontError {
        ContextualizedFontError {
            error,
            context: self,
        }
    }

    /// Create error context for font loading operations
    pub fn for_font_loading(family: impl Into<String>) -> Self {
        Self::new("font_loading").with_font_family(family)
    }

    /// Create error context for web font operations
    pub fn for_web_font(url: Url) -> Self {
        Self::new("web_font_loading").with_url(url)
    }

    /// Create error context for system font operations
    pub fn for_system_font(path: PathBuf) -> Self {
        Self::new("system_font_loading").with_path(path)
    }

    /// Create error context for font parsing operations
    pub fn for_font_parsing(path: PathBuf) -> Self {
        Self::new("font_parsing").with_path(path)
    }

    /// Create error context for cache operations
    pub fn for_cache_operation(operation: &str) -> Self {
        Self::new(format!("cache_{}", operation))
    }
}

/// A font error with additional context information
#[derive(Debug, Clone)]
pub struct ContextualizedFontError {
    pub error: FontError,
    pub context: FontErrorContext,
}

impl std::fmt::Display for ContextualizedFontError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} during {}", self.error, self.context.operation)?;

        if let Some(ref family) = self.context.font_family {
            write!(f, " (family: {})", family)?;
        }

        if let Some(ref url) = self.context.font_url {
            write!(f, " (url: {})", url)?;
        }

        if let Some(ref path) = self.context.file_path {
            write!(f, " (path: {})", path.display())?;
        }

        if !self.context.additional_info.is_empty() {
            write!(f, " [{}]", self.context.additional_info.join(", "))?;
        }

        Ok(())
    }
}

impl std::error::Error for ContextualizedFontError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}

impl From<FontError> for ContextualizedFontError {
    fn from(error: FontError) -> Self {
        Self {
            error,
            context: FontErrorContext::new("unknown_operation"),
        }
    }
}

impl ContextualizedFontError {
    /// Create a new contextualized error
    pub fn new(error: FontError, context: FontErrorContext) -> Self {
        Self { error, context }
    }

    /// Get the underlying error
    pub fn inner_error(&self) -> &FontError {
        &self.error
    }

    /// Get the context
    pub fn context(&self) -> &FontErrorContext {
        &self.context
    }

    /// Add additional context to this error
    pub fn with_context_info(mut self, info: impl Into<String>) -> Self {
        self.context.additional_info.push(info.into());
        self
    }

    /// Check if this is a recoverable error
    pub fn is_recoverable(&self) -> bool {
        self.error.is_recoverable()
    }

    /// Get error severity
    pub fn severity(&self) -> crate::error::types::FontErrorSeverity {
        self.error.severity()
    }
}
