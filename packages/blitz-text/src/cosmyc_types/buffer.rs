//! Enhanced buffer and layout run functionality
//!
//! This module provides enhanced buffer capabilities with caching optimization
//! and comprehensive layout run information extraction.

use std::sync::{Arc, Mutex};
use cosmyc_text::{
    Affinity, Align, Attrs, Buffer, Cursor, FontSystem, LayoutCursor, LayoutRun, Metrics, Motion,
    Shaping,
};
use unicode_segmentation::UnicodeSegmentation;

use super::glyph::GlyphInfo;

/// RAII guard that ensures buffer state is restored even if panics occur
pub struct BufferStateGuard<'a> {
    buffer: &'a mut EnhancedBuffer,
    font_system: &'a mut FontSystem,
    original_width: Option<f32>,
    original_height: Option<f32>,
    needs_cache_update: bool,
}

impl<'a> BufferStateGuard<'a> {
    /// Create a new state guard that will restore state on drop
    pub fn new(
        buffer: &'a mut EnhancedBuffer,
        font_system: &'a mut FontSystem,
    ) -> Self {
        let (original_width, original_height) = buffer.inner().size();
        
        Self {
            buffer,
            font_system,
            original_width,
            original_height,
            needs_cache_update: false,
        }
    }
    
    /// Temporarily modify buffer size for calculations
    pub fn set_temporary_size(&mut self, width: Option<f32>, height: Option<f32>) {
        self.buffer.inner_mut().set_size(self.font_system, width, height);
        self.needs_cache_update = true;
    }
    
    /// Get access to the buffer for calculations
    pub fn buffer(&self) -> &EnhancedBuffer {
        self.buffer
    }
    
    /// Get mutable access to both buffer and font system for calculations
    pub fn with_buffer_and_font_system<F, R>(&mut self, f: F) -> R 
    where
        F: FnOnce(&mut EnhancedBuffer, &mut FontSystem) -> R,
    {
        f(self.buffer, self.font_system)
    }
}

impl<'a> Drop for BufferStateGuard<'a> {
    fn drop(&mut self) {
        // Always restore original state, even if panic occurred
        self.buffer.inner_mut().set_size(
            self.font_system,
            self.original_width,
            self.original_height,
        );
        
        // Update cache if needed
        if self.needs_cache_update {
            self.buffer.update_cached_layout_runs();
        }
    }
}

/// Error types for CSS width calculations
#[derive(Debug)]
pub enum CssWidthCalculationError {
    FontSystemError(String),
    LayoutError(String),
    StateCorruption,
    SyncError,
}

impl std::fmt::Display for CssWidthCalculationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CssWidthCalculationError::FontSystemError(msg) => write!(f, "Font system error: {}", msg),
            CssWidthCalculationError::LayoutError(msg) => write!(f, "Layout calculation failed: {}", msg),
            CssWidthCalculationError::StateCorruption => write!(f, "State corruption detected"),
            CssWidthCalculationError::SyncError => write!(f, "Thread synchronization failed"),
        }
    }
}

impl std::error::Error for CssWidthCalculationError {}

/// Performance metrics for CSS width calculations
#[derive(Debug, Default)]
pub struct CssWidthMetrics {
    pub calculation_count: u64,
    pub total_duration: std::time::Duration,
    pub error_count: u64,
    pub cache_hit_count: u64,
    pub cache_miss_count: u64,
}

/// Thread-safe wrapper for EnhancedBuffer calculations
pub struct ThreadSafeBufferCalculator {
    buffer: Arc<Mutex<EnhancedBuffer>>,
}

impl ThreadSafeBufferCalculator {
    pub fn new(buffer: EnhancedBuffer) -> Self {
        Self {
            buffer: Arc::new(Mutex::new(buffer)),
        }
    }
    
    /// Perform CSS width calculation with thread safety
    pub fn calculate_css_widths(
        &self,
        font_system: &mut FontSystem,
    ) -> Result<(f32, f32), Box<dyn std::error::Error>> {
        let mut buffer = self.buffer.lock()
            .map_err(|_| "Buffer lock poisoned")?;
        
        let min_width = buffer.css_min_content_width(font_system);
        let max_width = buffer.css_max_content_width(font_system);
        
        Ok((min_width, max_width))
    }
}

