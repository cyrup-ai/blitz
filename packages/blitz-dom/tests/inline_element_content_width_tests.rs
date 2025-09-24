//! Tests for inline element content width calculation
//! 
//! Validates that the calculate_content_widths_with_inline_elements() method
//! correctly includes inline elements (images, videos, canvas) in content width calculations

use blitz_text::{FontSystem, Metrics, EnhancedBuffer};
use blitz_dom::node::{TextLayout, InlineBox};

fn create_layout_with_image(width: f32) -> TextLayout {
    let mut font_system = FontSystem::new();
    let buffer = EnhancedBuffer::new(&mut font_system, Metrics::new(16.0, 20.0));
    
    // Create an inline box representing an image element
    let inline_box = InlineBox {
        id: 1,
        index: 0,
        width,
        height: 100.0, // Fixed height for simplicity
        x: 0.0,
        y: 0.0,
    };
    
    TextLayout {
        text: "Hello World".to_string(),
        layout: buffer,
        inline_boxes: vec![inline_box],
        cached_content_widths: None,
        cached_text_hash: None,
    }
}

fn create_layout_with_images(widths: Vec<f32>) -> TextLayout {
    let mut font_system = FontSystem::new();
    let buffer = EnhancedBuffer::new(&mut font_system, Metrics::new(16.0, 20.0));
    
    // Create multiple inline boxes representing image elements
    let inline_boxes: Vec<InlineBox> = widths
        .into_iter()
        .enumerate()
        .map(|(index, width)| InlineBox {
            id: (index + 1) as u64,
            index,
            width,
            height: 100.0, // Fixed height for simplicity
            x: index as f32 * width, // Position them horizontally
            y: 0.0,
        })
        .collect();
    
    TextLayout {
        text: "Hello World with images".to_string(),
        layout: buffer,
        inline_boxes,
        cached_content_widths: None,
        cached_text_hash: None,
    }
}

#[test]
fn test_image_contributes_to_content_width() {
    // Create layout with text + 100px image
    let mut layout = create_layout_with_image(100.0);
    let mut font_system = FontSystem::new();
    
    let widths = layout.calculate_content_widths_with_inline_elements(&mut font_system);
    
    // Should include image width, not ignore it
    assert!(widths.min >= 100.0, "Image should contribute to min-content: got {}", widths.min);
    assert!(widths.max >= 100.0, "Image should contribute to max-content: got {}", widths.max);
}

#[test]
fn test_multiple_images_content_width() {
    // Create layout with 3 images: 100px, 50px, 75px
    let mut layout = create_layout_with_images(vec![100.0, 50.0, 75.0]);
    let mut font_system = FontSystem::new();
    
    let widths = layout.calculate_content_widths_with_inline_elements(&mut font_system);
    
    // Min-content: largest image (100px)
    assert!(widths.min >= 100.0, "Min-content should include largest image: got {}", widths.min);
    // Max-content: sum of images (225px) plus text
    assert!(widths.max >= 225.0, "Max-content should include sum of all images: got {}", widths.max);
}

#[test]
fn test_no_inline_elements() {
    // Test that text-only layout still works correctly
    let mut font_system = FontSystem::new();
    let buffer = EnhancedBuffer::new(&mut font_system, Metrics::new(16.0, 20.0));
    
    let mut layout = TextLayout {
        text: "Hello World".to_string(),
        layout: buffer,
        inline_boxes: vec![], // No inline elements
        cached_content_widths: None,
        cached_text_hash: None,
    };
    
    let widths = layout.calculate_content_widths_with_inline_elements(&mut font_system);
    
    // Should work normally for text-only content
    assert!(widths.min >= 0.0, "Text-only min-content should be non-negative");
    assert!(widths.max >= widths.min, "Text-only max-content should be >= min-content");
}

#[test]
fn test_empty_text_with_images() {
    // Test layout with no text but with inline elements
    let mut font_system = FontSystem::new();
    let buffer = EnhancedBuffer::new(&mut font_system, Metrics::new(16.0, 20.0));
    
    let inline_box = InlineBox {
        id: 1,
        index: 0,
        width: 150.0,
        height: 100.0,
        x: 0.0,
        y: 0.0,
    };
    
    let mut layout = TextLayout {
        text: "".to_string(), // Empty text
        layout: buffer,
        inline_boxes: vec![inline_box],
        cached_content_widths: None,
        cached_text_hash: None,
    };
    
    let widths = layout.calculate_content_widths_with_inline_elements(&mut font_system);
    
    // Should be dominated by the inline element
    assert!(widths.min >= 150.0, "Empty text with image should have min-content >= image width");
    assert!(widths.max >= 150.0, "Empty text with image should have max-content >= image width");
}

#[test]
fn test_css_compliance_min_max_relationship() {
    // Test that CSS specification is followed: min-content <= max-content
    let mut layout = create_layout_with_images(vec![80.0, 120.0, 60.0]);
    let mut font_system = FontSystem::new();
    
    let widths = layout.calculate_content_widths_with_inline_elements(&mut font_system);
    
    // CSS requirement: min-content should always be <= max-content
    assert!(widths.min <= widths.max, 
        "CSS compliance violation: min-content ({}) should be <= max-content ({})", 
        widths.min, widths.max);
    
    // Min-content should be at least as wide as the largest image (120px)
    assert!(widths.min >= 120.0, 
        "Min-content should be at least as wide as largest inline element: got {}", 
        widths.min);
    
    // Max-content should include sum of all images (260px) plus text
    assert!(widths.max >= 260.0, 
        "Max-content should include sum of all inline elements: got {}", 
        widths.max);
}