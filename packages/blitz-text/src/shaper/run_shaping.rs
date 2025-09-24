//! Complex script shaping with bidirectional text support

use std::sync::atomic::{AtomicUsize, Ordering};

use cosmyc_text::{Attrs, AttrsOwned, Buffer, FontSystem, Metrics, Shaping};

use super::glyph_analysis::GlyphAnalyzer;
use crate::analysis::TextAnalyzer;
use crate::error::ShapingError;
use crate::features::FeatureLookup;
use crate::shaping::types::{ShapedGlyph, ShapedRun, TextDirection as ShapingTextDirection};
use crate::types::{TextDirection, TextRun};

/// Statistics for run shaping operations
static RUN_SHAPING_OPERATIONS: AtomicUsize = AtomicUsize::new(0);
static COMPLEX_RUNS_SHAPED: AtomicUsize = AtomicUsize::new(0);

thread_local! {
    static SHAPED_RUNS_BUFFER: std::cell::RefCell<Vec<ShapedRun>> =
        std::cell::RefCell::new(Vec::with_capacity(16));
    static GLYPHS_BUFFER: std::cell::RefCell<Vec<ShapedGlyph>> =
        std::cell::RefCell::new(Vec::with_capacity(256));
    static TEXT_RUNS_BUFFER: std::cell::RefCell<Vec<TextRun>> =
        std::cell::RefCell::new(Vec::with_capacity(8));
}

/// Complex script shaper with bidirectional text support
pub struct RunShaper {
    analyzer: TextAnalyzer,
}

impl RunShaper {
    /// Create new run shaper
    pub fn new() -> Self {
        Self {
            analyzer: TextAnalyzer::new(),
        }
    }

    /// Create text runs with buffer reuse for zero allocation
    pub fn create_text_runs_optimized(
        &self,
        text: &str,
        analysis: &crate::types::TextAnalysis,
        bidi_info: Option<&unicode_bidi::BidiInfo>,
        attrs: Attrs,
    ) -> Result<Vec<TextRun>, ShapingError> {
        let owned_attrs = AttrsOwned::new(&attrs); // Convert to owned attrs
        TEXT_RUNS_BUFFER.with(|buffer| {
            let mut runs = buffer.borrow_mut();
            runs.clear();

            if let Some(bidi) = bidi_info {
                // Handle bidirectional text
                let para_range = 0..text.len();
                let bidi_runs = self.analyzer.extract_bidi_runs(bidi, para_range);

                for bidi_run in bidi_runs {
                    for script_run in &analysis.script_runs {
                        let start = bidi_run.start.max(script_run.start);
                        let end = bidi_run.end.min(script_run.end);

                        if start < end {
                            let text_slice = text[start..end].to_string();
                            let language = self
                                .analyzer
                                .detect_language(&text_slice, script_run.script);
                            let features =
                                FeatureLookup::get_features_for_script(script_run.script);

                            runs.push(TextRun {
                                text: text_slice,
                                start,
                                end,
                                script: script_run.script,
                                direction: bidi_run.direction,
                                level: bidi_run.level,
                                attrs: owned_attrs.clone(),
                                language,
                                features,
                            });
                        }
                    }
                }
            } else {
                // Handle left-to-right text
                for script_run in &analysis.script_runs {
                    let text_slice = text[script_run.start..script_run.end].to_string();
                    let language = self
                        .analyzer
                        .detect_language(&text_slice, script_run.script);
                    let features = FeatureLookup::get_features_for_script(script_run.script);

                    runs.push(TextRun {
                        text: text_slice,
                        start: script_run.start,
                        end: script_run.end,
                        script: script_run.script,
                        direction: TextDirection::LeftToRight,
                        level: unicode_bidi::Level::ltr(),
                        attrs: owned_attrs.clone(),
                        language,
                        features,
                    });
                }
            }

            Ok(runs.clone()) // Return cloned runs for external use
        })
    }

    /// Shape text runs with zero allocation (reuse thread-local buffers)
    pub fn shape_runs_optimized(
        &self,
        font_system: &mut FontSystem,
        text_runs: Vec<TextRun>,
    ) -> Result<Vec<ShapedRun>, ShapingError> {
        RUN_SHAPING_OPERATIONS.fetch_add(1, Ordering::Relaxed);

        SHAPED_RUNS_BUFFER.with(|runs_buffer| {
            let mut shaped_runs = runs_buffer.borrow_mut();
            shaped_runs.clear();

            for run in text_runs {
                let shaped_run = self.shape_single_run_optimized(font_system, run)?;
                shaped_runs.push(shaped_run);
            }

            COMPLEX_RUNS_SHAPED.fetch_add(shaped_runs.len(), Ordering::Relaxed);
            Ok(shaped_runs.clone())
        })
    }

