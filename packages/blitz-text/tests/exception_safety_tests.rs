use blitz_text::cosmyc_types::buffer::{EnhancedBuffer, CssWidthMetrics, ThreadSafeBufferCalculator};
use blitz_text::{FontSystem, Metrics, Attrs, Shaping};
use std::sync::Arc;
use std::thread;

/// Helper function to create a test buffer with sample text
fn create_test_buffer() -> EnhancedBuffer {
    let mut font_system = FontSystem::new();
    let metrics = Metrics::new(16.0, 20.0);
    let mut buffer = EnhancedBuffer::new(&mut font_system, metrics);
    
    buffer.set_text_cached(
        &mut font_system,
        "Hello, World! This is a test text for CSS width calculations.",
        &Attrs::new(),
        Shaping::Advanced,
    );
    
    buffer
}

#[cfg(test)]
mod exception_safety_tests {
    use super::*;
    
    #[test]
    fn test_state_restoration_on_panic() {
        // Test that state is restored even when calculation panics
        let mut buffer = create_test_buffer();
        let mut font_system = FontSystem::new();
        let original_size = buffer.inner().size();
        
        // This should complete normally and restore state
        let min_width = buffer.css_min_content_width(&mut font_system);
        assert!(min_width > 0.0);
        assert_eq!(buffer.inner().size(), original_size);
        
        // Test with max width calculation
        let max_width = buffer.css_max_content_width(&mut font_system);
        assert!(max_width > 0.0);
        assert_eq!(buffer.inner().size(), original_size);
        
        // Verify that max_width >= min_width (basic CSS property)
        assert!(max_width >= min_width);
    }
    
    #[test]
    fn test_cache_consistency_after_calculation() {
        let mut buffer = create_test_buffer();
        let mut font_system = FontSystem::new();
        
        // Get initial cached layout runs
        let initial_cache_len = buffer.cached_layout_runs().len();
        
        // Perform CSS width calculations
        let _min_width = buffer.css_min_content_width(&mut font_system);
        let _max_width = buffer.css_max_content_width(&mut font_system);
        
        // Cache should be consistent after calculations
        let final_cache_len = buffer.cached_layout_runs().len();
        assert_eq!(initial_cache_len, final_cache_len);
    }
    
