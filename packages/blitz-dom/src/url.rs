use std::ops::Deref;
use std::str::FromStr;

use style::servo_arc::Arc as ServoArc;
use style::stylesheets::UrlExtraData;
use url::Url;

#[derive(Debug, Clone)]
pub enum DocumentUrlError {
    AllFallbacksFailed,
}

impl std::fmt::Display for DocumentUrlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DocumentUrlError::AllFallbacksFailed => write!(f, "All URL fallback mechanisms failed"),
        }
    }
}

impl std::error::Error for DocumentUrlError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DocumentUrlError::AllFallbacksFailed => None,
        }
    }
}

/// Document URL management system for CSS resource resolution
/// 
/// Provides robust URL parsing with multiple fallback mechanisms
/// for handling various URL formats in document contexts.
/// 
/// The DocumentUrl manages a base URL that serves as the foundation for
/// resolving relative URLs in CSS stylesheets, HTML documents, and other
/// web resources. It integrates seamlessly with the Stylo CSS engine
/// through UrlExtraData creation.
/// 
/// # Examples
/// 
/// ```rust
/// # use blitz_dom::url::DocumentUrl;
/// # use std::str::FromStr;
/// let doc_url = DocumentUrl::from_str("https://example.com/")?;
/// let resolved = doc_url.resolve_relative("styles.css");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone)]
pub(crate) struct DocumentUrl {
    base_url: ServoArc<Url>,
}

impl DocumentUrl {
    /// Creates stylo `UrlExtraData` for CSS engine integration
    /// 
    /// This method provides the interface between DocumentUrl and Servo's
    /// Stylo CSS engine, enabling proper URL resolution in CSS contexts.
    /// 
    /// # Returns
    /// UrlExtraData wrapping the base URL for stylo processing
    pub(crate) fn url_extra_data(&self) -> UrlExtraData {
        UrlExtraData(ServoArc::clone(&self.base_url))
    }

    /// Resolves a relative URL against this document's base URL
    /// 
    /// Attempts to resolve the provided relative URL string against the
    /// document's base URL using standard URL resolution rules.
    /// 
    /// # Arguments
    /// * `raw` - The relative URL string to resolve
    /// 
    /// # Returns
    /// Some(Url) if resolution succeeds, None if the relative URL is invalid
    /// 
    /// # Examples
    /// ```rust
    /// # use blitz_dom::url::DocumentUrl;
    /// # use std::str::FromStr;
    /// let base = DocumentUrl::from_str("https://example.com/page/")?;
    /// let resolved = base.resolve_relative("../other.html");
    /// assert!(resolved.is_some());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub(crate) fn resolve_relative(&self, raw: &str) -> Option<url::Url> {
        self.base_url.join(raw).ok()
    }

    /// Creates a fallback URL when all standard URL creation methods fail
    /// 
    /// This method provides the last resort for URL creation by attempting
    /// multiple fallback strategies in order of preference:
    /// 1. data: URL (minimal and always valid)
    /// 2. file:/// URL (local file system root)
    /// 3. about:blank URL (standard browser placeholder)
    /// 
    /// # Returns
    /// Result containing a DocumentUrl with a working fallback URL,
    /// or DocumentUrlError::AllFallbacksFailed if even basic URL parsing fails
    fn create_stub_url() -> Result<Self, DocumentUrlError> {
        // Create a minimal working URL with proper validation
        // First, try to create a valid data URL as it's the most minimal
        if let Ok(data_url) = url::Url::parse("data:") {
            return Ok(Self {
                base_url: ServoArc::new(data_url),
            });
        }

        // If data URL fails, try file system root
        if let Ok(file_url) = url::Url::from_file_path("/") {
            return Ok(Self {
                base_url: ServoArc::new(file_url),
            });
        }

        // If file path fails, try about:blank (standard browser placeholder)
        if let Ok(about_url) = url::Url::parse("about:blank") {
            return Ok(Self {
                base_url: ServoArc::new(about_url),
            });
        }

        // Return an error instead of terminating process
        Err(DocumentUrlError::AllFallbacksFailed)
    }
}

