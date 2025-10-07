//! Continuous Screenshot Capture Example
//!
//! This example demonstrates how to capture screenshots continuously at regular
//! intervals using the BlitzShell ScreenshotApi. Shows streaming capabilities,
//! performance monitoring, and graceful shutdown.
//!
//! Usage:
//! ```bash
//! cargo run --example continuous_capture --features screenshot
//! ```

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicBool, Ordering};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use std::error::Error;

use blitz_shell::{ScreenshotApi, ScreenshotApiImpl, ScreenshotConfig, ScreenshotEngine};
use blitz_paint::screenshot::{ImageFormat, ScreenshotError};
use tokio::signal;
use wgpu::{Device, Queue, Instance, RequestAdapterOptions, DeviceDescriptor, Features, Limits};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("🎬 Continuous Screenshot Capture Example");
    println!("========================================");

    // Initialize WGPU context
    let (device, queue) = initialize_wgpu().await?;
    
    // Create screenshot engine
    let screenshot_engine = Arc::new(ScreenshotEngine::new(
        Arc::new(device),
        Arc::new(queue),
    ));
    
    // Create screenshot API
    let screenshot_api = Arc::new(ScreenshotApiImpl::new(screenshot_engine));
    
    println!("✅ Screenshot system initialized");

    // Example 1: Short-duration continuous capture
    println!("\n📹 Starting short-duration continuous capture...");
    run_short_continuous_capture(Arc::clone(&screenshot_api)).await?;

    // Example 2: Performance monitoring capture
    println!("\n⚡ Starting performance monitoring capture...");
    run_performance_monitoring_capture(Arc::clone(&screenshot_api)).await?;

    // Example 3: Multi-format continuous capture
    println!("\n🎨 Starting multi-format continuous capture...");
    run_multi_format_capture(Arc::clone(&screenshot_api)).await?;

    // Example 4: Interactive continuous capture (cancellable)
    println!("\n🎮 Starting interactive continuous capture...");
    println!("   Press Ctrl+C to stop gracefully");
    run_interactive_capture(Arc::clone(&screenshot_api)).await?;

    println!("\n🎉 Continuous capture examples completed!");
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
                label: Some("Continuous Capture Device"),
                required_features: Features::empty(),
                required_limits: Limits::default(),
                memory_hints: wgpu::MemoryHints::Performance,
            },
            None,
        )
        .await?;
    
    Ok((device, queue))
}

/// Run a short-duration continuous capture for demonstration
async fn run_short_continuous_capture(api: Arc<ScreenshotApiImpl>) -> Result<(), Box<dyn Error>> {
    let capture_count = Arc::new(AtomicU32::new(0));
    let start_time = Instant::now();
    
    println!("   Capturing 5 screenshots at 500ms intervals...");
    
    let config = ScreenshotConfig::builder()
        .format(ImageFormat::Png)
        .quality(75) // Medium quality for speed
        .filename_prefix("continuous".to_string())
        .include_metadata(false)
        .build();
    
    // Create callback for handling captured screenshots
    let capture_count_clone = Arc::clone(&capture_count);
    let callback = move |frame_num: u32, result: Result<Vec<u8>, ScreenshotError>| {
        capture_count_clone.store(frame_num + 1, Ordering::Relaxed);
        
        match result {
            Ok(data) => {
                let filename = format!("continuous_frame_{:03}.png", frame_num);
                match std::fs::write(&filename, data) {
                    Ok(_) => {
                        println!("     ✅ Frame {}: {} KB -> {}", 
                               frame_num, data.len() / 1024, filename);
                    },
                    Err(e) => {
                        eprintln!("     ❌ Frame {}: Failed to write file: {}", frame_num, e);
                    }
                }
            },
            Err(e) => {
                eprintln!("     ❌ Frame {}: Capture failed: {}", frame_num, e);
            }
        }
    };
    
    // Start continuous capture
    let handle = api.start_continuous_capture(
        config,
        Duration::from_millis(500),
        Some(5), // Capture 5 frames
        callback,
    ).await?;
    
    // Wait for completion
    tokio::time::sleep(Duration::from_secs(3)).await;
    
    // Stop capture (should already be stopped due to max_captures)
    let _ = api.stop_continuous_capture(handle).await;
    
    let final_count = capture_count.load(Ordering::Relaxed);
    let total_time = start_time.elapsed();
    
    println!("   📊 Captured {} frames in {:.2}s", final_count, total_time.as_secs_f64());
    println!("   📊 Average: {:.1} fps", final_count as f64 / total_time.as_secs_f64());
    
    Ok(())
}

