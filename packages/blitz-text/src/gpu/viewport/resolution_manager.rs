//! Resolution management and analysis for viewport operations
//!
//! This module handles resolution validation, tracking, pattern analysis,
//! and optimal resolution prediction based on usage patterns.

use std::collections::HashMap;
use std::time::Instant;

use glyphon::Resolution;

use super::types::{OptimalResolutionPrediction, ResolutionAnalysis, ResolutionEvent};
use crate::gpu::{GpuTextError, GpuTextResult};

/// Resolution manager for tracking and analyzing resolution changes
pub struct ResolutionManager {
    /// Current resolution
    current_resolution: Resolution,
    /// History of resolution changes
    resolution_history: Vec<ResolutionEvent>,
    /// Maximum history size
    max_history_size: usize,
}

impl ResolutionManager {
    /// Create new resolution manager
    pub fn new() -> Self {
        Self {
            current_resolution: Resolution {
                width: 0,
                height: 0,
            },
            resolution_history: Vec::new(),
            max_history_size: 100,
        }
    }

    /// Create resolution manager with custom history size
    pub fn with_history_size(max_history_size: usize) -> Self {
        Self {
            current_resolution: Resolution {
                width: 0,
                height: 0,
            },
            resolution_history: Vec::new(),
            max_history_size,
        }
    }

    /// Update resolution and track changes
    pub fn update_resolution(
        &mut self,
        new_resolution: Resolution,
        timestamp: Instant,
    ) -> GpuTextResult<bool> {
        // Validate the new resolution
        self.validate_resolution(new_resolution)?;

        // Check if resolution actually changed
        let resolution_changed = self.current_resolution.width != new_resolution.width
            || self.current_resolution.height != new_resolution.height;

        if resolution_changed {
            // Create resolution event
            let event = ResolutionEvent {
                timestamp,
                old_resolution: self.current_resolution,
                new_resolution,
                duration_since_last_change: self.get_duration_since_last_change(timestamp),
            };

            // Add to history
            self.resolution_history.push(event);

            // Maintain history size limit
            if self.resolution_history.len() > self.max_history_size {
                let drain_end = self.resolution_history.len() - self.max_history_size;
                self.resolution_history.drain(0..drain_end);
            }

            // Update current resolution
            self.current_resolution = new_resolution;
        }

        Ok(resolution_changed)
    }

    /// Validate resolution parameters
    pub fn validate_resolution(&self, resolution: Resolution) -> GpuTextResult<()> {
        // Check for reasonable resolution bounds
        if resolution.width == 0 || resolution.height == 0 {
            return Err(GpuTextError::InvalidTextArea(format!(
                "Invalid resolution: {}x{} (dimensions must be > 0)",
                resolution.width, resolution.height
            )));
        }

        // Check for unreasonably large resolutions
        const MAX_DIMENSION: u32 = 16384; // 16K resolution limit
        if resolution.width > MAX_DIMENSION || resolution.height > MAX_DIMENSION {
            return Err(GpuTextError::InvalidTextArea(format!(
                "Resolution too large: {}x{} (max {}x{})",
                resolution.width, resolution.height, MAX_DIMENSION, MAX_DIMENSION
            )));
        }

        // Check for texture memory limits (estimate)
        let estimated_memory = (resolution.width as u64) * (resolution.height as u64) * 4; // 4 bytes per pixel
        const MAX_TEXTURE_MEMORY: u64 = 512 * 1024 * 1024; // 512MB limit
        if estimated_memory > MAX_TEXTURE_MEMORY {
            return Err(GpuTextError::InvalidTextArea(format!(
                "Resolution requires too much memory: {}x{} ({} MB)",
                resolution.width,
                resolution.height,
                estimated_memory / (1024 * 1024)
            )));
        }

        Ok(())
    }

    /// Get current resolution
    pub fn current_resolution(&self) -> Resolution {
        self.current_resolution
    }

    /// Get resolution change history
    pub fn get_resolution_history(&self) -> &[ResolutionEvent] {
        &self.resolution_history
    }

    /// Get total number of resolution changes
    pub fn total_resolution_changes(&self) -> usize {
        self.resolution_history.len()
    }

    /// Analyze resolution patterns
    pub fn analyze_resolution_patterns(&self) -> ResolutionAnalysis {
        if self.resolution_history.is_empty() {
            return ResolutionAnalysis {
                most_common_resolution: self.current_resolution,
                avg_resolution: self.current_resolution,
                resolution_stability: 1.0,
                change_frequency_hz: 0.0,
                aspect_ratio_consistency: 1.0,
            };
        }

        // Calculate most common resolution
        let mut resolution_counts = HashMap::new();
        for event in &self.resolution_history {
            *resolution_counts.entry(event.new_resolution).or_insert(0) += 1;
        }
        let most_common_resolution = resolution_counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(res, _)| res)
            .unwrap_or(self.current_resolution);

