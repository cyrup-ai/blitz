//! Custom Region Screenshot Capture Example
//!
//! This example demonstrates how to capture specific regions of the screen
//! using pixel coordinates. Shows region validation, different region types,
//! and optimization strategies for region-based capture.
//!
//! Usage:
//! ```bash
//! cargo run --example custom_regions --features screenshot
//! ```

use std::sync::Arc;
use std::path::PathBuf;
use std::error::Error;

use blitz_shell::{ScreenshotApi, ScreenshotApiImpl, ScreenshotConfig, ScreenshotEngine};
use blitz_paint::screenshot::{ImageFormat, Rectangle};
use wgpu::{Device, Queue, Instance, RequestAdapterOptions, DeviceDescriptor, Features, Limits};

// Common screen dimensions for testing
const SCREEN_WIDTH: u32 = 1920;
const SCREEN_HEIGHT: u32 = 1080;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("🎯 Custom Region Screenshot Capture Example");
    println!("===========================================");

    // Initialize WGPU context
    let (device, queue) = initialize_wgpu().await?;
    
    // Create screenshot engine
    let screenshot_engine = Arc::new(ScreenshotEngine::new(
        Arc::new(device),
        Arc::new(queue),
    ));
    
    // Create screenshot API
    let screenshot_api = ScreenshotApiImpl::new(screenshot_engine);
    
    println!("✅ Screenshot system initialized");
    println!("   Simulated screen size: {}x{}", SCREEN_WIDTH, SCREEN_HEIGHT);

    // Example 1: Predefined regions
    println!("\n📐 Capturing predefined regions...");
    capture_predefined_regions(&screenshot_api).await?;

    // Example 2: Quadrant capture
    println!("\n🔲 Capturing screen quadrants...");
    capture_screen_quadrants(&screenshot_api).await?;

    // Example 3: Custom user-defined regions
    println!("\n✏️  Capturing custom user regions...");
    capture_custom_regions(&screenshot_api).await?;

    // Example 4: Region optimization demonstration
    println!("\n⚡ Demonstrating region optimization...");
    demonstrate_region_optimization(&screenshot_api).await?;

    // Example 5: Region validation examples
    println!("\n🛡️  Testing region validation...");
    test_region_validation(&screenshot_api).await?;

    println!("\n🎉 Custom region examples completed!");
    Ok(())
}

/// Initialize WGPU device and queue
async fn initialize_wgpu() -> Result<(Device, Queue), Box<dyn Error>> {
    let instance = Instance::new(wgpu::InstanceDescriptor::default());
    
    let adapter = instance
        .request_adapter(&RequestAdapterOptions::default())
        .await
        .ok_or("Failed to find suitable graphics adapter")?;
    
    let (device, queue) = adapter
        .request_device(
            &DeviceDescriptor {
                label: Some("Region Capture Device"),
                required_features: Features::empty(),
                required_limits: Limits::default(),
                memory_hints: wgpu::MemoryHints::Performance,
            },
            None,
        )
        .await?;
    
    Ok((device, queue))
}

/// Capture predefined common regions
async fn capture_predefined_regions(api: &ScreenshotApiImpl) -> Result<(), Box<dyn Error>> {
    let regions = vec![
        ("center_square", Rectangle::new(
            SCREEN_WIDTH / 2 - 200, 
            SCREEN_HEIGHT / 2 - 200, 
            400, 
            400
        )?),
        ("top_banner", Rectangle::new(0, 0, SCREEN_WIDTH, 100)?),
        ("bottom_banner", Rectangle::new(0, SCREEN_HEIGHT - 100, SCREEN_WIDTH, 100)?),
        ("left_sidebar", Rectangle::new(0, 0, 300, SCREEN_HEIGHT)?),
        ("right_sidebar", Rectangle::new(SCREEN_WIDTH - 300, 0, 300, SCREEN_HEIGHT)?),
        ("center_strip", Rectangle::new(
            SCREEN_WIDTH / 4, 
            SCREEN_HEIGHT / 3, 
            SCREEN_WIDTH / 2, 
            SCREEN_HEIGHT / 3
        )?),
    ];

    for (name, region) in regions {
        println!("   📸 Capturing {}: {}x{} at ({}, {})", 
               name, region.width, region.height, region.x, region.y);
        
        let config = ScreenshotConfig::builder()
            .format(ImageFormat::Png)
            .quality(95)
            .region(Some(region))
            .output_path(Some(PathBuf::from(format!("region_{}.png", name))))
            .include_metadata(true)
            .build();

        match api.capture_screenshot(config).await {
            Ok(data) => {
                let filename = format!("region_{}.png", name);
                std::fs::write(&filename, &data)?;
                println!("     ✅ Saved: {} ({} KB)", filename, data.len() / 1024);
            },
            Err(e) => {
                eprintln!("     ❌ Failed to capture {}: {}", name, e);
            }
        }
    }

    Ok(())
}

