// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Format Comparison Example
//!
//! Demonstrates PNG, JPEG, and WebP output with quality settings comparison.
//! Shows file size differences, encoding performance, and visual quality trade-offs.

use blitz_shell::{BlitzShell, ScreenshotApi, ScreenshotResult};
use std::time::{Duration, Instant};
use std::path::{Path, PathBuf};
use std::fs;

/// Format configuration for comparison testing
#[derive(Debug, Clone)]
struct FormatConfig {
    name: &'static str,
    extension: &'static str,
    qualities: Vec<u8>,
    supports_transparency: bool,
    lossless_available: bool,
}

/// Results from format comparison testing
#[derive(Debug)]
struct ComparisonResult {
    format: String,
    quality: Option<u8>,
    file_size: u64,
    encode_time_ms: u64,
    success: bool,
    error_message: Option<String>,
}

/// Format comparison runner
struct FormatComparison {
    shell: BlitzShell,
    output_dir: PathBuf,
    test_content: String,
}

impl FormatComparison {
    /// Create new format comparison runner
    fn new() -> ScreenshotResult<Self> {
        let shell = BlitzShell::new()?;
        let output_dir = PathBuf::from("screenshot_format_comparison");
        
        // Create output directory
        if !output_dir.exists() {
            fs::create_dir_all(&output_dir)
                .map_err(|e| format!("Failed to create output directory: {}", e))?;
        }
        
        let test_content = Self::generate_test_content();
        
        Ok(Self {
            shell,
            output_dir,
            test_content,
        })
    }
    