/// Enhanced Buffer wrapper with additional functionality
#[derive(Debug, Clone)]
pub struct EnhancedBuffer {
    inner: Buffer,
    cached_layout_runs: Vec<LayoutRunInfo>,
    last_shaped_text: String,
}

impl EnhancedBuffer {
    /// Create new enhanced buffer
    pub fn new(font_system: &mut FontSystem, metrics: Metrics) -> Self {
        Self {
            inner: Buffer::new(font_system, metrics),
            cached_layout_runs: Vec::new(),
            last_shaped_text: String::new(),
        }
    }

    /// Create new empty enhanced buffer
    pub fn new_empty(metrics: Metrics) -> Self {
        Self {
            inner: Buffer::new_empty(metrics),
            cached_layout_runs: Vec::new(),
            last_shaped_text: String::new(),
        }
    }

    /// Get reference to inner buffer
    pub fn inner(&self) -> &Buffer {
        &self.inner
    }

    /// Get mutable reference to inner buffer
    pub fn inner_mut(&mut self) -> &mut Buffer {
        &mut self.inner
    }

    /// Set text with caching optimization
    pub fn set_text_cached(
        &mut self,
        font_system: &mut FontSystem,
        text: &str,
        attrs: &Attrs,
        shaping: Shaping,
    ) {
        // Only update if text changed
        if text != self.last_shaped_text {
            self.inner.set_text(font_system, text, attrs, shaping);
            self.last_shaped_text = text.to_string();
            self.update_cached_layout_runs();
        }
    }

    /// Set rich text with spans
    pub fn set_rich_text_cached<'r, 's, I>(
        &mut self,
        font_system: &mut FontSystem,
        spans: I,
        default_attrs: &Attrs,
        shaping: Shaping,
        alignment: Option<Align>,
    ) where
        I: Iterator<Item = (&'s str, Attrs<'r>)>,
    {
        self.inner
            .set_rich_text(font_system, spans, default_attrs, shaping, alignment);
        self.update_cached_layout_runs();
    }

    /// Set buffer size with cache invalidation
    pub fn set_size_cached(
        &mut self,
        font_system: &mut FontSystem,
        width: Option<f32>,
        height: Option<f32>,
    ) {
        self.inner.set_size(font_system, width, height);
        self.update_cached_layout_runs();
    }

    /// Set buffer wrap with cache invalidation
    pub fn set_wrap_cached(&mut self, font_system: &mut FontSystem, wrap: cosmyc_text::Wrap) {
        self.inner.set_wrap(font_system, wrap);
        self.update_cached_layout_runs();
    }

    /// Calculate CSS min-content width with exception safety
    pub fn css_min_content_width(&mut self, font_system: &mut FontSystem) -> f32 {
        let mut guard = BufferStateGuard::new(self, font_system);
        
        // Temporarily set infinite width - state will be restored automatically
        guard.set_temporary_size(Some(f32::INFINITY), None);
        
        let text = guard.buffer().extract_text_content();
        let mut max_unbreakable_width = 0.0f32;
        
        // Performance optimization: ASCII fast-path for common cases
        if text.is_ascii() {
            // Fast path for ASCII-only text: use simple whitespace splitting
            for word in text.split_whitespace() {
                let word_width = guard.with_buffer_and_font_system(|buffer, font_system| {
                    buffer.measure_text_sequence(font_system, word)
                });
                max_unbreakable_width = max_unbreakable_width.max(word_width);
            }
        } else {
            // Unicode path for complex scripts
            // Phase 1A: Unicode word boundary detection
            for word in text.unicode_words() {
                let word_width = guard.with_buffer_and_font_system(|buffer, font_system| {
                    buffer.measure_text_sequence(font_system, word)
                });
                max_unbreakable_width = max_unbreakable_width.max(word_width);
            }
            
            // Phase 1B: Handle CJK and complex scripts with grapheme boundaries
            for grapheme_cluster in text.graphemes(true) {
                if guard.buffer().is_unbreakable_sequence(grapheme_cluster) {
                    let cluster_width = guard.with_buffer_and_font_system(|buffer, font_system| {
                        buffer.measure_text_sequence(font_system, grapheme_cluster)
                    });
                    max_unbreakable_width = max_unbreakable_width.max(cluster_width);
                }
            }
        }
        
        // Phase 1C: Handle inline elements (replaced content)
        max_unbreakable_width = max_unbreakable_width.max(guard.buffer().measure_inline_elements());
        
        max_unbreakable_width
        // State automatically restored here via Drop trait
    }