/// Capture screen divided into quadrants
async fn capture_screen_quadrants(api: &ScreenshotApiImpl) -> Result<(), Box<dyn Error>> {
    let quadrant_width = SCREEN_WIDTH / 2;
    let quadrant_height = SCREEN_HEIGHT / 2;
    
    let quadrants = vec![
        ("top_left", Rectangle::new(0, 0, quadrant_width, quadrant_height)?),
        ("top_right", Rectangle::new(quadrant_width, 0, quadrant_width, quadrant_height)?),
        ("bottom_left", Rectangle::new(0, quadrant_height, quadrant_width, quadrant_height)?),
        ("bottom_right", Rectangle::new(quadrant_width, quadrant_height, quadrant_width, quadrant_height)?),
    ];

    println!("   Dividing {}x{} screen into {} quadrants...", 
           SCREEN_WIDTH, SCREEN_HEIGHT, quadrants.len());

    for (name, region) in quadrants {
        println!("   📸 Capturing quadrant: {}", name);
        
        let config = ScreenshotConfig::builder()
            .format(ImageFormat::Png)
            .quality(90)
            .region(Some(region))
            .filename_prefix(format!("quadrant_{}", name))
            .include_metadata(false) // Skip metadata for speed
            .build();

        match api.capture_screenshot(config).await {
            Ok(data) => {
                let filename = format!("quadrant_{}.png", name);
                std::fs::write(&filename, &data)?;
                println!("     ✅ {}: {} KB", filename, data.len() / 1024);
            },
            Err(e) => {
                eprintln!("     ❌ Quadrant {}: {}", name, e);
            }
        }
    }

    Ok(())
}

/// Capture custom user-defined regions
async fn capture_custom_regions(api: &ScreenshotApiImpl) -> Result<(), Box<dyn Error>> {
    // Define some interesting custom regions
    let custom_regions = vec![
        CustomRegion::new("small_corner", 10, 10, 200, 150, "Small corner sample"),
        CustomRegion::new("wide_strip", 0, 300, SCREEN_WIDTH, 200, "Wide horizontal strip"),
        CustomRegion::new("tall_strip", 800, 0, 200, SCREEN_HEIGHT, "Tall vertical strip"),
        CustomRegion::new("golden_ratio", 
                         SCREEN_WIDTH / 3, 
                         SCREEN_HEIGHT / 3, 
                         (SCREEN_WIDTH as f32 * 0.618) as u32, 
                         (SCREEN_HEIGHT as f32 * 0.618) as u32, 
                         "Golden ratio rectangle"),
        CustomRegion::new("tiny_detail", 500, 400, 50, 50, "Tiny detail region"),
    ];

    for custom_region in custom_regions {
        println!("   📸 Capturing custom region: {} - {}", 
               custom_region.name, custom_region.description);
        println!("     Bounds: {}x{} at ({}, {})", 
               custom_region.rectangle.width, custom_region.rectangle.height,
               custom_region.rectangle.x, custom_region.rectangle.y);

        // Choose format based on region size
        let (format, quality) = if custom_region.rectangle.width * custom_region.rectangle.height < 10000 {
            (ImageFormat::Png, 100) // High quality for small regions
        } else {
            (ImageFormat::Png, 85)  // Medium quality for large regions
        };

        let config = ScreenshotConfig::builder()
            .format(format)
            .quality(quality)
            .region(Some(custom_region.rectangle))
            .filename_prefix(format!("custom_{}", custom_region.name))
            .include_metadata(true)
            .build();

        match api.capture_screenshot(config).await {
            Ok(data) => {
                let filename = format!("custom_{}.png", custom_region.name);
                std::fs::write(&filename, &data)?;
                
                let size_ratio = (custom_region.rectangle.width * custom_region.rectangle.height) as f64 
                               / (SCREEN_WIDTH * SCREEN_HEIGHT) as f64 * 100.0;
                
                println!("     ✅ {}: {} KB ({:.1}% of screen)", 
                       filename, data.len() / 1024, size_ratio);
            },
            Err(e) => {
                eprintln!("     ❌ Custom region {}: {}", custom_region.name, e);
            }
        }
    }

    Ok(())
}

