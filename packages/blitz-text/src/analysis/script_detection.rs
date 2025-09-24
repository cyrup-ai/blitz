//! Script detection and analysis for text shaping optimization
//!
//! This module provides ultra-fast script detection with thread-local caching
//! and optimized script run segmentation.

use unicode_script::Script;

use super::caching::CacheManager;
use crate::error::ShapingError;
use crate::types::{ScriptComplexity, ScriptRun};

/// Script detection engine with performance optimizations
pub struct ScriptDetector;

impl ScriptDetector {
    /// Ultra-fast script detection with thread-local caching and buffer reuse
    pub fn detect_script_runs_optimized(text: &str) -> Result<Vec<ScriptRun>, ShapingError> {
        CacheManager::with_script_run_buffer(|buffer| {
            let mut current_script = None;
            let mut run_start = 0;

            for (byte_pos, ch) in text.char_indices() {
                let script = CacheManager::get_script_cached(ch);

                // Check for script boundary with optimized comparison
                if let Some(prev_script) = current_script {
                    if !Self::scripts_compatible_fast(prev_script, script) {
                        // Finalize previous run
                        buffer.push(ScriptRun {
                            start: run_start,
                            end: byte_pos,
                            script: prev_script,
                            complexity: ScriptComplexity::from_script(prev_script),
                        });
                        run_start = byte_pos;
                    }
                }

                current_script = Some(script);
            }

            // Finalize last run
            if let Some(script) = current_script {
                buffer.push(ScriptRun {
                    start: run_start,
                    end: text.len(),
                    script,
                    complexity: ScriptComplexity::from_script(script),
                });
            }

            // Clone the buffer contents (only actual data, not capacity)
            Ok(buffer.clone())
        })
    }

    /// Ultra-fast script compatibility check (compile-time optimized)
    #[inline]
    fn scripts_compatible_fast(script1: Script, script2: Script) -> bool {
        match (script1, script2) {
            // Exact match (most common case) - use equality instead of ptr comparison
            _ if script1 == script2 => true,

            // Common script compatibility (compile-time constants)
            (Script::Common, _) | (_, Script::Common) => true,
            (Script::Inherited, _) | (_, Script::Inherited) => true,
            (Script::Hiragana, Script::Katakana) | (Script::Katakana, Script::Hiragana) => true,
            (Script::Han, Script::Hiragana) | (Script::Hiragana, Script::Han) => true,
            (Script::Han, Script::Katakana) | (Script::Katakana, Script::Han) => true,

            _ => false,
        }
    }

    /// Fast complex scripts check with compile-time optimization
    #[inline]
    pub const fn has_complex_scripts_fast(runs: &[ScriptRun]) -> bool {
        let mut i = 0;
        while i < runs.len() {
            if matches!(
                runs[i].complexity,
                ScriptComplexity::Complex | ScriptComplexity::VeryComplex
            ) {
                return true;
            }
            i += 1;
        }
        false
    }

    /// Fast complexity score calculation with optimized loop
    #[inline]
    pub fn calculate_complexity_score_fast(runs: &[ScriptRun], requires_bidi: bool) -> u32 {
        let mut score = 0;
        let mut i = 0;

        while i < runs.len() {
            score += runs[i].complexity.cache_priority();
            i += 1;
        }

        if requires_bidi {
            score += 5;
        }

        // Penalty for script changes (complexity increases with fragmentation)
        if runs.len() > 3 {
            score += (runs.len() as u32 - 3) * 2;
        }

        score
    }

    /// Language detection based on script with compile-time optimization
    #[inline]
    pub const fn detect_language(_text: &str, script: Script) -> Option<&'static str> {
        // Primary language detection based on script (compile-time constants)
        match script {
            Script::Arabic => Some("ar"),
            Script::Hebrew => Some("he"),
            Script::Devanagari => Some("hi"),
            Script::Bengali => Some("bn"),
            Script::Tamil => Some("ta"),
            Script::Telugu => Some("te"),
            Script::Kannada => Some("kn"),
            Script::Malayalam => Some("ml"),
            Script::Thai => Some("th"),
            Script::Lao => Some("lo"),
            Script::Myanmar => Some("my"),
            Script::Khmer => Some("km"),
            Script::Greek => Some("el"),
            Script::Cyrillic => Some("ru"), // Default to Russian
            Script::Han => Some("zh"),      // Default to Chinese
            Script::Hiragana | Script::Katakana => Some("ja"),
            Script::Hangul => Some("ko"),
            Script::Tibetan => Some("bo"),
            _ => None,
        }
    }

    /// Check if text contains only ASCII characters (ultra-fast path)
    #[inline]
    pub const fn is_ascii_only(text: &str) -> bool {
        let bytes = text.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] >= 128 {
                return false;
            }
            i += 1;
        }
        true
    }

    /// Check if text contains only Latin script (fast path)
    pub fn is_latin_only(text: &str) -> bool {
        if Self::is_ascii_only(text) {
            return true;
        }

        text.chars().all(|ch| {
            let script = CacheManager::get_script_cached(ch);
            matches!(script, Script::Latin | Script::Common | Script::Inherited)
        })
    }

    /// Determine if text needs complex processing (optimization hint)
    pub fn needs_complex_processing(text: &str) -> bool {
        // ASCII text never needs complex processing
        if Self::is_ascii_only(text) {
            return false;
        }

        // Check for complex scripts
        text.chars().any(|ch| {
            let script = CacheManager::get_script_cached(ch);
            crate::features::FeatureLookup::requires_complex_shaping(script)
        })
    }
}