    /// Calculate CSS max-content width with exception safety
    pub fn css_max_content_width(&mut self, font_system: &mut FontSystem) -> f32 {
        let mut guard = BufferStateGuard::new(self, font_system);
        
        // Temporarily set infinite width - state will be restored automatically
        guard.set_temporary_size(Some(f32::INFINITY), None);
        
        let text = guard.buffer().extract_text_content();
        let mut max_line_width = 0.0f32;
        
        // Performance optimization: ASCII fast-path for forced breaks
        if text.is_ascii() {
            // Fast path: simple newline splitting for ASCII text
            for line_segment in text.split('\n') {
                if !line_segment.trim().is_empty() {
                    let line_width = guard.with_buffer_and_font_system(|buffer, font_system| {
                        buffer.measure_text_sequence(font_system, line_segment)
                    });
                    max_line_width = max_line_width.max(line_width);
                }
            }
        } else {
            // Unicode path: handle complex line break rules if needed
            // For now, use same logic but could be enhanced for complex scripts
            for line_segment in text.split('\n') {
                if !line_segment.trim().is_empty() {
                    let line_width = guard.with_buffer_and_font_system(|buffer, font_system| {
                        buffer.measure_text_sequence(font_system, line_segment)
                    });
                    max_line_width = max_line_width.max(line_width);
                }
            }
        }
        
        // Handle inline elements in max-content calculation
        max_line_width = max_line_width.max(guard.buffer().measure_inline_elements());
        
        max_line_width
        // State automatically restored here via Drop trait
    }

    /// Measure individual text sequence with proper font metrics
    fn measure_text_sequence(&mut self, font_system: &mut FontSystem, text: &str) -> f32 {
        // Create temporary buffer for measurement
        let mut temp_buffer = Buffer::new(font_system, self.inner.metrics());
        temp_buffer.set_text(font_system, text, &Attrs::new(), Shaping::Advanced);
        temp_buffer.set_size(font_system, Some(f32::INFINITY), None);
        
        // Get actual measured width
        temp_buffer.layout_runs()
            .map(|run| run.line_w)
            .fold(0.0f32, f32::max)
    }
    
    /// Determine if sequence is unbreakable based on script and content
    fn is_unbreakable_sequence(&self, text: &str) -> bool {
        // CJK characters can break between most characters
        // Latin words cannot break within word boundaries
        // Complex scripts need script-specific analysis
        
        use unicode_script::UnicodeScript;
        
        let mut has_cjk = false;
        let mut has_alphabetic = false;
        let mut has_other = false;
        
        for ch in text.chars() {
            match ch.script() {
                unicode_script::Script::Han | 
                unicode_script::Script::Hiragana | 
                unicode_script::Script::Katakana => {
                    has_cjk = true;
                }
                unicode_script::Script::Latin |
                unicode_script::Script::Cyrillic |
                unicode_script::Script::Arabic => {
                    has_alphabetic = true;
                }
                _ => {
                    has_other = true;
                }
            }
        }
        
        // If any CJK characters, the sequence is breakable
        if has_cjk {
            return false;
        }
        
        // If alphabetic or other scripts (but no CJK), treat as unbreakable
        // This follows the conservative approach for non-CJK text
        has_alphabetic || has_other
    }
    
    /// Extract text content from buffer for analysis
    fn extract_text_content(&self) -> String {
        // Extract all text content from layout runs
        self.inner.layout_runs()
            .map(|run| run.text)
            .collect::<Vec<_>>()
            .join("")
    }
    
