//! Core UnifiedTextSystem struct definition and constructors
//!
//! This module contains the main structure definition for the unified text system
//! and its initialization logic that coordinates all subsystems.

use std::cell::RefCell;
use std::time::Instant;

use cosmyc_text::{Attrs, FontSystem};
use glyphon::CustomGlyph;
use thread_local::ThreadLocal;
use wgpu::{DepthStencilState, Device, MultisampleState, Queue, TextureFormat};

use super::super::config::{TextSystemError, TextSystemResult, UnifiedTextConfig};
use super::super::performance::SystemPerformanceMonitor;
use crate::cosmyc::CosmicTextIntegration;
use crate::custom_glyphs::{CustomGlyphSystem, GlyphSystemConfig};
use crate::gpu::{EnhancedGpuCache, EnhancedTextAtlas, EnhancedTextRenderer, EnhancedViewport};
use crate::measurement::EnhancedTextMeasurer;
use crate::measurement::TextMeasurement;

/// Comprehensive unified text system combining measurement and GPU rendering
pub struct UnifiedTextSystem {
    // Core systems - lock-free per-thread FontSystem instances
    pub(super) font_system: ThreadLocal<RefCell<FontSystem>>,

    // Measurement components
    pub(super) text_measurer: EnhancedTextMeasurer,

    // Cosmic-text integration
    pub(super) cosmyc_integration: CosmicTextIntegration,

    // GPU rendering components
    pub(super) text_renderer: EnhancedTextRenderer,
    pub(super) text_atlas: EnhancedTextAtlas,
    pub(super) viewport: EnhancedViewport,
    pub(super) gpu_cache: EnhancedGpuCache,

    // Custom glyph system
    pub(super) custom_glyph_system: CustomGlyphSystem,

    // System state
    pub(super) config: UnifiedTextConfig,
    pub(super) performance_monitor: SystemPerformanceMonitor,

    // Statistics
    pub(super) stats_start_time: Instant,
}

impl UnifiedTextSystem {
    /// Measure text with given attributes and constraints
    pub fn measure_text(
        &mut self,
        text: &str,
        attrs: Attrs,
        max_width: Option<f32>,
        _max_height: Option<f32>,
    ) -> TextSystemResult<TextMeasurement> {
        let request = crate::measurement::types::measurement_request::MeasurementRequest {
            text: text.to_string(),
            font_id: 0, // Default font ID
            font_size: attrs
                .metrics_opt
                .map(|m| cosmyc_text::Metrics::from(m).font_size)
                .unwrap_or(14.0),
            max_width,
            enable_shaping: true,
            language: None,
            direction: None,
        };
        self.text_measurer
            .measure_text(&request)
            .map_err(|e| TextSystemError::Measurement(e))
    }

    /// Get custom glyphs for a text range
    pub fn get_custom_glyphs_for_text_range(
        &mut self,
        text: &str,
        range: std::ops::Range<usize>,
    ) -> TextSystemResult<Vec<CustomGlyph>> {
        self.custom_glyph_system
            .get_glyphs_for_range(text, range)
            .map_err(TextSystemError::CustomGlyph)
    }

    /// Create a new unified text system
    pub fn new(
        device: &Device,
        queue: &Queue,
        format: TextureFormat,
        multisample: MultisampleState,
        depth_stencil: Option<DepthStencilState>,
    ) -> TextSystemResult<Self> {
        let font_system = ThreadLocal::new();

        // Create measurement components
        let text_measurer = EnhancedTextMeasurer::new()?;

        // Create cosmyc-text integration
        let cosmyc_integration = CosmicTextIntegration::new();

        // Create GPU components
        let gpu_cache = EnhancedGpuCache::new(device)?;
        let glyphon_cache = gpu_cache.glyphon_cache();

        let mut text_atlas = EnhancedTextAtlas::new(device, queue, glyphon_cache, format);
        let text_renderer = EnhancedTextRenderer::new(
            text_atlas.inner_mut(),
            device,
            multisample,
            depth_stencil
        );

        let viewport = EnhancedViewport::new(device, glyphon_cache);

        // Create performance monitor
        let performance_monitor = SystemPerformanceMonitor::new();

        // Create custom glyph system
        let custom_glyph_system = CustomGlyphSystem::new(GlyphSystemConfig::default());

        Ok(Self {
            font_system,
            text_measurer,
            cosmyc_integration,
            text_renderer,
            text_atlas,
            viewport,
            gpu_cache,
            custom_glyph_system,
            config: UnifiedTextConfig::default(),
            performance_monitor,
            stats_start_time: Instant::now(),
        })
    }

    /// Create a new unified text system with custom configuration
    pub fn with_config(
        device: &Device,
        queue: &Queue,
        format: TextureFormat,
        multisample: MultisampleState,
        depth_stencil: Option<DepthStencilState>,
        config: UnifiedTextConfig,
    ) -> TextSystemResult<Self> {
        let mut system = Self::new(device, queue, format, multisample, depth_stencil)?;
        system.config = config;
        Ok(system)
    }



    /// Get reference to thread-local font system (lock-free access)
    /// Each thread gets its own FontSystem instance for zero contention
    pub fn with_font_system<T>(&self, f: impl FnOnce(&mut FontSystem) -> T) -> T {
        let font_system_cell = self.font_system.get_or(|| RefCell::new(FontSystem::new()));
        let mut font_system = font_system_cell.borrow_mut();
        f(&mut *font_system)
    }

    /// Get current configuration
    pub fn config(&self) -> &UnifiedTextConfig {
        &self.config
    }
}