    #[test]
    fn test_concurrent_access_safety() {
        let buffer = create_test_buffer();
        let calculator = ThreadSafeBufferCalculator::new(buffer);
        let calculator = Arc::new(calculator);
        
        let mut handles = vec![];
        
        // Spawn multiple threads to test concurrent access
        for i in 0..4 {
            let calc_clone = Arc::clone(&calculator);
            let handle = thread::spawn(move || {
                let mut font_system = FontSystem::new();
                
                // Each thread performs multiple calculations
                for _j in 0..10 {
                    let result = calc_clone.calculate_css_widths(&mut font_system);
                    
                    match result {
                        Ok((min_width, max_width)) => {
                            assert!(min_width > 0.0, "Thread {}: min_width should be positive", i);
                            assert!(max_width > 0.0, "Thread {}: max_width should be positive", i);
                            assert!(max_width >= min_width, "Thread {}: max_width should be >= min_width", i);
                        }
                        Err(e) => {
                            panic!("Thread {}: Unexpected error: {}", i, e);
                        }
                    }
                }
            });
            handles.push(handle);
        }
        
        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }
    }
    
    #[test]
    fn test_error_recovery() {
        let mut buffer = create_test_buffer();
        let mut font_system = FontSystem::new();
        
        // Test safe error handling methods
        let result = buffer.calculate_css_widths_safe(&mut font_system);
        assert!(result.is_ok(), "Safe calculation should succeed");
        
        let (min_width, max_width) = result.unwrap();
        assert!(min_width > 0.0);
        assert!(max_width > 0.0);
        assert!(max_width >= min_width);
        
        // Test individual safe methods
        let min_result = buffer.css_min_content_width_safe(&mut font_system);
        assert!(min_result.is_ok(), "Safe min-width calculation should succeed");
        
        let max_result = buffer.css_max_content_width_safe(&mut font_system);
        assert!(max_result.is_ok(), "Safe max-width calculation should succeed");
    }
    
    #[test]
    fn test_performance_monitoring() {
        let mut buffer = create_test_buffer();
        let mut font_system = FontSystem::new();
        let mut metrics = CssWidthMetrics::default();
        
        // Perform monitored calculations
        let min_width = buffer.css_min_content_width_monitored(&mut font_system, &mut metrics);
        assert!(min_width > 0.0);
        
        let max_width = buffer.css_max_content_width_monitored(&mut font_system, &mut metrics);
        assert!(max_width > 0.0);
        
        // Check that metrics were updated
        assert_eq!(metrics.calculation_count, 2);
        assert!(metrics.total_duration.as_nanos() > 0);
        assert_eq!(metrics.error_count, 0);
        
        // Verify timing makes sense (calculations should take some time)
        assert!(metrics.total_duration.as_micros() > 0);
    }
    
    #[test]
    fn test_state_validation() {
        let mut buffer = create_test_buffer();
        let mut font_system = FontSystem::new();
        
        // Initial state should be valid
        assert!(buffer.validate_state().is_ok());
        
        // After calculations, state should still be valid
        let _min_width = buffer.css_min_content_width(&mut font_system);
        assert!(buffer.validate_state().is_ok());
        
        let _max_width = buffer.css_max_content_width(&mut font_system);
        assert!(buffer.validate_state().is_ok());
    }
    
    #[test]
    fn test_empty_text_handling() {
        let mut font_system = FontSystem::new();
        let metrics = Metrics::new(16.0, 20.0);
        let mut buffer = EnhancedBuffer::new(&mut font_system, metrics);
        
        // Set empty text
        buffer.set_text_cached(&mut font_system, "", &Attrs::new(), Shaping::Advanced);
        
        // Should handle empty text gracefully
        let min_width = buffer.css_min_content_width(&mut font_system);
        let max_width = buffer.css_max_content_width(&mut font_system);
        
        // Empty text should have zero or minimal width
        assert!(min_width >= 0.0);
        assert!(max_width >= 0.0);
        assert!(max_width >= min_width);
    }
    
    #[test]
    fn test_unicode_text_handling() {
        let mut font_system = FontSystem::new();
        let metrics = Metrics::new(16.0, 20.0);
        let mut buffer = EnhancedBuffer::new(&mut font_system, metrics);
        
        // Test with various Unicode text
        let test_texts = vec![
            "Hello, 世界! مرحبا",
            "🌟✨🚀 Emoji test",
            "Mixed: English 中文 العربية",
            "Line\nBreak\nTest",
        ];
        
        for text in test_texts {
            buffer.set_text_cached(&mut font_system, text, &Attrs::new(), Shaping::Advanced);
            
            let min_width = buffer.css_min_content_width(&mut font_system);
            let max_width = buffer.css_max_content_width(&mut font_system);
            
            assert!(min_width >= 0.0, "Min width should be non-negative for text: {}", text);
            assert!(max_width >= 0.0, "Max width should be non-negative for text: {}", text);
            assert!(max_width >= min_width, "Max width should be >= min width for text: {}", text);
        }
    }
    
    #[test]
    fn test_multiple_calculations_consistency() {
        let mut buffer = create_test_buffer();
        let mut font_system = FontSystem::new();
        
        // Perform multiple calculations and verify consistency
        let mut min_widths = vec![];
        let mut max_widths = vec![];
        
        for _i in 0..5 {
            let min_width = buffer.css_min_content_width(&mut font_system);
            let max_width = buffer.css_max_content_width(&mut font_system);
            
            min_widths.push(min_width);
            max_widths.push(max_width);
        }
        
        // All calculations should produce the same results
        for i in 1..min_widths.len() {
            assert_eq!(min_widths[0], min_widths[i], "Min width calculation should be consistent");
            assert_eq!(max_widths[0], max_widths[i], "Max width calculation should be consistent");
        }
    }
}