    /// Measure width contribution from inline elements
    fn measure_inline_elements(&self) -> f32 {
        // Inline elements are measured at the TextLayout level in blitz-dom
        // to maintain architectural separation. See:
        // TextLayout::calculate_content_widths_with_inline_elements()
        0.0f32  // Text-only measurement
    }

    /// Get cached layout run information
    pub fn cached_layout_runs(&self) -> &[LayoutRunInfo] {
        &self.cached_layout_runs
    }

    /// Update cached layout run information
    fn update_cached_layout_runs(&mut self) {
        self.cached_layout_runs.clear();

        for run in self.inner.layout_runs() {
            let info = LayoutRunInfo::from_layout_run(&run);
            self.cached_layout_runs.push(info);
        }
    }

    /// Get character position at coordinates
    pub fn hit_test(&self, x: f32, y: f32) -> Option<Cursor> {
        self.inner.hit(x, y)
    }

    /// Get cursor layout information
    pub fn get_cursor_layout(
        &mut self,
        font_system: &mut FontSystem,
        cursor: Cursor,
    ) -> Option<LayoutCursor> {
        self.inner.layout_cursor(font_system, cursor)
    }

    /// Move cursor with motion
    pub fn move_cursor(
        &mut self,
        font_system: &mut FontSystem,
        cursor: Cursor,
        cursor_x_opt: Option<i32>,
        motion: Motion,
    ) -> Option<(Cursor, Option<i32>)> {
        self.inner
            .cursor_motion(font_system, cursor, cursor_x_opt, motion)
    }

    /// Calculate CSS widths with comprehensive error handling
    pub fn calculate_css_widths_safe(
        &mut self,
        font_system: &mut FontSystem,
    ) -> Result<(f32, f32), CssWidthCalculationError> {
        // Validate initial state
        self.validate_state()
            .map_err(|_| CssWidthCalculationError::StateCorruption)?;
        
        let min_width = self.css_min_content_width_safe(font_system)?;
        let max_width = self.css_max_content_width_safe(font_system)?;
        
        // Validate final state
        self.validate_state()
            .map_err(|_| CssWidthCalculationError::StateCorruption)?;
        
        Ok((min_width, max_width))
    }

    /// Calculate CSS min-content width with error handling
    pub fn css_min_content_width_safe(
        &mut self,
        font_system: &mut FontSystem,
    ) -> Result<f32, CssWidthCalculationError> {
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.css_min_content_width(font_system)
        })) {
            Ok(width) => Ok(width),
            Err(_) => Err(CssWidthCalculationError::LayoutError(
                "Panic occurred during min-content width calculation".to_string()
            )),
        }
    }

    /// Calculate CSS max-content width with error handling
    pub fn css_max_content_width_safe(
        &mut self,
        font_system: &mut FontSystem,
    ) -> Result<f32, CssWidthCalculationError> {
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.css_max_content_width(font_system)
        })) {
            Ok(width) => Ok(width),
            Err(_) => Err(CssWidthCalculationError::LayoutError(
                "Panic occurred during max-content width calculation".to_string()
            )),
        }
    }

    /// Calculate CSS min-content width with performance monitoring
    pub fn css_min_content_width_monitored(
        &mut self,
        font_system: &mut FontSystem,
        metrics: &mut CssWidthMetrics,
    ) -> f32 {
        let start = std::time::Instant::now();
        metrics.calculation_count += 1;
        
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.css_min_content_width(font_system)
        }));
        
        metrics.total_duration += start.elapsed();
        
        match result {
            Ok(width) => width,
            Err(_) => {
                metrics.error_count += 1;
                // Return safe fallback value
                0.0
            }
        }
    }

    /// Calculate CSS max-content width with performance monitoring
    pub fn css_max_content_width_monitored(
        &mut self,
        font_system: &mut FontSystem,
        metrics: &mut CssWidthMetrics,
    ) -> f32 {
        let start = std::time::Instant::now();
        metrics.calculation_count += 1;
        
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.css_max_content_width(font_system)
        }));
        
        metrics.total_duration += start.elapsed();
        
        match result {
            Ok(width) => width,
            Err(_) => {
                metrics.error_count += 1;
                // Return safe fallback value
                0.0
            }
        }
    }

    /// Validate buffer state for consistency
    pub fn validate_state(&self) -> Result<(), ()> {
        // Check that buffer is in valid state
        // Verify cache consistency
        // Check that all invariants are maintained
        
        // Basic validation: check that inner buffer exists and has valid metrics
        let metrics = self.inner.metrics();
        if metrics.font_size <= 0.0 || metrics.line_height <= 0.0 {
            return Err(());
        }
        
        // Check that cached layout runs are consistent
        // This is a simplified check - could be more comprehensive
        let text = self.extract_text_content();
        if text != self.last_shaped_text && !self.cached_layout_runs.is_empty() {
            return Err(());
        }
        
        Ok(())
    }
}

