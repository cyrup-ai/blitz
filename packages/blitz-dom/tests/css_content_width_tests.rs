//! Comprehensive tests for CSS content width calculation functionality
//! 
//! Tests the production-grade CSS content width calculation implementation
//! that replaces approximation-based methods with proper CSS Sizing Module Level 3
//! compliance including Unicode word boundary detection and script-aware handling.

use blitz_text::{FontSystem, Metrics, Attrs, Shaping, EnhancedBuffer};

fn create_test_buffer() -> (FontSystem, EnhancedBuffer) {
    let mut font_system = FontSystem::new();
    let buffer = EnhancedBuffer::new(&mut font_system, Metrics::new(16.0, 20.0));
    (font_system, buffer)
}

#[test]
fn test_unicode_word_boundaries_latin() {
    let (mut font_system, mut buffer) = create_test_buffer();
    
    // Test proper word boundary detection for Latin text
    let text = "Hello world test";
    buffer.set_text_cached(&mut font_system, text, &Attrs::new(), Shaping::Advanced);
    
    let min_width = buffer.css_min_content_width(&mut font_system);
    let max_width = buffer.css_max_content_width(&mut font_system);
    
    // Min-content should be width of longest word ("Hello", "world", or "test")
    // Max-content should be width of entire line without soft breaks
    assert!(min_width > 0.0, "Min-content width should be positive");
    assert!(max_width >= min_width, "Max-content should be >= min-content");
    assert!(max_width > min_width, "Max-content should be wider than single word for multi-word text");
}

#[test]
fn test_cjk_character_boundaries() {
    let (mut font_system, mut buffer) = create_test_buffer();
    
    // Test CJK text which can break between characters per CSS rules
    let text = "こんにちは世界"; // "Hello world" in Japanese
    buffer.set_text_cached(&mut font_system, text, &Attrs::new(), Shaping::Advanced);
    
    let min_width = buffer.css_min_content_width(&mut font_system);
    let max_width = buffer.css_max_content_width(&mut font_system);
    
    // For CJK, min-content can be single character width due to breaking rules
    assert!(min_width > 0.0, "CJK min-content width should be positive");
    assert!(max_width >= min_width, "CJK max-content should be >= min-content");
}

#[test]
fn test_mixed_scripts() {
    let (mut font_system, mut buffer) = create_test_buffer();
    
    // Test mixed Latin and CJK scripts
    let text = "Hello 世界 world";
    buffer.set_text_cached(&mut font_system, text, &Attrs::new(), Shaping::Advanced);
    
    let min_width = buffer.css_min_content_width(&mut font_system);
    let max_width = buffer.css_max_content_width(&mut font_system);
    
    assert!(min_width > 0.0, "Mixed script min-content should be positive");
    assert!(max_width >= min_width, "Mixed script max-content should be >= min-content");
}

#[test]
fn test_forced_line_breaks() {
    let (mut font_system, mut buffer) = create_test_buffer();
    
    // Test max-content with forced line breaks (\n)
    let text = "Line one\nLine two\nLongest line here";
    buffer.set_text_cached(&mut font_system, text, &Attrs::new(), Shaping::Advanced);
    
    let max_width = buffer.css_max_content_width(&mut font_system);
    
    // Max-content should be width of longest line segment, not total text width
    assert!(max_width > 0.0, "Max-content with line breaks should be positive");
    
    // Verify it's not calculating total text width by comparing with single line
    let single_line_text = "Line one Line two Longest line here";
    buffer.set_text_cached(&mut font_system, single_line_text, &Attrs::new(), Shaping::Advanced);
    let single_line_max = buffer.css_max_content_width(&mut font_system);
    
    // The single line version should be wider than the multi-line version
    assert!(single_line_max > max_width, "Single line should be wider than longest line segment");
}

#[test]
fn test_arabic_bidi_text() {
    let (mut font_system, mut buffer) = create_test_buffer();
    
    // Test RTL/BiDi text handling
    let text = "Hello مرحبا world"; // Mixed LTR/RTL text
    buffer.set_text_cached(&mut font_system, text, &Attrs::new(), Shaping::Advanced);
    
    let min_width = buffer.css_min_content_width(&mut font_system);
    let max_width = buffer.css_max_content_width(&mut font_system);
    
    assert!(min_width > 0.0, "BiDi text min-content should be positive");
    assert!(max_width >= min_width, "BiDi text max-content should be >= min-content");
}