    /// Shape single text run with comprehensive analysis
    fn shape_single_run_optimized(
        &self,
        font_system: &mut FontSystem,
        run: TextRun,
    ) -> Result<ShapedRun, ShapingError> {
        let metrics = if let Some(cached_metrics) = run.attrs.as_attrs().metrics_opt {
            cached_metrics.into()
        } else {
            Metrics::new(16.0, 20.0) // Default metrics
        };

        let mut buffer = Buffer::new(font_system, metrics);

        // Set shaping direction based on script and bidi level
        let shaping_mode = if run.script.is_complex() {
            Shaping::Advanced
        } else {
            Shaping::Basic
        };

        buffer.set_text(font_system, &run.text, &run.attrs.as_attrs(), shaping_mode);
        buffer.shape_until_scroll(font_system, false);

        // Extract glyphs with optimized allocation
        GLYPHS_BUFFER.with(|glyphs_buffer| {
            let mut glyphs = glyphs_buffer.borrow_mut();
            glyphs.clear();

            let mut total_width: f32 = 0.0;
            let mut max_ascent: f32 = 0.0;
            let mut max_descent: f32 = 0.0;

            for layout_run in buffer.layout_runs() {
                max_ascent = max_ascent.max(layout_run.line_height * 0.8);
                max_descent = max_descent.max(layout_run.line_height * 0.2);

                let font_size = run.attrs.as_attrs().metadata as f32;

                for glyph in layout_run.glyphs {
                    glyphs.push(ShapedGlyph {
                        glyph_id: glyph.glyph_id,
                        cluster: glyph.start as u32,
                        x_advance: glyph.w,
                        y_advance: 0.0,
                        x_offset: glyph.x,
                        y_offset: glyph.y,
                        flags: GlyphAnalyzer::determine_glyph_flags_fast(&glyph, &layout_run),
                        font_size,
                        color: run.attrs.as_attrs().color_opt.map(|c| c.0),
                    });
                    total_width += glyph.w;
                }
            }

            Ok(ShapedRun {
                glyphs: glyphs.clone(),
                script: run.script,
                direction: convert_text_direction(run.direction),
                language: run.language.map(|s| s.to_string()),
                level: run.level,
                width: total_width,
                height: max_ascent + max_descent,
                ascent: max_ascent,
                descent: max_descent,
                line_gap: metrics.line_height - max_ascent - max_descent,
                start_index: run.start,
                end_index: run.end,
            })
        })
    }

    /// Get analyzer reference for external use
    pub fn analyzer(&self) -> &TextAnalyzer {
        &self.analyzer
    }

    /// Get run shaping statistics
    pub fn stats() -> RunShapingStats {
        RunShapingStats {
            operations: RUN_SHAPING_OPERATIONS.load(Ordering::Relaxed),
            complex_runs_shaped: COMPLEX_RUNS_SHAPED.load(Ordering::Relaxed),
        }
    }

    /// Clear run shaping buffers
    pub fn clear_buffers() {
        SHAPED_RUNS_BUFFER.with(|buffer| buffer.borrow_mut().clear());
        GLYPHS_BUFFER.with(|buffer| buffer.borrow_mut().clear());
        TEXT_RUNS_BUFFER.with(|buffer| buffer.borrow_mut().clear());
    }

    /// Clear analyzer caches
    pub fn clear_caches(&mut self) {
        self.analyzer.clear_caches();
    }
}

impl Default for RunShaper {
    fn default() -> Self {
        Self::new()
    }
}

/// Run shaping statistics
#[derive(Debug, Clone)]
pub struct RunShapingStats {
    pub operations: usize,
    pub complex_runs_shaped: usize,
}

impl RunShapingStats {
    /// Calculate average runs per operation
    #[inline]
    pub fn avg_runs_per_operation(&self) -> f64 {
        if self.operations > 0 {
            self.complex_runs_shaped as f64 / self.operations as f64
        } else {
            0.0
        }
    }
}

/// Convert types::TextDirection to shaping::types::TextDirection
fn convert_text_direction(direction: TextDirection) -> ShapingTextDirection {
    match direction {
        TextDirection::LeftToRight => ShapingTextDirection::LeftToRight,
        TextDirection::RightToLeft => ShapingTextDirection::RightToLeft,
        TextDirection::TopToBottom => ShapingTextDirection::TopToBottom,
        TextDirection::BottomToTop => ShapingTextDirection::BottomToTop,
    }
}

/// Helper trait for script complexity detection
trait ScriptComplexity {
    fn is_complex(&self) -> bool;
}

impl ScriptComplexity for unicode_script::Script {
    fn is_complex(&self) -> bool {
        matches!(
            self,
            unicode_script::Script::Arabic
                | unicode_script::Script::Hebrew
                | unicode_script::Script::Devanagari
                | unicode_script::Script::Bengali
                | unicode_script::Script::Gujarati
                | unicode_script::Script::Gurmukhi
                | unicode_script::Script::Kannada
                | unicode_script::Script::Malayalam
                | unicode_script::Script::Oriya
                | unicode_script::Script::Tamil
                | unicode_script::Script::Telugu
                | unicode_script::Script::Thai
                | unicode_script::Script::Lao
                | unicode_script::Script::Myanmar
                | unicode_script::Script::Khmer
        )
    }
}