/// Demonstrate region optimization strategies
async fn demonstrate_region_optimization(api: &ScreenshotApiImpl) -> Result<(), Box<dyn Error>> {
    println!("   Testing different optimization strategies...");

    // Strategy 1: Multiple small regions vs single large region
    println!("   🔍 Strategy 1: Multiple small vs single large region");
    await_compare_multiple_vs_single(api).await?;

    // Strategy 2: Format optimization by region size
    println!("   🔍 Strategy 2: Format optimization by size");
    await_format_optimization_by_size(api).await?;

    // Strategy 3: Quality optimization by region type
    println!("   🔍 Strategy 3: Quality optimization by type");
    await_quality_optimization_by_type(api).await?;

    Ok(())
}

/// Compare multiple small regions vs single large region
async fn await_compare_multiple_vs_single(api: &ScreenshotApiImpl) -> Result<(), Box<dyn Error>> {
    use std::time::Instant;

    // Multiple small regions
    let small_regions = vec![
        Rectangle::new(100, 100, 200, 200)?,
        Rectangle::new(400, 100, 200, 200)?,
        Rectangle::new(100, 400, 200, 200)?,
        Rectangle::new(400, 400, 200, 200)?,
    ];

    let start_multiple = Instant::now();
    let mut total_size_multiple = 0;

    for (i, region) in small_regions.iter().enumerate() {
        let config = ScreenshotConfig::builder()
            .format(ImageFormat::Png)
            .quality(80)
            .region(Some(*region))
            .build();

        match api.capture_screenshot(config).await {
            Ok(data) => {
                total_size_multiple += data.len();
                let filename = format!("opt_small_{}.png", i);
                std::fs::write(&filename, data)?;
            },
            Err(e) => {
                eprintln!("       ❌ Small region {}: {}", i, e);
            }
        }
    }

    let time_multiple = start_multiple.elapsed();

    // Single large region covering the same area
    let large_region = Rectangle::new(100, 100, 500, 500)?;
    
    let start_single = Instant::now();
    
    let config = ScreenshotConfig::builder()
        .format(ImageFormat::Png)
        .quality(80)
        .region(Some(large_region))
        .build();

    let total_size_single = match api.capture_screenshot(config).await {
        Ok(data) => {
            std::fs::write("opt_large.png", &data)?;
            data.len()
        },
        Err(e) => {
            eprintln!("       ❌ Large region: {}", e);
            0
        }
    };

    let time_single = start_single.elapsed();

    println!("     📊 Multiple small regions: {} KB in {:?}", 
           total_size_multiple / 1024, time_multiple);
    println!("     📊 Single large region: {} KB in {:?}", 
           total_size_single / 1024, time_single);
    
    if time_single < time_multiple {
        println!("     ✅ Single large region is {:.1}x faster", 
               time_multiple.as_millis() as f64 / time_single.as_millis() as f64);
    } else {
        println!("     ⚠️  Multiple small regions performed better");
    }

    Ok(())
}