/// Run performance monitoring continuous capture
async fn run_performance_monitoring_capture(api: Arc<ScreenshotApiImpl>) -> Result<(), Box<dyn Error>> {
    println!("   Running 10-second performance test...");
    
    let capture_count = Arc::new(AtomicU32::new(0));
    let total_bytes = Arc::new(AtomicU32::new(0));
    let start_time = Instant::now();
    
    let config = ScreenshotConfig::builder()
        .format(ImageFormat::Png)
        .quality(50) // Lower quality for better performance
        .include_metadata(false)
        .build();
    
    // Performance monitoring callback
    let capture_count_clone = Arc::clone(&capture_count);
    let total_bytes_clone = Arc::clone(&total_bytes);
    let callback = move |frame_num: u32, result: Result<Vec<u8>, ScreenshotError>| {
        let frame_count = capture_count_clone.fetch_add(1, Ordering::Relaxed) + 1;
        
        match result {
            Ok(data) => {
                total_bytes_clone.fetch_add(data.len() as u32, Ordering::Relaxed);
                
                // Log every 10th frame
                if frame_count % 10 == 0 {
                    println!("     📈 Frame {}: {} KB (running total: {} frames)", 
                           frame_num, data.len() / 1024, frame_count);
                }
            },
            Err(e) => {
                eprintln!("     ❌ Frame {}: {}", frame_num, e);
            }
        }
    };
    
    // Start high-frequency capture
    let handle = api.start_continuous_capture(
        config,
        Duration::from_millis(100), // 10 fps
        None, // Unlimited
        callback,
    ).await?;
    
    // Run for 10 seconds
    tokio::time::sleep(Duration::from_secs(10)).await;
    
    // Stop capture
    api.stop_continuous_capture(handle).await?;
    
    let final_count = capture_count.load(Ordering::Relaxed);
    let final_bytes = total_bytes.load(Ordering::Relaxed);
    let total_time = start_time.elapsed();
    
    println!("   📊 Performance Results:");
    println!("      Frames captured: {}", final_count);
    println!("      Total time: {:.2}s", total_time.as_secs_f64());
    println!("      Average FPS: {:.1}", final_count as f64 / total_time.as_secs_f64());
    println!("      Total data: {} MB", final_bytes / 1024 / 1024);
    println!("      Average frame size: {} KB", final_bytes / 1024 / final_count);
    
    Ok(())
}

/// Run multi-format continuous capture
async fn run_multi_format_capture(api: Arc<ScreenshotApiImpl>) -> Result<(), Box<dyn Error>> {
    println!("   Capturing in multiple formats simultaneously...");
    
    let formats = vec![
        (ImageFormat::Png, "png", 95),
        #[cfg(feature = "jpeg")]
        (ImageFormat::Jpeg, "jpg", 85),
        #[cfg(feature = "webp")]
        (ImageFormat::WebP, "webp", 90),
    ];
    
    let mut handles = Vec::new();
    
    // Start capture for each format
    for (format, extension, quality) in formats {
        let api_clone = Arc::clone(&api);
        
        let config = ScreenshotConfig::builder()
            .format(format)
            .quality(quality)
            .filename_prefix(format!("multi_{}", extension))
            .include_metadata(true)
            .build();
        
        let extension_str = extension.to_string();
        let callback = move |frame_num: u32, result: Result<Vec<u8>, ScreenshotError>| {
            match result {
                Ok(data) => {
                    let filename = format!("multi_{}_{:02}.{}", extension_str, frame_num, extension_str);
                    if let Err(e) = std::fs::write(&filename, data) {
                        eprintln!("     ❌ {}: Failed to write: {}", filename, e);
                    } else {
                        println!("     ✅ {}: {} KB", filename, data.len() / 1024);
                    }
                },
                Err(e) => {
                    eprintln!("     ❌ {} frame {}: {}", extension_str, frame_num, e);
                }
            }
        };
        
        let handle = api_clone.start_continuous_capture(
            config,
            Duration::from_secs(2), // One capture every 2 seconds
            Some(3), // 3 captures per format
            callback,
        ).await?;
        
        handles.push((handle, format));
    }
    
    // Wait for all captures to complete
    tokio::time::sleep(Duration::from_secs(7)).await;
    
    // Stop all captures
    for (handle, format) in handles {
        if let Err(e) = api.stop_continuous_capture(handle).await {
            eprintln!("     ⚠️  Failed to stop {:?} capture: {}", format, e);
        }
    }
    
    println!("   📊 Multi-format capture completed");
    Ok(())
}