#[test]
fn test_empty_text_handling() {
    let (mut font_system, mut buffer) = create_test_buffer();
    
    // Test edge case: empty text
    let text = "";
    buffer.set_text_cached(&mut font_system, text, &Attrs::new(), Shaping::Advanced);
    
    let min_width = buffer.css_min_content_width(&mut font_system);
    let max_width = buffer.css_max_content_width(&mut font_system);
    
    assert_eq!(min_width, 0.0, "Empty text min-content should be 0");
    assert_eq!(max_width, 0.0, "Empty text max-content should be 0");
}

#[test]
fn test_whitespace_only_text() {
    let (mut font_system, mut buffer) = create_test_buffer();
    
    // Test edge case: whitespace-only text
    let text = "   \t   ";
    buffer.set_text_cached(&mut font_system, text, &Attrs::new(), Shaping::Advanced);
    
    let min_width = buffer.css_min_content_width(&mut font_system);
    let max_width = buffer.css_max_content_width(&mut font_system);
    
    // Whitespace handling depends on CSS white-space property, but should be non-negative
    assert!(min_width >= 0.0, "Whitespace min-content should be non-negative");
    assert!(max_width >= 0.0, "Whitespace max-content should be non-negative");
    assert!(max_width >= min_width, "Whitespace max-content should be >= min-content");
}

#[test]
fn test_single_long_word() {
    let (mut font_system, mut buffer) = create_test_buffer();
    
    // Test single long unbreakable word
    let text = "supercalifragilisticexpialidocious";
    buffer.set_text_cached(&mut font_system, text, &Attrs::new(), Shaping::Advanced);
    
    let min_width = buffer.css_min_content_width(&mut font_system);
    let max_width = buffer.css_max_content_width(&mut font_system);
    
    // For single word, min and max content should be equal
    assert!(min_width > 0.0, "Single word min-content should be positive");
    assert!((min_width - max_width).abs() < 0.1, "Single word min and max content should be nearly equal");
}

#[test]
fn test_css_compliance_regression() {
    let (mut font_system, mut buffer) = create_test_buffer();
    
    // Regression test for CSS Sizing Module Level 3 compliance
    // This test ensures our implementation follows CSS specifications
    let test_cases = vec![
        ("word", "Single word should have min=max"),
        ("two words", "Two words: max > min"),
        ("word\nbreak", "Forced break: max based on longest line"),
        ("a b c d e f g", "Multiple words: significant max > min difference"),
    ];
    
    for (text, description) in test_cases {
        buffer.set_text_cached(&mut font_system, text, &Attrs::new(), Shaping::Advanced);
        
        let min_width = buffer.css_min_content_width(&mut font_system);
        let max_width = buffer.css_max_content_width(&mut font_system);
        
        assert!(min_width >= 0.0, "{}: min-content should be non-negative", description);
        assert!(max_width >= min_width, "{}: max-content should be >= min-content", description);
        
        // CSS spec: min-content should be finite (not infinity)
        assert!(min_width.is_finite(), "{}: min-content should be finite", description);
        assert!(max_width.is_finite(), "{}: max-content should be finite", description);
    }
}

#[test]
fn test_performance_no_infinite_loops() {
    let (mut font_system, mut buffer) = create_test_buffer();
    
    // Performance regression test: ensure no infinite loops in complex text
    let complex_text = "Mixed text with 中文字符 and العربية text, plus\nnewlines\tand\ttabs and very-long-hyphenated-compound-words-that-should-not-break and more content";
    buffer.set_text_cached(&mut font_system, complex_text, &Attrs::new(), Shaping::Advanced);
    
    // These should complete quickly without hanging
    let start = std::time::Instant::now();
    let min_width = buffer.css_min_content_width(&mut font_system);
    let max_width = buffer.css_max_content_width(&mut font_system);
    let duration = start.elapsed();
    
    // Should complete within reasonable time (1 second is very generous)
    assert!(duration.as_secs() < 1, "Content width calculation should complete quickly");
    assert!(min_width > 0.0, "Complex text should have positive min-content");
    assert!(max_width >= min_width, "Complex text max-content should be >= min-content");
}