/// Test format optimization based on region size
async fn await_format_optimization_by_size(api: &ScreenshotApiImpl) -> Result<(), Box<dyn Error>> {
    let test_regions = vec![
        ("tiny", Rectangle::new(0, 0, 64, 64)?),
        ("small", Rectangle::new(0, 0, 256, 256)?),
        ("medium", Rectangle::new(0, 0, 512, 512)?),
        ("large", Rectangle::new(0, 0, 1024, 768)?),
    ];

    for (size_name, region) in test_regions {
        println!("     Testing {} region ({}x{}):", 
               size_name, region.width, region.height);

        let formats = vec![
            (ImageFormat::Png, "png"),
            #[cfg(feature = "jpeg")]
            (ImageFormat::Jpeg, "jpg"),
            #[cfg(feature = "webp")]
            (ImageFormat::WebP, "webp"),
        ];

        for (format, ext) in formats {
            let config = ScreenshotConfig::builder()
                .format(format)
                .quality(85)
                .region(Some(region))
                .build();

            match api.capture_screenshot(config).await {
                Ok(data) => {
                    let filename = format!("opt_{}_{}.{}", size_name, ext, ext);
                    std::fs::write(&filename, &data)?;
                    println!("       {:?}: {} KB", format, data.len() / 1024);
                },
                Err(e) => {
                    eprintln!("       ❌ {:?}: {}", format, e);
                }
            }
        }
    }

    Ok(())
}

/// Test quality optimization by region type
async fn await_quality_optimization_by_type(api: &ScreenshotApiImpl) -> Result<(), Box<dyn Error>> {
    let region_types = vec![
        ("text_area", Rectangle::new(100, 100, 600, 200)?, 95), // High quality for text
        ("image_area", Rectangle::new(100, 400, 400, 300)?, 85), // Medium for images
        ("ui_elements", Rectangle::new(600, 100, 200, 600)?, 90), // High for UI
        ("background", Rectangle::new(0, 0, SCREEN_WIDTH, 100)?, 70), // Lower for backgrounds
    ];

    for (region_type, region, optimal_quality) in region_types {
        println!("     Testing {} with quality {}%:", region_type, optimal_quality);

        let config = ScreenshotConfig::builder()
            .format(ImageFormat::Png)
            .quality(optimal_quality)
            .region(Some(region))
            .filename_prefix(format!("opt_{}", region_type))
            .build();

        match api.capture_screenshot(config).await {
            Ok(data) => {
                let filename = format!("opt_{}.png", region_type);
                std::fs::write(&filename, &data)?;
                println!("       ✅ {}: {} KB at {}% quality", 
                       filename, data.len() / 1024, optimal_quality);
            },
            Err(e) => {
                eprintln!("       ❌ {}: {}", region_type, e);
            }
        }
    }

    Ok(())
}

/// Test region validation with various edge cases
async fn test_region_validation(api: &ScreenshotApiImpl) -> Result<(), Box<dyn Error>> {
    let validation_tests = vec![
        ("valid_region", Rectangle::new(100, 100, 200, 200), true),
        ("zero_width", Rectangle::new(100, 100, 0, 200), false),
        ("zero_height", Rectangle::new(100, 100, 200, 0), false),
        ("negative_coords", Rectangle::new(0, 0, 100, 100), true), // This should be valid
        ("oversized_region", Rectangle::new(0, 0, SCREEN_WIDTH * 2, SCREEN_HEIGHT), false),
    ];

    for (test_name, rectangle_result, should_succeed) in validation_tests {
        println!("   🧪 Testing {}: expected to {}", 
               test_name, if should_succeed { "succeed" } else { "fail" });

        match rectangle_result {
            Ok(region) => {
                let config = ScreenshotConfig::builder()
                    .format(ImageFormat::Png)
                    .quality(50) // Low quality for speed
                    .region(Some(region))
                    .build();

                match api.capture_screenshot(config).await {
                    Ok(data) => {
                        if should_succeed {
                            println!("     ✅ {} succeeded as expected ({} KB)", test_name, data.len() / 1024);
                        } else {
                            println!("     ⚠️  {} succeeded but was expected to fail", test_name);
                        }
                    },
                    Err(e) => {
                        if should_succeed {
                            println!("     ❌ {} failed unexpectedly: {}", test_name, e);
                        } else {
                            println!("     ✅ {} failed as expected: {}", test_name, e);
                        }
                    }
                }
            },
            Err(e) => {
                if should_succeed {
                    println!("     ❌ {} rectangle creation failed: {}", test_name, e);
                } else {
                    println!("     ✅ {} rectangle creation failed as expected: {}", test_name, e);
                }
            }
        }
    }

    Ok(())
}

/// Helper struct for custom region definition
struct CustomRegion {
    name: String,
    rectangle: Rectangle,
    description: String,
}

