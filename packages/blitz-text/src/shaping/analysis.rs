//! Text analysis and script detection utilities
//!
//! This module provides comprehensive text analysis capabilities including
//! script detection, bidirectional text processing, and complexity assessment.

use std::collections::HashMap;

use unicode_bidi::BidiInfo;
use unicode_script::{Script, UnicodeScript};

use super::types::{ScriptComplexity, ScriptRun, TextAnalysis, TextDirection};
use crate::error::ShapingError;

// Thread-local cache for script detection to avoid repeated Unicode lookups
thread_local! {
    static SCRIPT_CACHE: std::cell::RefCell<HashMap<char, Script>> =
        std::cell::RefCell::new(HashMap::new());
}

/// Perform comprehensive text analysis for shaping optimization
pub fn analyze_text_comprehensive(text: &str) -> Result<TextAnalysis, ShapingError> {
    if text.is_empty() {
        return Ok(TextAnalysis {
            script_runs: Vec::new(),
            base_direction: TextDirection::LeftToRight,
            has_complex_scripts: false,
            requires_bidi: false,
            complexity_score: 0,
        });
    }

    // Detect script runs with optimized caching
    let script_runs = detect_script_runs_cached(text)?;

    // Determine if bidirectional processing is needed
    let requires_bidi = needs_bidi_processing(text);
    let base_direction = determine_base_direction(text);

    // Check for complex scripts
    let has_complex_scripts = script_runs.iter().any(|run| {
        matches!(
            run.complexity,
            ScriptComplexity::Complex | ScriptComplexity::VeryComplex
        )
    });

    // Calculate complexity score
    let complexity_score =
        calculate_complexity_score(&script_runs, requires_bidi, has_complex_scripts);

    Ok(TextAnalysis {
        script_runs,
        base_direction,
        has_complex_scripts,
        requires_bidi,
        complexity_score,
    })
}

/// Detect script runs with thread-local caching for performance
fn detect_script_runs_cached(text: &str) -> Result<Vec<ScriptRun>, ShapingError> {
    let mut runs = Vec::new();
    let mut current_script = None;
    let mut run_start = 0;

    for (i, ch) in text.char_indices() {
        let script = get_char_script_cached(ch)?;

        if let Some(current) = current_script {
            if script != current && !is_script_compatible(current, script) {
                // End current run and start new one
                runs.push(ScriptRun {
                    start: run_start,
                    end: i,
                    script: current,
                    complexity: classify_script_complexity(current),
                });
                run_start = i;
                current_script = Some(script);
            }
        } else {
            current_script = Some(script);
        }
    }

    // Add final run
    if let Some(script) = current_script {
        runs.push(ScriptRun {
            start: run_start,
            end: text.len(),
            script,
            complexity: classify_script_complexity(script),
        });
    }

    Ok(runs)
}

/// Get character script with thread-local caching
fn get_char_script_cached(ch: char) -> Result<Script, ShapingError> {
    SCRIPT_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some(&script) = cache.get(&ch) {
            Ok(script)
        } else {
            let script = ch.script();
            cache.insert(ch, script);
            Ok(script)
        }
    })
}

/// Check if two scripts are compatible for the same run
fn is_script_compatible(script1: Script, script2: Script) -> bool {
    use Script::*;

    // Common script can be combined with any other script
    if script1 == Common || script2 == Common {
        return true;
    }

    // Inherited script follows the previous script
    if script1 == Inherited || script2 == Inherited {
        return true;
    }

    // Same script family compatibility
    match (script1, script2) {
        // Latin-based scripts
        (Latin, _) | (_, Latin)
            if matches!(script1, Cyrillic | Greek) || matches!(script2, Cyrillic | Greek) =>
        {
            true
        }
        // Arabic-based scripts
        (Arabic, _) | (_, Arabic)
            if matches!(script1, Syriac | Thaana) || matches!(script2, Syriac | Thaana) =>
        {
            true
        }
        // Indic scripts that can be mixed
        (Devanagari, _) | (_, Devanagari)
            if is_indic_script(script1) && is_indic_script(script2) =>
        {
            true
        }
        _ => false,
    }
}