impl Default for DocumentUrl {
    fn default() -> Self {
        // Try to create a proper default URL with validation
        // Start with the most appropriate default for document contexts

        // First try: about:blank (standard browser default for empty documents)
        if let Ok(url) = Self::from_str("about:blank") {
            return url;
        }

        // Second try: empty data URL (minimal valid URL)
        if let Ok(url) = Self::from_str("data:") {
            return url;
        }

        // Third try: file system root (for local documents)
        if let Ok(url) = url::Url::from_file_path("/").map(Self::from) {
            return url;
        }

        // If all standard methods fail, URL parsing system has issues
        eprintln!("WARNING: Standard URL creation failed, using fallback");
        match Self::create_stub_url() {
            Ok(url) => url,
            Err(e) => panic!("Failed to create default DocumentUrl: {}", e),
        }
    }
}
impl FromStr for DocumentUrl {
    type Err = url::ParseError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        Self::from_str_with_validation(input)
    }
}

impl DocumentUrl {
    /// Parses a URL string with comprehensive validation and error handling
    /// 
    /// This method provides robust URL parsing with multiple validation steps:
    /// 1. Input validation (non-empty, trimmed)
    /// 2. Direct URL parsing attempt
    /// 3. Relative URL resolution with appropriate base URLs
    /// 
    /// # Arguments
    /// * `input` - The URL string to parse
    /// 
    /// # Returns
    /// Result containing a DocumentUrl or url::ParseError for invalid input
    /// 
    /// # Errors
    /// Returns url::ParseError if the input cannot be parsed as a valid URL
    /// even with relative URL resolution attempts
    fn from_str_with_validation(input: &str) -> Result<Self, url::ParseError> {
        // 1. Validate input is not empty/whitespace
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err(url::ParseError::EmptyHost);
        }

