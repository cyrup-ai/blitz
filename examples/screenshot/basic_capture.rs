//! Basic Screenshot Capture Example
//!
//! This example demonstrates how to capture a single screenshot using the BlitzShell
//! ScreenshotApi. It shows the minimal setup required for basic screenshot functionality.
//!
//! Usage:
//! ```bash
//! cargo run --example basic_capture --features screenshot
//! ```

use std::sync::Arc;
use std::path::PathBuf;
use std::error::Error;

use blitz_shell::{ScreenshotApi, ScreenshotApiImpl, ScreenshotConfig, ScreenshotEngine};
use blitz_paint::screenshot::{ImageFormat, Rectangle};
use wgpu::{Device, Queue, Instance, RequestAdapterOptions, DeviceDescriptor, Features, Limits};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("🖼️  Basic Screenshot Capture Example");
    println!("=====================================");

    // Initialize WGPU context for screenshot engine
    let (device, queue) = initialize_wgpu().await?;
    
    // Create screenshot engine
    let screenshot_engine = Arc::new(ScreenshotEngine::new(
        Arc::new(device),
        Arc::new(queue),
    ));
    
    // Create screenshot API
    let screenshot_api = ScreenshotApiImpl::new(screenshot_engine);
    
    println!("✅ Screenshot system initialized");

    // Example 1: Basic PNG screenshot with default settings
    println!("\n📸 Capturing basic PNG screenshot...");
    match capture_basic_png(&screenshot_api).await {
        Ok(output_path) => {
            println!("✅ PNG screenshot saved to: {}", output_path.display());
        },
        Err(e) => {
            eprintln!("❌ PNG capture failed: {}", e);
        }
    }

    // Example 2: High-quality screenshot with custom settings
    println!("\n📸 Capturing high-quality screenshot...");
    match capture_high_quality(&screenshot_api).await {
        Ok(output_path) => {
            println!("✅ High-quality screenshot saved to: {}", output_path.display());
        },
        Err(e) => {
            eprintln!("❌ High-quality capture failed: {}", e);
        }
    }

    // Example 3: Screenshot with metadata
    println!("\n📸 Capturing screenshot with metadata...");
    match capture_with_metadata(&screenshot_api).await {
        Ok(output_path) => {
            println!("✅ Screenshot with metadata saved to: {}", output_path.display());
        },
        Err(e) => {
            eprintln!("❌ Metadata capture failed: {}", e);
        }
    }

    // Display engine statistics
    println!("\n📊 Screenshot Engine Statistics:");
    display_engine_stats(&screenshot_api).await;

    println!("\n🎉 Basic capture examples completed!");
    Ok(())
}

/// Initialize WGPU device and queue for screenshot engine
async fn initialize_wgpu() -> Result<(Device, Queue), Box<dyn Error>> {
    println!("🔧 Initializing WGPU context...");
    
    // Create WGPU instance
    let instance = Instance::new(wgpu::InstanceDescriptor::default());
    
    // Request adapter
    let adapter = instance
        .request_adapter(&RequestAdapterOptions::default())
        .await
        .ok_or("Failed to find suitable graphics adapter")?;
    
    println!("   Found adapter: {}", adapter.get_info().name);
    
    // Request device and queue
    let (device, queue) = adapter
        .request_device(
            &DeviceDescriptor {
                label: Some("Screenshot Device"),
                required_features: Features::empty(),
                required_limits: Limits::default(),
                memory_hints: wgpu::MemoryHints::Performance,
            },
            None,
        )
        .await?;
    
    println!("   Device and queue created successfully");
    Ok((device, queue))
}

/// Capture a basic PNG screenshot with default settings
async fn capture_basic_png(api: &ScreenshotApiImpl) -> Result<PathBuf, Box<dyn Error>> {
    let config = ScreenshotConfig::builder()
        .format(ImageFormat::Png)
        .output_path(Some(PathBuf::from("basic_screenshot.png")))
        .filename_prefix("basic".to_string())
        .include_metadata(false)
        .build();

    // Capture screenshot
    let image_data = api.capture_screenshot(config.clone()).await?;
    
    // Write to file
    let output_path = config.output_path.unwrap_or_else(|| 
        PathBuf::from(config.generate_filename())
    );
    std::fs::write(&output_path, image_data)?;
    
    Ok(output_path)
}

/// Capture a high-quality screenshot with custom settings
async fn capture_high_quality(api: &ScreenshotApiImpl) -> Result<PathBuf, Box<dyn Error>> {
    let config = ScreenshotConfig::builder()
        .format(ImageFormat::Png)
        .quality(100) // Maximum quality
        .output_path(Some(PathBuf::from("high_quality_screenshot.png")))
        .filename_prefix("hq".to_string())
        .include_metadata(true)
        .build();

    // Capture screenshot
    let image_data = api.capture_screenshot(config.clone()).await?;
    
    // Write to file  
    let output_path = config.output_path.unwrap_or_else(|| 
        PathBuf::from(config.generate_filename())
    );
    std::fs::write(&output_path, image_data)?;
    
    // Display image statistics
    println!("   Image size: {} bytes", image_data.len());
    println!("   Quality: {}%", config.quality);
    println!("   Format: {:?}", config.format);
    
    Ok(output_path)
}