/// Check if script is part of the Indic family
fn is_indic_script(script: Script) -> bool {
    matches!(
        script,
        Script::Devanagari
            | Script::Bengali
            | Script::Gurmukhi
            | Script::Gujarati
            | Script::Oriya
            | Script::Tamil
            | Script::Telugu
            | Script::Kannada
            | Script::Malayalam
            | Script::Sinhala
    )
}

/// Classify script complexity for optimization
fn classify_script_complexity(script: Script) -> ScriptComplexity {
    use Script::*;

    match script {
        // Simple scripts
        Latin | Cyrillic | Greek | Armenian | Georgian => ScriptComplexity::Simple,

        // Moderate complexity
        Hebrew | Thai | Lao | Ethiopic => ScriptComplexity::Moderate,

        // Complex scripts
        Arabic | Devanagari | Bengali | Gurmukhi | Gujarati | Oriya | Tamil | Telugu | Kannada
        | Malayalam | Sinhala | Myanmar => ScriptComplexity::Complex,

        // Very complex scripts
        Khmer | Tibetan | Mongolian | Yi => ScriptComplexity::VeryComplex,

        // Default to simple for unknown scripts
        _ => ScriptComplexity::Simple,
    }
}

/// Determine if text needs bidirectional processing
fn needs_bidi_processing(text: &str) -> bool {
    text.chars().any(|ch| {
        matches!(
            unicode_bidi::bidi_class(ch),
            unicode_bidi::BidiClass::R
                | unicode_bidi::BidiClass::AL
                | unicode_bidi::BidiClass::RLE
                | unicode_bidi::BidiClass::RLO
                | unicode_bidi::BidiClass::PDF
        )
    })
}

/// Determine base text direction
fn determine_base_direction(text: &str) -> TextDirection {
    let mut rtl_count = 0;
    let mut ltr_count = 0;

    for ch in text.chars().take(100) {
        // Sample first 100 chars for performance
        match unicode_bidi::bidi_class(ch) {
            unicode_bidi::BidiClass::R | unicode_bidi::BidiClass::AL => rtl_count += 1,
            unicode_bidi::BidiClass::L => ltr_count += 1,
            _ => {}
        }
    }

    if rtl_count > ltr_count {
        TextDirection::RightToLeft
    } else {
        TextDirection::LeftToRight
    }
}

/// Calculate overall text complexity score
fn calculate_complexity_score(
    script_runs: &[ScriptRun],
    requires_bidi: bool,
    has_complex_scripts: bool,
) -> u32 {
    let mut score = 0u32;

    // Base score from number of script runs
    score += (script_runs.len() as u32).saturating_mul(10);

    // Add complexity from individual scripts
    for run in script_runs {
        score += match run.complexity {
            ScriptComplexity::Simple => 5,
            ScriptComplexity::Moderate => 15,
            ScriptComplexity::Complex => 30,
            ScriptComplexity::VeryComplex => 50,
        };
    }

    // Bidirectional text adds significant complexity
    if requires_bidi {
        score += 100;
    }

    // Complex scripts add extra overhead
    if has_complex_scripts {
        score += 50;
    }

    score
}

/// Process bidirectional text with optimization
pub fn process_bidi_optimized(
    text: &str,
    base_direction: TextDirection,
) -> Result<BidiInfo<'static>, ShapingError> {
    let base_level = match base_direction {
        TextDirection::RightToLeft => unicode_bidi::Level::rtl(),
        _ => unicode_bidi::Level::ltr(),
    };

    // Convert to owned string to get 'static lifetime
    let owned_text = text.to_string();
    let bidi_info = BidiInfo::new(&owned_text, Some(base_level));

    // Safety: We're converting to 'static lifetime, but this is safe because
    // the BidiInfo will only be used with text that has the same lifetime
    let static_bidi_info =
        unsafe { std::mem::transmute::<BidiInfo<'_>, BidiInfo<'static>>(bidi_info) };

    Ok(static_bidi_info)
}