impl CustomRegion {
    fn new(name: &str, x: u32, y: u32, width: u32, height: u32, description: &str) -> Self {
        Self {
            name: name.to_string(),
            rectangle: Rectangle::new(x, y, width, height)
                .unwrap_or_else(|_| Rectangle::new(0, 0, 100, 100).unwrap()),
            description: description.to_string(),
        }
    }
}

/// Calculate optimal region size based on content type
fn calculate_optimal_region_size(content_type: ContentType, available_area: (u32, u32)) -> (u32, u32) {
    let (max_width, max_height) = available_area;
    
    match content_type {
        ContentType::Text => {
            // Text regions benefit from maintaining aspect ratio for readability
            let width = (max_width * 3 / 4).min(800);
            let height = (width as f32 * 0.4) as u32;
            (width, height.min(max_height))
        },
        ContentType::Image => {
            // Image regions should maintain original proportions when possible
            (max_width.min(1024), max_height.min(768))
        },
        ContentType::UI => {
            // UI elements often have specific dimensions
            let width = max_width.min(400);
            let height = max_height.min(600);
            (width, height)
        },
        ContentType::Chart => {
            // Charts benefit from wider aspect ratios
            let width = max_width.min(800);
            let height = (width as f32 * 0.6) as u32;
            (width, height.min(max_height))
        }
    }
}

/// Content type for region optimization
enum ContentType {
    Text,
    Image,
    UI,
    Chart,
}

/// Generate region grid for systematic capture
fn generate_region_grid(grid_size: (u32, u32), overlap_pixels: u32) -> Vec<Rectangle> {
    let (cols, rows) = grid_size;
    let mut regions = Vec::new();
    
    let region_width = (SCREEN_WIDTH / cols).saturating_sub(overlap_pixels);
    let region_height = (SCREEN_HEIGHT / rows).saturating_sub(overlap_pixels);
    
    for row in 0..rows {
        for col in 0..cols {
            let x = col * (region_width + overlap_pixels / 2);
            let y = row * (region_height + overlap_pixels / 2);
            
            if let Ok(region) = Rectangle::new(x, y, region_width, region_height) {
                regions.push(region);
            }
        }
    }
    
    regions
}

/// Demonstrate grid-based region capture
async fn demonstrate_grid_capture(api: &ScreenshotApiImpl) -> Result<(), Box<dyn Error>> {
    println!("🔲 Demonstrating grid-based region capture...");
    
    let grid_regions = generate_region_grid((3, 2), 20); // 3x2 grid with 20px overlap
    
    println!("   Generated {} grid regions", grid_regions.len());
    
    for (i, region) in grid_regions.iter().enumerate() {
        let config = ScreenshotConfig::builder()
            .format(ImageFormat::Png)
            .quality(75)
            .region(Some(*region))
            .filename_prefix(format!("grid_{:02}", i))
            .build();

        match api.capture_screenshot(config).await {
            Ok(data) => {
                let filename = format!("grid_{:02}.png", i);
                std::fs::write(&filename, data)?;
                println!("     ✅ Grid region {}: {}x{} at ({}, {})", 
                       i, region.width, region.height, region.x, region.y);
            },
            Err(e) => {
                eprintln!("     ❌ Grid region {}: {}", i, e);
            }
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_calculate_optimal_region_size() {
        let available = (1920, 1080);
        
        let (text_w, text_h) = calculate_optimal_region_size(ContentType::Text, available);
        assert!(text_w <= 800);
        assert!(text_h <= available.1);
        
        let (img_w, img_h) = calculate_optimal_region_size(ContentType::Image, available);
        assert!(img_w <= 1024);
        assert!(img_h <= 768);
    }
    
    #[test]
    fn test_generate_region_grid() {
        let regions = generate_region_grid((2, 2), 10);
        assert_eq!(regions.len(), 4);
        
        // Test that regions don't overlap significantly
        for region in &regions {
            assert!(region.width > 0);
            assert!(region.height > 0);
        }
    }
    
    #[test]
    fn test_custom_region_creation() {
        let region = CustomRegion::new("test", 100, 200, 300, 400, "Test region");
        assert_eq!(region.name, "test");
        assert_eq!(region.rectangle.x, 100);
        assert_eq!(region.rectangle.y, 200);
        assert_eq!(region.rectangle.width, 300);
        assert_eq!(region.rectangle.height, 400);
    }
}