    /// Generate rich test content for comparison
    fn generate_test_content() -> String {
        r#"<!DOCTYPE html>
<html>
<head>
    <style>
        body {
            font-family: Arial, sans-serif;
            background: linear-gradient(45deg, #ff6b6b, #4ecdc4, #45b7d1, #96ceb4);
            margin: 0;
            padding: 20px;
        }
        .container {
            background: white;
            border-radius: 10px;
            padding: 30px;
            box-shadow: 0 10px 30px rgba(0,0,0,0.3);
            max-width: 800px;
            margin: 0 auto;
        }
        .header {
            color: #2c3e50;
            text-align: center;
            margin-bottom: 30px;
            text-shadow: 2px 2px 4px rgba(0,0,0,0.1);
        }
        .color-grid {
            display: grid;
            grid-template-columns: repeat(4, 1fr);
            gap: 10px;
            margin: 20px 0;
        }
        .color-box {
            height: 80px;
            border-radius: 8px;
            display: flex;
            align-items: center;
            justify-content: center;
            color: white;
            font-weight: bold;
            text-shadow: 1px 1px 2px rgba(0,0,0,0.5);
        }
        .transparency-demo {
            position: relative;
            background: url('data:image/svg+xml,<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 20 20"><rect width="10" height="10" fill="%23ccc"/><rect x="10" y="10" width="10" height="10" fill="%23ccc"/></svg>') repeat;
            padding: 20px;
            border-radius: 8px;
            margin: 20px 0;
        }
        .semi-transparent {
            background: rgba(255, 99, 71, 0.7);
            padding: 15px;
            border-radius: 5px;
            color: white;
            text-align: center;
        }
        .text-demo {
            column-count: 2;
            column-gap: 30px;
            text-align: justify;
            line-height: 1.6;
            color: #333;
        }
    </style>
</head>
<body>
    <div class="container">
        <h1 class="header">Screenshot Format Comparison Test</h1>
        
        <div class="color-grid">
            <div class="color-box" style="background: #e74c3c;">Red</div>
            <div class="color-box" style="background: #3498db;">Blue</div>
            <div class="color-box" style="background: #2ecc71;">Green</div>
            <div class="color-box" style="background: #f39c12;">Orange</div>
            <div class="color-box" style="background: #9b59b6;">Purple</div>
            <div class="color-box" style="background: #1abc9c;">Teal</div>
            <div class="color-box" style="background: #34495e;">Dark</div>
            <div class="color-box" style="background: #ecf0f1;">Light</div>
        </div>
        
        <div class="transparency-demo">
            <div class="semi-transparent">
                This demonstrates transparency support - important for WebP and PNG
            </div>
        </div>
        
        <div class="text-demo">
            <p>This content tests how different image formats handle various visual elements. PNG excels at preserving sharp text and graphics with transparency support, making it ideal for screenshots with UI elements.</p>
            
            <p>JPEG compression works best for photographic content and gradients, offering smaller file sizes at the cost of some quality loss. The quality parameter directly affects the compression level.</p>
            
            <p>WebP combines the best of both worlds, supporting both lossy and lossless compression modes, transparency, and often achieving smaller file sizes than PNG or JPEG while maintaining superior quality.</p>
            
            <p>For screenshot applications, the choice depends on content type, file size requirements, and browser/viewer compatibility considerations.</p>
        </div>
    </div>
</body>
</html>"#.to_string()
    }
    
    /// Get supported format configurations
    fn get_format_configs() -> Vec<FormatConfig> {
        vec![
            FormatConfig {
                name: "PNG",
                extension: "png",
                qualities: vec![], // PNG is lossless only
                supports_transparency: true,
                lossless_available: true,
            },
            FormatConfig {
                name: "JPEG",
                extension: "jpg",
                qualities: vec![10, 30, 50, 70, 85, 95],
                supports_transparency: false,
                lossless_available: false,
            },
            FormatConfig {
                name: "WebP",
                extension: "webp",
                qualities: vec![10, 30, 50, 70, 85, 95, 100], // 100 = lossless
                supports_transparency: true,
                lossless_available: true,
            },
        ]
    }
    
    /// Run comprehensive format comparison
    async fn run_comparison(&mut self) -> ScreenshotResult<()> {
        println!("🎨 Starting format comparison...");
        println!("📁 Output directory: {}", self.output_dir.display());
        
        // Load test content into browser
        self.shell.load_html(&self.test_content).await?;
        
        // Wait for content to render
        tokio::time::sleep(Duration::from_millis(1000)).await;
        
        let mut results = Vec::new();
        let format_configs = Self::get_format_configs();
        
        // Test each format configuration
        for config in &format_configs {
            println!("\n📸 Testing {} format...", config.name);
            
            if config.qualities.is_empty() {
                // Lossless format (PNG)
                let result = self.capture_with_format(config, None).await;
                results.push(result);
            } else {
                // Quality-based format (JPEG, WebP)
                for &quality in &config.qualities {
                    let result = self.capture_with_format(config, Some(quality)).await;
                    results.push(result);
                }
            }
        }
        
        // Generate comparison report
        self.generate_report(&results)?;
        
        println!("\n✅ Format comparison completed!");
        println!("📊 Check comparison_report.txt for detailed analysis");
        
        Ok(())
    }
    
    /// Capture screenshot with specific format and quality
    async fn capture_with_format(
        &self,
        config: &FormatConfig,
        quality: Option<u8>,
    ) -> ComparisonResult {
        let quality_str = match quality {
            Some(q) if q == 100 && config.lossless_available => "lossless".to_string(),
            Some(q) => format!("q{}", q),
            None => "lossless".to_string(),
        };
        
        let filename = format!(
            "test_{}_{}.{}",
            config.name.to_lowercase(),
            quality_str,
            config.extension
        );
        
        let filepath = self.output_dir.join(&filename);
        
        println!("  📷 Capturing {} (quality: {})", filename, quality_str);
        
        let start_time = Instant::now();
        
        // Configure screenshot request
        let mut screenshot_config = self.shell.screenshot_config_builder()
            .format(match config.name {
                "PNG" => "png",
                "JPEG" => "jpeg", 
                "WebP" => "webp",
                _ => "png",
            });
        
        if let Some(q) = quality {
            screenshot_config = screenshot_config.quality(q);
        }
        
        // Capture screenshot
        let capture_result = self.shell.capture_screenshot(
            screenshot_config.build()
        ).await;
        
        let encode_time = start_time.elapsed();
        
        match capture_result {
            Ok(image_data) => {
                // Save to file
                match fs::write(&filepath, &image_data) {
                    Ok(()) => {
                        let file_size = image_data.len() as u64;
                        println!("    ✅ Saved {} ({} bytes, {}ms)", 
                               filename, file_size, encode_time.as_millis());
                        
                        ComparisonResult {
                            format: format!("{}{}", 
                                config.name,
                                quality.map(|q| format!(" Q{}", q)).unwrap_or_default()
                            ),
                            quality,
                            file_size,
                            encode_time_ms: encode_time.as_millis() as u64,
                            success: true,
                            error_message: None,
                        }
                    }
                    Err(e) => {
                        println!("    ❌ Failed to save {}: {}", filename, e);
                        ComparisonResult {
                            format: format!("{}{}", 
                                config.name,
                                quality.map(|q| format!(" Q{}", q)).unwrap_or_default()
                            ),
                            quality,
                            file_size: 0,
                            encode_time_ms: encode_time.as_millis() as u64,
                            success: false,
                            error_message: Some(format!("Save failed: {}", e)),
                        }
                    }
                }
            }
            Err(e) => {
                println!("    ❌ Capture failed for {}: {}", filename, e);
                ComparisonResult {
                    format: format!("{}{}", 
                        config.name,
                        quality.map(|q| format!(" Q{}", q)).unwrap_or_default()
                    ),
                    quality,
                    file_size: 0,
                    encode_time_ms: encode_time.as_millis() as u64,
                    success: false,
                    error_message: Some(format!("Capture failed: {}", e)),
                }
            }
        }
    }
    
    /// Generate detailed comparison report
    fn generate_report(&self, results: &[ComparisonResult]) -> ScreenshotResult<()> {
        let report_path = self.output_dir.join("comparison_report.txt");
        let mut report = String::new();
        
        report.push_str("📊 SCREENSHOT FORMAT COMPARISON REPORT\n");
        report.push_str("=====================================\n\n");
        
        // Summary statistics
        let successful_results: Vec<_> = results.iter().filter(|r| r.success).collect();
        let failed_results: Vec<_> = results.iter().filter(|r| !r.success).collect();
        
        report.push_str(&format!("Total tests: {}\n", results.len()));
        report.push_str(&format!("Successful: {}\n", successful_results.len()));
        report.push_str(&format!("Failed: {}\n\n", failed_results.len()));
        
        // Detailed results table
        report.push_str("DETAILED RESULTS:\n");
        report.push_str("Format          Quality  File Size (KB)  Encode Time (ms)  Status\n");
        report.push_str("----------------------------------------------------------------\n");
        
        for result in results {
            let status = if result.success { "✅ OK" } else { "❌ FAIL" };
            let quality_str = result.quality.map(|q| q.to_string()).unwrap_or_else(|| "-".to_string());
            
            report.push_str(&format!(
                "{:<15} {:<7}  {:<14}  {:<16}  {}\n",
                result.format,
                quality_str,
                if result.success { format!("{:.1}", result.file_size as f64 / 1024.0) } else { "-".to_string() },
                if result.success { result.encode_time_ms.to_string() } else { "-".to_string() },
                status
            ));
        }
        
        // Analysis by format
        report.push_str("\n\nANALYSIS BY FORMAT:\n");
        report.push_str("==================\n");
        
        let png_results: Vec<_> = successful_results.iter().filter(|r| r.format.starts_with("PNG")).collect();
        let jpeg_results: Vec<_> = successful_results.iter().filter(|r| r.format.starts_with("JPEG")).collect();
        let webp_results: Vec<_> = successful_results.iter().filter(|r| r.format.starts_with("WebP")).collect();
        
        if !png_results.is_empty() {
            report.push_str("\nPNG (Lossless):\n");
            for result in &png_results {
                report.push_str(&format!(
                    "  File size: {:.1} KB, Encode time: {} ms\n",
                    result.file_size as f64 / 1024.0,
                    result.encode_time_ms
                ));
            }
        }
        
        if !jpeg_results.is_empty() {
            report.push_str("\nJPEG (Lossy):\n");
            let min_size = jpeg_results.iter().map(|r| r.file_size).min().unwrap_or(0) as f64 / 1024.0;
            let max_size = jpeg_results.iter().map(|r| r.file_size).max().unwrap_or(0) as f64 / 1024.0;
            let avg_encode_time = jpeg_results.iter().map(|r| r.encode_time_ms).sum::<u64>() as f64 / jpeg_results.len() as f64;
            
            report.push_str(&format!(
                "  Size range: {:.1} - {:.1} KB\n  Average encode time: {:.1} ms\n",
                min_size, max_size, avg_encode_time
            ));
        }
        
        if !webp_results.is_empty() {
            report.push_str("\nWebP (Lossy + Lossless):\n");
            let min_size = webp_results.iter().map(|r| r.file_size).min().unwrap_or(0) as f64 / 1024.0;
            let max_size = webp_results.iter().map(|r| r.file_size).max().unwrap_or(0) as f64 / 1024.0;
            let avg_encode_time = webp_results.iter().map(|r| r.encode_time_ms).sum::<u64>() as f64 / webp_results.len() as f64;
            
            report.push_str(&format!(
                "  Size range: {:.1} - {:.1} KB\n  Average encode time: {:.1} ms\n",
                min_size, max_size, avg_encode_time
            ));
        }
        
        // Recommendations
        report.push_str("\n\nRECOMMENDATIONS:\n");
        report.push_str("===============\n");
        
        if let Some(smallest) = successful_results.iter().min_by_key(|r| r.file_size) {
            report.push_str(&format!(
                "• Smallest file: {} ({:.1} KB)\n",
                smallest.format,
                smallest.file_size as f64 / 1024.0
            ));
        }
        
        if let Some(fastest) = successful_results.iter().min_by_key(|r| r.encode_time_ms) {
            report.push_str(&format!(
                "• Fastest encoding: {} ({} ms)\n",
                fastest.format,
                fastest.encode_time_ms
            ));
        }
        
        report.push_str("\n• PNG: Best for UI screenshots with transparency\n");
        report.push_str("• JPEG: Best for photographic content, small files\n");
        report.push_str("• WebP: Best overall balance of size and quality\n");
        
        // Failure analysis
        if !failed_results.is_empty() {
            report.push_str("\n\nFAILURES:\n");
            report.push_str("========\n");
            for result in &failed_results {
                if let Some(ref error) = result.error_message {
                    report.push_str(&format!("• {}: {}\n", result.format, error));
                }
            }
        }
        
        report.push_str(&format!("\nGenerated: {}\n", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")));
        
        fs::write(&report_path, report)
            .map_err(|e| format!("Failed to write report: {}", e))?;
        
        Ok(())
    }
}

/// Display usage information
fn print_usage() {
    println!("🎨 Screenshot Format Comparison Tool");
    println!();
    println!("This example demonstrates the differences between PNG, JPEG, and WebP");
    println!("formats for screenshot capture, including:");
    println!("• File size comparisons across quality levels");
    println!("• Encoding performance measurements");
    println!("• Visual quality trade-offs");
    println!("• Format-specific features (transparency, lossless modes)");
    println!();
    println!("Output files will be saved to 'screenshot_format_comparison/' directory");
    println!("A detailed comparison report will be generated as 'comparison_report.txt'");
    println!();
    println!("Usage: cargo run --example format_comparison");
}

/// Validate output files and show summary
fn validate_and_summarize(output_dir: &Path) -> ScreenshotResult<()> {
    if !output_dir.exists() {
        return Err("Output directory not found".into());
    }
    
    let mut file_count = 0;
    let mut total_size = 0u64;
    let mut formats = std::collections::HashMap::new();
    
    for entry in fs::read_dir(output_dir).map_err(|e| format!("Failed to read output directory: {}", e))? {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let path = entry.path();
        
        if path.is_file() {
            if let Some(extension) = path.extension().and_then(|e| e.to_str()) {
                if matches!(extension, "png" | "jpg" | "jpeg" | "webp") {
                    file_count += 1;
                    if let Ok(metadata) = fs::metadata(&path) {
                        total_size += metadata.len();
                        *formats.entry(extension.to_uppercase()).or_insert(0u32) += 1;
                    }
                }
            }
        }
    }
    
    println!("\n📊 COMPARISON SUMMARY:");
    println!("Files generated: {}", file_count);
    println!("Total size: {:.1} KB", total_size as f64 / 1024.0);
    
    for (format, count) in formats {
        println!("  {}: {} files", format, count);
    }
    
    Ok(())
}

#[tokio::main]
async fn main() -> ScreenshotResult<()> {
    print_usage();
    
    let mut comparison = FormatComparison::new()?;
    
    println!("🚀 Starting format comparison...");
    
    match comparison.run_comparison().await {
        Ok(()) => {
            validate_and_summarize(&comparison.output_dir)?;
            println!("\n✅ Format comparison completed successfully!");
        }
        Err(e) => {
            eprintln!("\n❌ Format comparison failed: {}", e);
            std::process::exit(1);
        }
    }
    
    Ok(())
}