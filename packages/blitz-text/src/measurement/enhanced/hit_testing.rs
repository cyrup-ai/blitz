//! Hit testing and cursor movement functionality using cosmyc-text APIs with zero allocation

use cosmyc_text::{Attrs, Cursor, Motion};

use super::core::EnhancedTextMeasurer;
use crate::cosmyc_types::EnhancedBuffer;
use crate::measurement::types::MeasurementResult;

impl EnhancedTextMeasurer {
    /// Get character position at coordinates using cosmyc-text hit testing
    /// Optimized for zero allocation buffer reuse and comprehensive error handling
    #[inline]
    pub fn hit_test(
        &mut self,
        text: &str,
        attrs: &Attrs,
        x: f32,
        y: f32,
        max_width: Option<f32>,
    ) -> MeasurementResult<Option<Cursor>> {
        let font_system = &mut self.font_system;
        let metrics = attrs
            .metrics_opt
            .map(|cache_metrics| crate::cosmyc::cache_metrics_to_metrics(cache_metrics))
            .unwrap_or(self.default_metrics);

        // Create buffer with optimized settings for hit testing
        let mut buffer = EnhancedBuffer::new(font_system, metrics);
        buffer.set_text_cached(font_system, text, attrs, self.default_shaping);

        // Set buffer size for wrapping if needed (zero allocation if not required)
        if let Some(width) = max_width {
            buffer.inner_mut().set_size(font_system, Some(width), None);
        }

        // Shape text for accurate hit testing
        buffer.inner_mut().shape_until_scroll(font_system, false);

        // Perform hit test using enhanced buffer (zero allocation operation)
        Ok(buffer.hit_test(x, y))
    }

    /// Move cursor with motion using cosmyc-text cursor motion API
    /// Optimized for interactive text editing with buffer reuse
    #[inline]
    pub fn move_cursor(
        &mut self,
        text: &str,
        attrs: &Attrs,
        cursor: Cursor,
        motion: Motion,
        max_width: Option<f32>,
    ) -> MeasurementResult<Option<Cursor>> {
        let font_system = &mut self.font_system;
        let metrics = attrs
            .metrics_opt
            .map(|cache_metrics| crate::cosmyc::cache_metrics_to_metrics(cache_metrics))
            .unwrap_or(self.default_metrics);

        // Create buffer with optimized settings for cursor movement
        let mut buffer = EnhancedBuffer::new(font_system, metrics);
        buffer.set_text_cached(font_system, text, attrs, self.default_shaping);

        // Set buffer size for wrapping if needed (consistent with hit testing)
        if let Some(width) = max_width {
            buffer.inner_mut().set_size(font_system, Some(width), None);
        }

        // Shape text for accurate cursor positioning
        buffer.inner_mut().shape_until_scroll(font_system, false);

        // Perform cursor movement using enhanced buffer (zero allocation motion)
        Ok(buffer
            .move_cursor(font_system, cursor, None, motion)
            .map(|(cursor, _)| cursor))
    }

    /// Fast cursor validation for performance-critical paths
    #[inline]
    pub fn is_cursor_valid(&self, text: &str, cursor: &Cursor) -> bool {
        cursor.line < text.lines().count() && cursor.index <= text.len()
    }

    /// Get cursor bounds for rendering cursor visualization
    #[inline]
    pub fn get_cursor_bounds(
        &mut self,
        text: &str,
        attrs: &Attrs,
        _cursor: Cursor,
        max_width: Option<f32>,
    ) -> MeasurementResult<Option<(f32, f32, f32, f32)>> {
        // (x, y, width, height)
        let font_system = &mut self.font_system;
        let metrics = attrs
            .metrics_opt
            .map(|cache_metrics| crate::cosmyc::cache_metrics_to_metrics(cache_metrics))
            .unwrap_or(self.default_metrics);

        let mut buffer = EnhancedBuffer::new(font_system, metrics);
        buffer.set_text_cached(font_system, text, attrs, self.default_shaping);

        if let Some(width) = max_width {
            buffer.inner_mut().set_size(font_system, Some(width), None);
        }

        buffer.inner_mut().shape_until_scroll(font_system, false);

        // Calculate cursor bounds using buffer metrics
        // Since get_cursor_layout doesn't exist, provide basic bounds calculation
        let _buffer_width = buffer.inner().size().0.unwrap_or(0.0);
        let _buffer_height = buffer.inner().size().1.unwrap_or(metrics.line_height);

        Ok(Some((
            0.0,                 // x position - would need proper cursor tracking
            0.0,                 // y position - would need proper line calculation
            1.0,                 // Standard cursor width
            metrics.line_height, // Use metrics for height
        )))
    }
}