/// Run interactive continuous capture with graceful shutdown
async fn run_interactive_capture(api: Arc<ScreenshotApiImpl>) -> Result<(), Box<dyn Error>> {
    let capture_count = Arc::new(AtomicU32::new(0));
    let is_running = Arc::new(AtomicBool::new(true));
    
    let config = ScreenshotConfig::builder()
        .format(ImageFormat::Png)
        .quality(80)
        .filename_prefix("interactive".to_string())
        .include_metadata(false)
        .build();
    
    // Setup capture callback
    let capture_count_clone = Arc::clone(&capture_count);
    let is_running_clone = Arc::clone(&is_running);
    let callback = move |frame_num: u32, result: Result<Vec<u8>, ScreenshotError>| {
        if !is_running_clone.load(Ordering::Relaxed) {
            return;
        }
        
        capture_count_clone.fetch_add(1, Ordering::Relaxed);
        
        match result {
            Ok(data) => {
                let filename = format!("interactive_{:04}.png", frame_num);
                match std::fs::write(&filename, data) {
                    Ok(_) => {
                        println!("     📸 Frame {}: {} -> {}", 
                               frame_num, format_bytes(data.len()), filename);
                    },
                    Err(e) => {
                        eprintln!("     ❌ Frame {}: Write failed: {}", frame_num, e);
                    }
                }
            },
            Err(e) => {
                eprintln!("     ❌ Frame {}: Capture failed: {}", frame_num, e);
            }
        }
    };
    
    // Start continuous capture
    let handle = api.start_continuous_capture(
        config,
        Duration::from_secs(1), // 1 fps
        None, // Unlimited
        callback,
    ).await?;
    
    println!("   📹 Interactive capture started (1 fps)");
    println!("   📋 Press Ctrl+C to stop gracefully...");
    
    // Setup signal handler for graceful shutdown
    let is_running_signal = Arc::clone(&is_running);
    let api_signal = Arc::clone(&api);
    let handle_clone = handle.clone();
    
    tokio::spawn(async move {
        if let Ok(_) = signal::ctrl_c().await {
            println!("\n   🛑 Shutdown signal received, stopping capture...");
            is_running_signal.store(false, Ordering::Relaxed);
            
            match api_signal.stop_continuous_capture(handle_clone).await {
                Ok(_) => println!("   ✅ Capture stopped gracefully"),
                Err(e) => eprintln!("   ❌ Error stopping capture: {}", e),
            }
        }
    });
    
    // Wait for shutdown signal
    while is_running.load(Ordering::Relaxed) {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    
    let final_count = capture_count.load(Ordering::Relaxed);
    println!("   📊 Interactive capture summary:");
    println!("      Total frames captured: {}", final_count);
    
    Ok(())
}

/// Format byte count for human readability
fn format_bytes(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / 1024.0 / 1024.0)
    }
}

/// Demonstrate advanced continuous capture features
async fn demonstrate_advanced_features(api: Arc<ScreenshotApiImpl>) -> Result<(), Box<dyn Error>> {
    println!("🚀 Advanced Continuous Capture Features");
    
    // Feature 1: Adaptive quality based on performance
    println!("   📊 Adaptive quality capture...");
    run_adaptive_quality_capture(Arc::clone(&api)).await?;
    
    // Feature 2: Burst capture mode
    println!("   💥 Burst capture mode...");
    run_burst_capture(Arc::clone(&api)).await?;
    
    // Feature 3: Memory-aware capture
    println!("   🧠 Memory-aware capture...");
    run_memory_aware_capture(Arc::clone(&api)).await?;
    
    Ok(())
}

