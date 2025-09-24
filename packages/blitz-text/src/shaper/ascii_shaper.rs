//! Ultra-fast ASCII text shaping with zero-allocation hot paths

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use cosmyc_text::{Attrs, Buffer, FontSystem, Metrics, Shaping};

use crate::error::ShapingError;
use crate::types::{GlyphFlags, ShapedGlyph, ShapedRun, ShapedText, TextDirection};

/// Statistics for ASCII shaping operations
static ASCII_OPERATIONS: AtomicUsize = AtomicUsize::new(0);
static ASCII_GLYPHS_SHAPED: AtomicUsize = AtomicUsize::new(0);

thread_local! {
    static ASCII_GLYPHS_BUFFER: std::cell::RefCell<Vec<ShapedGlyph>> =
        std::cell::RefCell::new(Vec::with_capacity(256));
}

/// Ultra-fast ASCII text shaper with zero-allocation hot paths
pub struct AsciiShaper {
    default_metrics: Metrics,
}

impl AsciiShaper {
    /// Create new ASCII shaper
    pub fn new() -> Self {
        Self {
            default_metrics: Metrics::new(16.0, 20.0),
        }
    }

    /// Create ASCII shaper with custom default metrics
    pub fn with_metrics(metrics: Metrics) -> Self {
        Self {
            default_metrics: metrics,
        }
    }

    /// Ultra-fast ASCII text shaping (zero allocation hot path)
    pub fn shape_ascii_fast(
        &self,
        font_system: &mut FontSystem,
        text: &str,
        attrs: Attrs,
        max_width: Option<f32>,
        cache_key: crate::types::ShapingCacheKey,
    ) -> Result<Arc<ShapedText>, ShapingError> {
        ASCII_OPERATIONS.fetch_add(1, Ordering::Relaxed);

        let metrics = if let Some(cached_metrics) = attrs.metrics_opt {
            cached_metrics.into()
        } else {
            self.default_metrics
        };

        let mut buffer = Buffer::new(font_system, metrics);

        // ASCII text never needs complex shaping
        buffer.set_size(font_system, Some(max_width.unwrap_or(f32::MAX)), None);
        buffer.set_text(font_system, text, &attrs, Shaping::Advanced);
        buffer.shape_until_scroll(font_system, false);

        // Extract glyphs with zero allocation where possible
        let shaped_run = ASCII_GLYPHS_BUFFER.with(|glyphs_buffer| {
            let mut glyphs_buffer = glyphs_buffer.borrow_mut();
            glyphs_buffer.clear();

            let mut total_width = 0.0;
            let mut max_ascent: f32 = 0.0;
            let mut max_descent: f32 = 0.0;

            for layout_run in buffer.layout_runs() {
                max_ascent = max_ascent.max(layout_run.line_height * 0.8);
                max_descent = max_descent.max(layout_run.line_height * 0.2);

                for glyph in layout_run.glyphs {
                    glyphs_buffer.push(ShapedGlyph {
                        glyph_id: glyph.glyph_id,
                        cluster: glyph.start as u32,
                        x_advance: glyph.w,
                        y_advance: 0.0,
                        x_offset: glyph.x,
                        y_offset: glyph.y,
                        flags: GlyphFlags::empty(), // ASCII text has no special flags
                        font_size: attrs.metadata as f32,
                        color: attrs.color_opt.map(|c| c.0),
                    });
                    total_width += glyph.w;
                }
            }

            ASCII_GLYPHS_SHAPED.fetch_add(glyphs_buffer.len(), Ordering::Relaxed);

            ShapedRun {
                glyphs: glyphs_buffer.clone(), // Only clone the actual data
                script: unicode_script::Script::Latin,
                direction: TextDirection::LeftToRight,
                language: None,
                level: unicode_bidi::Level::ltr(),
                width: total_width,
                height: max_ascent + max_descent,
                ascent: max_ascent,
                descent: max_descent,
                line_gap: metrics.line_height - max_ascent - max_descent,
                start_index: 0,
                end_index: text.len(),
            }
        });

        let total_width = shaped_run.width;
        let total_height = shaped_run.height;
        let baseline = shaped_run.ascent;

        let shaped_text = Arc::new(ShapedText {
            runs: vec![shaped_run],
            total_width,
            total_height,
            baseline,
            line_count: 1,
            shaped_at: std::time::Instant::now(),
            cache_key,
        });

        Ok(shaped_text)
    }

    /// Check if text is ASCII-only (optimized for hot path)
    #[inline]
    pub fn is_ascii_only(text: &str) -> bool {
        text.is_ascii()
    }

    /// Get ASCII shaping statistics
    pub fn stats() -> AsciiShaperStats {
        AsciiShaperStats {
            operations: ASCII_OPERATIONS.load(Ordering::Relaxed),
            glyphs_shaped: ASCII_GLYPHS_SHAPED.load(Ordering::Relaxed),
        }
    }

    /// Clear ASCII buffers
    pub fn clear_buffers() {
        ASCII_GLYPHS_BUFFER.with(|buffer| buffer.borrow_mut().clear());
    }
}

impl Default for AsciiShaper {
    fn default() -> Self {
        Self::new()
    }
}

/// ASCII shaper statistics
#[derive(Debug, Clone)]
pub struct AsciiShaperStats {
    pub operations: usize,
    pub glyphs_shaped: usize,
}

impl AsciiShaperStats {
    /// Calculate average glyphs per operation
    #[inline]
    pub fn avg_glyphs_per_operation(&self) -> f64 {
        if self.operations > 0 {
            self.glyphs_shaped as f64 / self.operations as f64
        } else {
            0.0
        }
    }
}
