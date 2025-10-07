//! Measurement request types for text measurement operations

use serde::{Deserialize, Serialize};

/// Request for text measurement operations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeasurementRequest {
    /// The text to measure
    pub text: String,
    /// Font ID for measurement (simplified to avoid external type)
    pub font_id: u32,
    /// Font size in points
    pub font_size: f32,
    /// Maximum width constraint (optional)
    pub max_width: Option<f32>,
    /// Whether to enable text shaping features
    pub enable_shaping: bool,
    /// Language code for text processing
    pub language: Option<String>,
    /// Text direction (for bidirectional text)
    pub direction: Option<TextDirection>,
    /// Font family name for text shaping
    pub font_family: Option<String>,
}

/// Text direction for bidirectional text support
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextDirection {
    /// Left-to-right text
    LeftToRight,
    /// Right-to-left text
    RightToLeft,
    /// Auto-detect direction
    Auto,
}

impl Default for TextDirection {
    fn default() -> Self {
        Self::LeftToRight
    }
}

impl MeasurementRequest {
    /// Create a new measurement request with minimal parameters
    pub fn new(text: String, font_id: u32, font_size: f32) -> Self {
        Self {
            text,
            font_id,
            font_size,
            max_width: None,
            enable_shaping: true,
            language: None,
            direction: Some(TextDirection::Auto),
            font_family: None,
        }
    }

    /// Create a measurement request with width constraint
    pub fn with_max_width(text: String, font_id: u32, font_size: f32, max_width: f32) -> Self {
        Self {
            text,
            font_id,
            font_size,
            max_width: Some(max_width),
            enable_shaping: true,
            language: None,
            direction: Some(TextDirection::Auto),
            font_family: None,
        }
    }

    /// Set the language for text processing
    pub fn with_language(mut self, language: String) -> Self {
        self.language = Some(language);
        self
    }

    /// Set the text direction
    pub fn with_direction(mut self, direction: TextDirection) -> Self {
        self.direction = Some(direction);
        self
    }

    /// Disable text shaping features
    pub fn without_shaping(mut self) -> Self {
        self.enable_shaping = false;
        self
    }

    /// Get the effective max width (returns a large value if None)
    pub fn effective_max_width(&self) -> f32 {
        self.max_width.unwrap_or(f32::MAX)
    }

    /// Check if this request has width constraints
    pub fn has_width_constraint(&self) -> bool {
        self.max_width.is_some()
    }

    /// Get the text length in characters
    pub fn text_length(&self) -> usize {
        self.text.chars().count()
    }

    /// Check if the text is empty
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }
}

impl Default for MeasurementRequest {
    fn default() -> Self {
        Self {
            text: String::new(),
            font_id: 0, // Default font ID
            font_size: 12.0,
            max_width: None,
            enable_shaping: true,
            language: None,
            direction: Some(TextDirection::Auto),
            font_family: None,
        }
    }
}