/// Cached layout run information for performance optimization
#[derive(Debug, Clone)]
pub struct LayoutRunInfo {
    pub line_index: usize,
    pub text: String,
    pub rtl: bool,
    pub glyph_count: usize,
    pub line_y: f32,
    pub line_top: f32,
    pub line_height: f32,
    pub line_width: f32,
    pub glyph_infos: Vec<GlyphInfo>,
}

impl LayoutRunInfo {
    /// Extract layout run information
    pub fn from_layout_run(run: &LayoutRun) -> Self {
        let glyph_infos: Vec<GlyphInfo> = run
            .glyphs
            .iter()
            .map(GlyphInfo::from_layout_glyph)
            .collect();

        Self {
            line_index: run.line_i,
            text: run.text.to_string(),
            rtl: run.rtl,
            glyph_count: run.glyphs.len(),
            line_y: run.line_y,
            line_top: run.line_top,
            line_height: run.line_height,
            line_width: run.line_w,
            glyph_infos,
        }
    }

    /// Get highlight bounds for cursor selection
    pub fn get_highlight_bounds(
        &self,
        cursor_start: Cursor,
        cursor_end: Cursor,
    ) -> Option<(f32, f32)> {
        let mut x_start = None;
        let mut x_end = None;
        let rtl_factor = if self.rtl { 1.0 } else { 0.0 };
        let ltr_factor = 1.0 - rtl_factor;

        for glyph_info in self.glyph_infos.iter() {
            let cursor = self.cursor_from_glyph_left(glyph_info);
            if cursor >= cursor_start && cursor <= cursor_end {
                if x_start.is_none() {
                    x_start = Some(glyph_info.x + glyph_info.w * rtl_factor);
                }
                x_end = Some(glyph_info.x + glyph_info.w * rtl_factor);
            }
            let cursor = self.cursor_from_glyph_right(glyph_info);
            if cursor >= cursor_start && cursor <= cursor_end {
                if x_start.is_none() {
                    x_start = Some(glyph_info.x + glyph_info.w * ltr_factor);
                }
                x_end = Some(glyph_info.x + glyph_info.w * ltr_factor);
            }
        }

        if let Some(x_start) = x_start {
            // Use safe error handling instead of expect() to prevent runtime panics
            if let Some(x_end) = x_end {
                let (x_start, x_end) = if x_start < x_end {
                    (x_start, x_end)
                } else {
                    (x_end, x_start)
                };
                Some((x_start, x_end - x_start))
            } else {
                // If we have a start but no end, create a zero-width selection at start position
                Some((x_start, 0.0))
            }
        } else {
            None
        }
    }

    fn cursor_from_glyph_left(&self, glyph: &GlyphInfo) -> Cursor {
        if self.rtl {
            Cursor::new_with_affinity(self.line_index, glyph.end, Affinity::Before)
        } else {
            Cursor::new_with_affinity(self.line_index, glyph.start, Affinity::After)
        }
    }

    fn cursor_from_glyph_right(&self, glyph: &GlyphInfo) -> Cursor {
        if self.rtl {
            Cursor::new_with_affinity(self.line_index, glyph.start, Affinity::After)
        } else {
            Cursor::new_with_affinity(self.line_index, glyph.end, Affinity::Before)
        }
    }
}