        // 2. Try direct parsing
        match url::Url::parse(trimmed) {
            Ok(url) => Ok(Self {
                base_url: ServoArc::new(url),
            }),
            Err(url::ParseError::RelativeUrlWithoutBase) => {
                // 3. Try as relative URL with sensible base
                Self::parse_relative_with_base(trimmed)
            }
            Err(e) => Err(e),
        }
    }

    /// Parses a relative URL by attempting resolution with common base URLs
    /// 
    /// When a URL cannot be parsed directly, this method attempts to resolve
    /// it as a relative URL using a series of common base URLs that are likely
    /// to work in document contexts.
    /// 
    /// # Arguments  
    /// * `relative_url` - The relative URL string to resolve
    /// 
    /// # Returns
    /// Result containing a DocumentUrl with the resolved URL or the original
    /// RelativeUrlWithoutBase error if resolution fails with all base URLs
    fn parse_relative_with_base(relative_url: &str) -> Result<Self, url::ParseError> {
        // Try with common base URLs for relative resolution
        let base_urls = ["about:blank", "data:"];

        for base_str in &base_urls {
            if let Ok(base) = url::Url::parse(base_str) {
                if let Ok(resolved) = base.join(relative_url) {
                    return Ok(Self {
                        base_url: ServoArc::new(resolved),
                    });
                }
            }
        }

        // If relative parsing fails, return the original error
        Err(url::ParseError::RelativeUrlWithoutBase)
    }
}
impl From<Url> for DocumentUrl {
    fn from(base_url: Url) -> Self {
        Self {
            base_url: ServoArc::new(base_url),
        }
    }
}
impl From<ServoArc<Url>> for DocumentUrl {
    fn from(base_url: ServoArc<Url>) -> Self {
        Self { base_url }
    }
}
impl Deref for DocumentUrl {
    type Target = Url;
    fn deref(&self) -> &Self::Target {
        &self.base_url
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_str_valid_urls() {
        // Test various valid URL formats
        let test_cases = vec![
            "https://example.com/",
            "http://localhost:8080/",
            "file:///path/to/file.html",
            "data:text/html,<h1>Hello</h1>",
            "about:blank",
        ];

        for url_str in test_cases {
            let result = DocumentUrl::from_str(url_str);
            assert!(result.is_ok(), "Failed to parse valid URL: {}", url_str);
            
            let doc_url = result.unwrap_or_else(|_| panic!("Should have parsed: {}", url_str));
            assert_eq!(doc_url.as_str(), url_str);
        }
    }

    #[test]
    fn test_from_str_invalid_urls() {
        // Test error handling for invalid URLs
        let invalid_cases = vec![
            "",           // Empty string
            "   ",        // Whitespace only
            "not a url",  // Invalid format
            "://missing-scheme",
            "http://",    // Incomplete URL
        ];

        for invalid_url in invalid_cases {
            let result = DocumentUrl::from_str(invalid_url);
            assert!(result.is_err(), "Should have failed to parse: {}", invalid_url);
        }
    }

    #[test]
    fn test_default_fallback_chain() {
        // Test default implementation fallbacks
        let default_url = DocumentUrl::default();
        
        // Should create a valid URL without panicking
        assert!(!default_url.as_str().is_empty());
        
        // Should be one of the expected fallback URLs
        let url_str = default_url.as_str();
        let is_expected_fallback = url_str == "about:blank" 
            || url_str == "data:" 
            || url_str == "file:///";
            
        assert!(is_expected_fallback, "Unexpected fallback URL: {}", url_str);
    }

    #[test]
    fn test_relative_url_resolution() {
        // Test resolve_relative method
        let base = DocumentUrl::from_str("https://example.com/path/page.html")
            .expect("Should parse base URL");

        // Test successful relative resolution
        let relative_cases = vec![
            ("./style.css", "https://example.com/path/style.css"),
            ("../other.html", "https://example.com/other.html"),
            ("subdir/file.js", "https://example.com/path/subdir/file.js"),
            ("/absolute.css", "https://example.com/absolute.css"),
        ];

        for (relative, expected) in relative_cases {
            let resolved = base.resolve_relative(relative);
            assert!(resolved.is_some(), "Failed to resolve: {}", relative);
            assert_eq!(resolved.unwrap().as_str(), expected);
        }

        // Test invalid relative URLs
        let invalid_relative = base.resolve_relative("://invalid");
        assert!(invalid_relative.is_none(), "Should fail to resolve invalid relative URL");
    }

    #[test]
    fn test_url_extra_data_creation() {
        // Test stylo integration
        let doc_url = DocumentUrl::from_str("https://example.com/test.html")
            .expect("Should parse test URL");

        let extra_data = doc_url.url_extra_data();
        
        // Verify the UrlExtraData contains our URL
        assert_eq!(extra_data.0.as_str(), "https://example.com/test.html");
    }

    #[test]
    fn test_clone_functionality() {
        // Test that DocumentUrl clones properly
        let original = DocumentUrl::from_str("https://example.com/")
            .expect("Should parse URL");

        let cloned = original.clone();
        assert_eq!(original.as_str(), cloned.as_str());

        // Verify they point to the same underlying URL but are separate instances
        assert_eq!(original.resolve_relative("test.css"), cloned.resolve_relative("test.css"));
    }

    #[test]
    fn test_create_stub_url_success() {
        // Test that create_stub_url returns a valid URL
        let result = DocumentUrl::create_stub_url();
        assert!(result.is_ok(), "create_stub_url should succeed");

        let stub_url = result.unwrap_or_else(|_| panic!("Should create stub URL"));
        assert!(!stub_url.as_str().is_empty());
    }

    #[test]
    fn test_from_url_conversion() {
        // Test From<Url> implementation
        let url = url::Url::parse("https://example.com/").expect("Should parse URL");
        let doc_url = DocumentUrl::from(url.clone());
        assert_eq!(doc_url.as_str(), url.as_str());
    }

    #[test]
    fn test_from_servo_arc_conversion() {
        // Test From<ServoArc<Url>> implementation
        let url = url::Url::parse("https://example.com/").expect("Should parse URL");
        let servo_arc = ServoArc::new(url.clone());
        let doc_url = DocumentUrl::from(servo_arc);
        assert_eq!(doc_url.as_str(), url.as_str());
    }

    #[test]
    fn test_deref_functionality() {
        // Test Deref implementation allows direct URL method access
        let doc_url = DocumentUrl::from_str("https://example.com/path")
            .expect("Should parse URL");

        // Should be able to call URL methods directly
        assert_eq!(doc_url.scheme(), "https");
        assert_eq!(doc_url.host_str(), Some("example.com"));
        assert_eq!(doc_url.path(), "/path");
    }

    #[test]
    fn test_error_types() {
        // Test DocumentUrlError functionality
        let fallback_error = DocumentUrlError::AllFallbacksFailed;

        // Test Display implementation
        assert!(!format!("{}", fallback_error).is_empty());

        // Test Debug implementation
        assert!(!format!("{:?}", fallback_error).is_empty());

        // Test Error trait
        assert!(std::error::Error::source(&fallback_error).is_none());
    }
}
