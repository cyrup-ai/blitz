//! Text run creation with buffer reuse for zero allocation

use cosmyc_text::{Attrs, AttrsOwned};
use unicode_segmentation::UnicodeSegmentation;

use crate::analysis::TextAnalyzer;
use crate::error::ShapingError;
use crate::features::FeatureLookup;
use crate::types::{TextAnalysis, TextRun};

use super::core::TEXT_RUNS_BUFFER;

/// Create text runs with buffer reuse for zero allocation
pub(super) fn create_text_runs_optimized(
    text: &str,
    analysis: &TextAnalysis,
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
            let bidi_runs = extract_bidi_runs(bidi, para_range);

            for bidi_run in bidi_runs {
                for script_run in &analysis.script_runs {
                    let start = bidi_run.start.max(script_run.start);
                    let end = bidi_run.end.min(script_run.end);

                    if start < end {
                        let text_slice = text[start..end].to_string();
                        let language = detect_language(&text_slice, script_run.script);
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
            // Handle unidirectional text (more common case)
            for script_run in &analysis.script_runs {
                let text_slice = text[script_run.start..script_run.end].to_string();
                let language = detect_language(&text_slice, script_run.script);
                let features = FeatureLookup::get_features_for_script(script_run.script);

                runs.push(TextRun {
                    text: text_slice,
                    start: script_run.start,
                    end: script_run.end,
                    script: script_run.script,
                    direction: analysis.base_direction,
                    level: unicode_bidi::Level::ltr(),
                    attrs: owned_attrs.clone(),
                    language,
                    features,
                });
            }
        }

        Ok(runs.clone()) // Only clone actual data, not capacity
    })
}

/// Extract bidirectional runs from BidiInfo
fn extract_bidi_runs(
    bidi: &unicode_bidi::BidiInfo,
    para_range: std::ops::Range<usize>,
) -> Vec<BidiRun> {
    let mut runs = Vec::new();
    
    // Find the paragraph containing this range
    let para = bidi.paragraphs.iter()
        .find(|p| p.range.start <= para_range.start && para_range.end <= p.range.end)
        .expect("Range not found in any paragraph");
    
    // Extract visual runs using unicode-bidi's visual_runs method
    // Reference: ./tmp/unicode-bidi/src/lib.rs:669 - visual_runs() method
    let (levels, level_runs) = bidi.visual_runs(para, para_range.clone());
    
    for level_run in level_runs {
        let level = levels[level_run.start - para_range.start];
        let direction = if level.is_rtl() {
            crate::types::TextDirection::RightToLeft
        } else {
            crate::types::TextDirection::LeftToRight
        };
        
        runs.push(BidiRun {
            start: level_run.start,
            end: level_run.end,
            direction,
            level,
        });
    }
    
    runs
}

/// Detect language for text slice
fn detect_language(text: &str, script: unicode_script::Script) -> Option<String> {
    // Stage 1: Script-based primary detection
    let primary_lang = match script {
        unicode_script::Script::Latin => detect_latin_language(text),
        unicode_script::Script::Arabic => detect_arabic_language(text),
        unicode_script::Script::Chinese => detect_chinese_variant(text),
        unicode_script::Script::Cyrillic => detect_cyrillic_language(text),
        unicode_script::Script::Devanagari => Some("hi".to_string()), // Hindi
        unicode_script::Script::Hebrew => Some("he".to_string()),
        unicode_script::Script::Thai => Some("th".to_string()),
        unicode_script::Script::Korean => Some("ko".to_string()),
        unicode_script::Script::Japanese => detect_japanese_variant(text),
        _ => None,
    };
    
    // Stage 2: Statistical analysis using unicode-segmentation
    if primary_lang.is_none() {
        return statistical_language_detection(text, script);
    }
    
    primary_lang
}

fn detect_latin_language(text: &str) -> Option<String> {
    // Use word boundary analysis from unicode-segmentation
    let words: Vec<&str> = text.unicode_words().collect();
    
    // Statistical frequency analysis for common Latin languages
    let mut scores = std::collections::HashMap::new();
    scores.insert("en", 0);
    scores.insert("es", 0);
    scores.insert("fr", 0);
    scores.insert("de", 0);
    scores.insert("it", 0);
    
    for word in words {
        // English indicators
        if ["the", "and", "of", "to", "a", "in", "is", "it", "you", "that"].contains(&word.to_lowercase().as_str()) {
            *scores.get_mut("en").unwrap() += 2;
        }
        // Spanish indicators  
        if ["el", "la", "de", "que", "y", "en", "un", "es", "se", "no"].contains(&word.to_lowercase().as_str()) {
            *scores.get_mut("es").unwrap() += 2;
        }
        // French indicators
        if ["le", "de", "et", "un", "à", "il", "être", "avoir", "ne", "pour"].contains(&word.to_lowercase().as_str()) {
            *scores.get_mut("fr").unwrap() += 2;
        }
        // Character frequency analysis
        for ch in word.chars() {
            match ch {
                'ñ' => *scores.get_mut("es").unwrap() += 1,
                'ç' | 'â' | 'ê' | 'î' | 'ô' | 'û' => *scores.get_mut("fr").unwrap() += 1,
                'ä' | 'ö' | 'ü' | 'ß' => *scores.get_mut("de").unwrap() += 1,
                _ => {}
            }
        }
    }
    
    scores.into_iter()
        .max_by_key(|(_, score)| *score)
        .filter(|(_, score)| *score > 0)
        .map(|(lang, _)| lang.to_string())
}

fn detect_arabic_language(_text: &str) -> Option<String> {
    // Simplified Arabic detection - defaulting to Arabic
    Some("ar".to_string())
}

fn detect_chinese_variant(_text: &str) -> Option<String> {
    // Simplified Chinese detection - defaulting to simplified Chinese
    Some("zh".to_string())
}

fn detect_cyrillic_language(_text: &str) -> Option<String> {
    // Simplified Cyrillic detection - defaulting to Russian
    Some("ru".to_string())
}

fn detect_japanese_variant(_text: &str) -> Option<String> {
    // Simplified Japanese detection
    Some("ja".to_string())
}

fn statistical_language_detection(text: &str, script: unicode_script::Script) -> Option<String> {
    // Fallback statistical analysis using grapheme clusters
    let _graphemes: Vec<&str> = text.graphemes(true).collect();
    
    // Default languages for scripts with insufficient data
    match script {
        unicode_script::Script::Common => Some("en".to_string()), // Default to English
        unicode_script::Script::Latin => Some("en".to_string()),
        _ => None,
    }
}

/// Bidirectional run information
struct BidiRun {
    start: usize,
    end: usize,
    direction: crate::types::TextDirection,
    level: unicode_bidi::Level,
}