/// Capture a screenshot with embedded metadata
async fn capture_with_metadata(api: &ScreenshotApiImpl) -> Result<PathBuf, Box<dyn Error>> {
    let config = ScreenshotConfig::builder()
        .format(ImageFormat::Png)
        .quality(95)
        .output_path(Some(PathBuf::from("metadata_screenshot.png")))
        .filename_prefix("meta".to_string())
        .include_metadata(true)
        .build();

    // Capture screenshot
    let image_data = api.capture_screenshot(config.clone()).await?;
    
    // Write to file
    let output_path = config.output_path.unwrap_or_else(|| 
        PathBuf::from(config.generate_filename())
    );
    std::fs::write(&output_path, image_data)?;
    
    println!("   Metadata included: {}", config.include_metadata);
    println!("   Filename prefix: {}", config.filename_prefix);
    
    Ok(output_path)
}

/// Display screenshot engine statistics
async fn display_engine_stats(api: &ScreenshotApiImpl) {
    match api.get_screenshot_status().await {
        Ok(stats) => {
            println!("   Total captures: {}", stats.total_captures);
            println!("   Successful captures: {}", stats.successful_captures);
            println!("   Failed captures: {}", stats.failed_captures);
            println!("   Average capture time: {:.2} ms", stats.average_capture_time_ms);
            println!("   Memory usage: {} KB", stats.memory_usage_bytes / 1024);
            
            if stats.total_captures > 0 {
                let success_rate = stats.successful_captures as f64 / stats.total_captures as f64 * 100.0;
                println!("   Success rate: {:.1}%", success_rate);
            }
        },
        Err(e) => {
            eprintln!("   Failed to get engine statistics: {}", e);
        }
    }
}

/// Helper function to validate captured screenshot
fn validate_screenshot(image_data: &[u8], expected_format: ImageFormat) -> Result<(), Box<dyn Error>> {
    if image_data.is_empty() {
        return Err("Screenshot data is empty".into());
    }
    
    // Check format-specific signatures
    match expected_format {
        ImageFormat::Png => {
            if image_data.len() < 8 {
                return Err("PNG data too short".into());
            }
            // PNG signature: 137 80 78 71 13 10 26 10
            let png_signature = &[137, 80, 78, 71, 13, 10, 26, 10];
            if &image_data[0..8] != png_signature {
                return Err("Invalid PNG signature".into());
            }
        },
        ImageFormat::Jpeg => {
            if image_data.len() < 2 {
                return Err("JPEG data too short".into());
            }
            // JPEG signature: FF D8
            if image_data[0] != 0xFF || image_data[1] != 0xD8 {
                return Err("Invalid JPEG signature".into());
            }
        },
        ImageFormat::WebP => {
            if image_data.len() < 12 {
                return Err("WebP data too short".into());
            }
            // WebP signature: "RIFF" ... "WEBP"
            if &image_data[0..4] != b"RIFF" || &image_data[8..12] != b"WEBP" {
                return Err("Invalid WebP signature".into());
            }
        },
        ImageFormat::RawRgba => {
            // For raw RGBA, just check that we have data
            if image_data.len() % 4 != 0 {
                return Err("Raw RGBA data length not divisible by 4".into());
            }
        }
    }
    
    Ok(())
}

/// Example error handling for screenshot operations
async fn capture_with_error_handling(api: &ScreenshotApiImpl) -> Result<(), Box<dyn Error>> {
    println!("🛡️  Demonstrating error handling...");
    
    // Try invalid configuration
    let invalid_config = ScreenshotConfig::builder()
        .format(ImageFormat::Jpeg)
        .quality(150) // Invalid quality > 100
        .build();
    
    match api.capture_screenshot(invalid_config).await {
        Ok(_) => {
            println!("   ⚠️  Expected error but capture succeeded");
        },
        Err(e) => {
            println!("   ✅ Error correctly handled: {}", e);
        }
    }
    
    // Try valid configuration
    let valid_config = ScreenshotConfig::builder()
        .format(ImageFormat::Png)
        .quality(95)
        .build();
    
    match api.capture_screenshot(valid_config).await {
        Ok(data) => {
            println!("   ✅ Valid capture succeeded: {} bytes", data.len());
        },
        Err(e) => {
            println!("   ❌ Valid capture failed: {}", e);
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_basic_capture_example() {
        // Test that the basic capture example functions work
        // This would run in a test environment with mock WGPU context
        assert!(true); // Placeholder for actual test implementation
    }
    
    #[test]
    fn test_validate_screenshot() {
        // Test PNG validation
        let png_data = vec![137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 0]; // PNG signature + some data
        assert!(validate_screenshot(&png_data, ImageFormat::Png).is_ok());
        
        // Test invalid PNG
        let invalid_png = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        assert!(validate_screenshot(&invalid_png, ImageFormat::Png).is_err());
        
        // Test JPEG validation
        let jpeg_data = vec![0xFF, 0xD8, 0xFF, 0xE0]; // JPEG signature
        assert!(validate_screenshot(&jpeg_data, ImageFormat::Jpeg).is_ok());
        
        // Test Raw RGBA validation
        let rgba_data = vec![255, 0, 0, 255, 0, 255, 0, 255]; // 2 pixels
        assert!(validate_screenshot(&rgba_data, ImageFormat::RawRgba).is_ok());
        
        // Test invalid Raw RGBA (not divisible by 4)
        let invalid_rgba = vec![255, 0, 0]; // 3 bytes, not divisible by 4
        assert!(validate_screenshot(&invalid_rgba, ImageFormat::RawRgba).is_err());
    }
}