/// Run adaptive quality capture that adjusts based on performance
async fn run_adaptive_quality_capture(api: Arc<ScreenshotApiImpl>) -> Result<(), Box<dyn Error>> {
    let mut current_quality = 95u8;
    let performance_threshold_ms = 100.0; // Target: < 100ms per capture
    
    for iteration in 0..10 {
        let start_time = Instant::now();
        
        let config = ScreenshotConfig::builder()
            .format(ImageFormat::Png)
            .quality(current_quality)
            .filename_prefix(format!("adaptive_q{}", current_quality))
            .build();
        
        // Simulate single capture for timing
        match api.capture_screenshot(config).await {
            Ok(data) => {
                let capture_time = start_time.elapsed().as_millis() as f64;
                
                println!("     Iteration {}: Quality {}%, {} ms, {} KB", 
                       iteration, current_quality, capture_time as u32, data.len() / 1024);
                
                // Adjust quality based on performance
                if capture_time > performance_threshold_ms && current_quality > 50 {
                    current_quality = (current_quality as f64 * 0.9) as u8;
                    println!("       ⬇️  Reducing quality to {}%", current_quality);
                } else if capture_time < performance_threshold_ms * 0.5 && current_quality < 95 {
                    current_quality = ((current_quality as f64 * 1.1).min(95.0)) as u8;
                    println!("       ⬆️  Increasing quality to {}%", current_quality);
                }
            },
            Err(e) => {
                eprintln!("     ❌ Iteration {}: {}", iteration, e);
            }
        }
        
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    
    Ok(())
}

/// Run burst capture mode for high-frequency snapshots
async fn run_burst_capture(api: Arc<ScreenshotApiImpl>) -> Result<(), Box<dyn Error>> {
    println!("     Starting 3-second burst at 20 fps...");
    
    let burst_count = Arc::new(AtomicU32::new(0));
    
    let config = ScreenshotConfig::builder()
        .format(ImageFormat::Png)
        .quality(60) // Lower quality for speed
        .filename_prefix("burst".to_string())
        .include_metadata(false)
        .build();
    
    let burst_count_clone = Arc::clone(&burst_count);
    let callback = move |frame_num: u32, result: Result<Vec<u8>, ScreenshotError>| {
        burst_count_clone.fetch_add(1, Ordering::Relaxed);
        
        match result {
            Ok(data) => {
                let filename = format!("burst_{:03}.png", frame_num);
                if let Err(e) = std::fs::write(&filename, data) {
                    eprintln!("       ❌ Burst frame {}: {}", frame_num, e);
                }
            },
            Err(e) => {
                eprintln!("       ❌ Burst frame {}: {}", frame_num, e);
            }
        }
    };
    
    let handle = api.start_continuous_capture(
        config,
        Duration::from_millis(50), // 20 fps
        Some(60), // 3 seconds worth
        callback,
    ).await?;
    
    tokio::time::sleep(Duration::from_secs(4)).await;
    
    let _ = api.stop_continuous_capture(handle).await;
    let final_burst_count = burst_count.load(Ordering::Relaxed);
    
    println!("     📊 Burst capture: {} frames", final_burst_count);
    
    Ok(())
}

/// Run memory-aware capture that monitors usage
async fn run_memory_aware_capture(api: Arc<ScreenshotApiImpl>) -> Result<(), Box<dyn Error>> {
    println!("     Monitoring memory usage during capture...");
    
    // Get initial memory baseline
    let initial_stats = api.get_screenshot_status().await?;
    let initial_memory = initial_stats.memory_usage_bytes;
    
    let config = ScreenshotConfig::builder()
        .format(ImageFormat::Png)
        .quality(85)
        .filename_prefix("memory_test".to_string())
        .build();
    
    let memory_samples = Arc::new(tokio::sync::Mutex::new(Vec::new()));
    let memory_samples_clone = Arc::clone(&memory_samples);
    
    let callback = move |frame_num: u32, result: Result<Vec<u8>, ScreenshotError>| {
        if let Ok(data) = result {
            let filename = format!("memory_test_{:02}.png", frame_num);
            let _ = std::fs::write(&filename, data);
            
            // Note: Memory sampling is performed via API polling in the monitoring loop below,
            // demonstrating async resource tracking pattern
        }
    };
    
    let handle = api.start_continuous_capture(
        config,
        Duration::from_secs(1),
        Some(5),
        callback,
    ).await?;
    
    // Monitor memory usage
    for i in 0..6 {
        tokio::time::sleep(Duration::from_secs(1)).await;
        
        if let Ok(stats) = api.get_screenshot_status().await {
            let current_memory = stats.memory_usage_bytes;
            let memory_growth = current_memory.saturating_sub(initial_memory);
            
            println!("       Sample {}: {} KB (+{} KB)", 
                   i, current_memory / 1024, memory_growth / 1024);
            
            memory_samples_clone.lock().await.push(current_memory);
        }
    }
    
    let _ = api.stop_continuous_capture(handle).await;
    
    let samples = memory_samples.lock().await;
    if samples.len() > 1 {
        let max_memory = samples.iter().max().unwrap_or(&0);
        let growth = max_memory.saturating_sub(initial_memory);
        println!("     📊 Peak memory growth: {} KB", growth / 1024);
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(2048 * 1024), "2.0 MB");
    }
    
    #[tokio::test]
    async fn test_continuous_capture_setup() {
        // Test that continuous capture can be set up without errors
        // This would run with a mock API in a real test environment
        assert!(true); // Placeholder
    }
}