        // Calculate average resolution
        let total_width: u64 = self
            .resolution_history
            .iter()
            .map(|e| e.new_resolution.width as u64)
            .sum();
        let total_height: u64 = self
            .resolution_history
            .iter()
            .map(|e| e.new_resolution.height as u64)
            .sum();
        let count = self.resolution_history.len() as u64;
        let avg_resolution = Resolution {
            width: (total_width / count) as u32,
            height: (total_height / count) as u32,
        };

        // Calculate stability (lower change rate = higher stability)
        let change_frequency = match (
            self.resolution_history.first(),
            self.resolution_history.last(),
        ) {
            (Some(first), Some(last)) => {
                let total_time = last.timestamp.duration_since(first.timestamp);
                if total_time > std::time::Duration::from_secs(1) {
                    self.resolution_history.len() as f64 / total_time.as_secs_f64()
                } else {
                    0.0
                }
            }
            _ => 0.0,
        };
        let resolution_stability = 1.0 / (1.0 + change_frequency);

        // Calculate aspect ratio consistency
        let aspect_ratios: Vec<f64> = self
            .resolution_history
            .iter()
            .map(|e| e.new_resolution.width as f64 / e.new_resolution.height as f64)
            .collect();
        let avg_aspect_ratio = aspect_ratios.iter().sum::<f64>() / aspect_ratios.len() as f64;
        let aspect_ratio_variance = aspect_ratios
            .iter()
            .map(|ar| (ar - avg_aspect_ratio).powi(2))
            .sum::<f64>()
            / aspect_ratios.len() as f64;
        let aspect_ratio_consistency = 1.0 / (1.0 + aspect_ratio_variance);

        ResolutionAnalysis {
            most_common_resolution,
            avg_resolution,
            resolution_stability,
            change_frequency_hz: change_frequency,
            aspect_ratio_consistency,
        }
    }

    /// Predict optimal resolution based on usage patterns
    pub fn predict_optimal_resolution(&self) -> OptimalResolutionPrediction {
        let analysis = self.analyze_resolution_patterns();

        // Use most common resolution as base prediction
        let mut predicted_resolution = analysis.most_common_resolution;

        // Adjust based on stability and patterns
        if analysis.resolution_stability < 0.5 {
            // High instability - use average resolution
            predicted_resolution = analysis.avg_resolution;
        }

        // Round to common display resolutions
        predicted_resolution = self.round_to_standard_resolution(predicted_resolution);

        // Calculate confidence based on stability and consistency
        let confidence = (analysis.resolution_stability + analysis.aspect_ratio_consistency) / 2.0;

        OptimalResolutionPrediction {
            predicted_resolution,
            confidence,
            reasoning: if analysis.resolution_stability > 0.8 {
                "High stability - using most common resolution".to_string()
            } else if analysis.change_frequency_hz > 10.0 {
                "High change frequency - using averaged resolution".to_string()
            } else {
                "Moderate usage - using most common resolution".to_string()
            },
        }
    }

    /// Round resolution to common standard resolutions
    fn round_to_standard_resolution(&self, resolution: Resolution) -> Resolution {
        const STANDARD_RESOLUTIONS: &[(u32, u32)] = &[
            (1920, 1080), // 1080p
            (2560, 1440), // 1440p
            (3840, 2160), // 4K
            (1680, 1050), // WSXGA+
            (1920, 1200), // WUXGA
            (2560, 1600), // WQXGA
            (1366, 768),  // HD
            (1280, 720),  // 720p
        ];

        let target_aspect_ratio = resolution.width as f64 / resolution.height as f64;

        // Find closest standard resolution by aspect ratio and total pixels
        let mut best_match = resolution;
        let mut best_score = f64::MAX;

        for &(width, height) in STANDARD_RESOLUTIONS {
            let standard_aspect_ratio = width as f64 / height as f64;
            let aspect_ratio_diff = (target_aspect_ratio - standard_aspect_ratio).abs();

            let target_pixels = (resolution.width as u64) * (resolution.height as u64);
            let standard_pixels = (width as u64) * (height as u64);
            let pixel_diff =
                (target_pixels as f64 - standard_pixels as f64).abs() / target_pixels as f64;

            let score = aspect_ratio_diff + pixel_diff;

            if score < best_score {
                best_score = score;
                best_match = Resolution { width, height };
            }
        }

        // Only use standard resolution if it's reasonably close
        if best_score < 0.2 {
            best_match
        } else {
            resolution
        }
    }

    /// Get duration since last resolution change
    fn get_duration_since_last_change(&self, current_time: Instant) -> std::time::Duration {
        self.resolution_history
            .last()
            .map(|event| current_time.duration_since(event.timestamp))
            .unwrap_or_else(|| std::time::Duration::from_secs(0))
    }

    /// Clear resolution history
    pub fn clear_history(&mut self) {
        self.resolution_history.clear();
    }
}

impl Default for ResolutionManager {
    fn default() -> Self {
        Self::new()
    }
}
