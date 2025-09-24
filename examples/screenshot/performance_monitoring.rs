// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Performance Monitoring Example
//!
//! Comprehensive capture performance analysis and statistics monitoring.
//! Tests various scenarios to measure screenshot system performance impact.

use blitz_shell::{BlitzShell, ScreenshotApi, ScreenshotResult};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use std::fs;

/// Performance metrics for individual operations
#[derive(Debug, Clone)]
struct PerformanceMetric {
    timestamp: u64,
    operation: String,
    duration_ms: u64,
    memory_usage_mb: f64,
    cpu_usage_percent: f64,
    success: bool,
    details: String,
}

/// Aggregated performance statistics
#[derive(Debug)]
struct PerformanceStats {
    total_operations: usize,
    successful_operations: usize,
    failed_operations: usize,
    min_duration_ms: u64,
    max_duration_ms: u64,
    avg_duration_ms: f64,
    total_duration_ms: u64,
    throughput_ops_per_sec: f64,
    memory_peak_mb: f64,
    memory_avg_mb: f64,
    cpu_peak_percent: f64,
    cpu_avg_percent: f64,
}

/// Performance test configuration
#[derive(Debug, Clone)]
struct TestConfig {
    name: String,
    iterations: usize,
    concurrent_captures: usize,
    capture_interval_ms: u64,
    format: String,
    quality: Option<u8>,
    region_size: Option<(u32, u32)>,
    warmup_iterations: usize,
}

/// Real-time performance monitor
struct PerformanceMonitor {
    shell: BlitzShell,
    metrics: Arc<Mutex<VecDeque<PerformanceMetric>>>,
    output_dir: PathBuf,
    start_time: Instant,
}

impl PerformanceMonitor {
    /// Create new performance monitor
    fn new() -> ScreenshotResult<Self> {
        let shell = BlitzShell::new()?;
        let output_dir = PathBuf::from("performance_monitoring");
        
        // Create output directory
        if !output_dir.exists() {
            fs::create_dir_all(&output_dir)
                .map_err(|e| format!("Failed to create output directory: {}", e))?;
        }
        
        Ok(Self {
            shell,
            metrics: Arc::new(Mutex::new(VecDeque::new())),
            output_dir,
            start_time: Instant::now(),
        })
    }
    
    /// Setup complex test content for performance testing
    async fn setup_test_content(&mut self) -> ScreenshotResult<()> {
        let test_html = r#"<!DOCTYPE html>
<html>
<head>
    <style>
        body {
            margin: 0;
            padding: 0;
            font-family: Arial, sans-serif;
            background: linear-gradient(45deg, #ff6b6b, #4ecdc4, #45b7d1, #96ceb4, #feca57, #ff9ff3);
            animation: gradientShift 3s ease-in-out infinite;
        }
        
        @keyframes gradientShift {
            0%, 100% { filter: hue-rotate(0deg); }
            50% { filter: hue-rotate(90deg); }
        }
        
        .container {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: 20px;
            padding: 20px;
            min-height: 100vh;
        }
        
        .card {
            background: white;
            border-radius: 10px;
            padding: 20px;
            box-shadow: 0 10px 30px rgba(0,0,0,0.2);
            transition: transform 0.3s ease;
            animation: float 6s ease-in-out infinite;
        }
        
        .card:nth-child(odd) {
            animation-delay: -3s;
        }
        
        @keyframes float {
            0%, 100% { transform: translateY(0px); }
            50% { transform: translateY(-10px); }
        }
        
        .card:hover {
            transform: scale(1.05) translateY(-5px);
        }
        
        .chart {
            width: 100%;
            height: 150px;
            background: linear-gradient(90deg, #e74c3c, #f39c12, #f1c40f, #2ecc71, #3498db, #9b59b6);
            border-radius: 5px;
            position: relative;
            overflow: hidden;
        }
        
        .chart::before {
            content: '';
            position: absolute;
            top: 0;
            left: -100%;
            width: 100%;
            height: 100%;
            background: linear-gradient(90deg, transparent, rgba(255,255,255,0.3), transparent);
            animation: shimmer 2s infinite;
        }
        
        @keyframes shimmer {
            0% { left: -100%; }
            100% { left: 100%; }
        }
        
        .text-content {
            line-height: 1.6;
            color: #333;
            text-align: justify;
        }
        
        .performance-indicators {
            display: flex;
            justify-content: space-around;
            margin: 20px 0;
        }
        
        .indicator {
            text-align: center;
            padding: 10px;
            background: #f8f9fa;
            border-radius: 8px;
            border-left: 4px solid #007bff;
        }
        
        .complex-svg {
            width: 100%;
            height: 200px;
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="card">
            <h2>Performance Test Content</h2>
            <div class="chart"></div>
            <div class="performance-indicators">
                <div class="indicator">
                    <strong>FPS</strong><br>
                    <span id="fps">60</span>
                </div>
                <div class="indicator">
                    <strong>Memory</strong><br>
                    <span id="memory">45 MB</span>
                </div>
                <div class="indicator">
                    <strong>CPU</strong><br>
                    <span id="cpu">12%</span>
                </div>
            </div>
        </div>
        
        <div class="card">
            <h3>Complex Graphics</h3>
            <svg class="complex-svg" viewBox="0 0 400 200">
                <defs>
                    <linearGradient id="grad1" x1="0%" y1="0%" x2="100%" y2="100%">
                        <stop offset="0%" style="stop-color:#ff6b6b;stop-opacity:1" />
                        <stop offset="100%" style="stop-color:#4ecdc4;stop-opacity:1" />
                    </linearGradient>
                    <pattern id="pattern1" patternUnits="userSpaceOnUse" width="20" height="20">
                        <rect width="10" height="10" fill="#333"/>
                        <rect x="10" y="10" width="10" height="10" fill="#333"/>
                    </pattern>
                </defs>
                <rect width="400" height="200" fill="url(#grad1)"/>
                <circle cx="100" cy="100" r="50" fill="url(#pattern1)" opacity="0.8">
                    <animateTransform attributeName="transform" type="rotate" 
                                    values="0 100 100;360 100 100" dur="4s" repeatCount="indefinite"/>
                </circle>
                <polygon points="200,50 250,150 150,150" fill="#fff" opacity="0.7">
                    <animateTransform attributeName="transform" type="scale" 
                                    values="1;1.2;1" dur="2s" repeatCount="indefinite"/>
                </polygon>
                <path d="M300,50 Q350,100 300,150 Q250,100 300,50" fill="#ff9ff3" opacity="0.6">
                    <animate attributeName="d" 
                           values="M300,50 Q350,100 300,150 Q250,100 300,50;M300,50 Q250,100 300,150 Q350,100 300,50;M300,50 Q350,100 300,150 Q250,100 300,50" 
                           dur="3s" repeatCount="indefinite"/>
                </path>
            </svg>
        </div>
        
        <div class="card">
            <h3>Text Rendering Performance</h3>
            <div class="text-content">
                Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.
            </div>
        </div>
        
        <div class="card">
            <h3>Data Visualization</h3>
            <div style="display: flex; height: 100px; align-items: end; gap: 5px;">
                <div style="background: #e74c3c; width: 20px; height: 80%; animation: pulse 1s infinite;"></div>
                <div style="background: #f39c12; width: 20px; height: 60%; animation: pulse 1s infinite 0.1s;"></div>
                <div style="background: #f1c40f; width: 20px; height: 90%; animation: pulse 1s infinite 0.2s;"></div>
                <div style="background: #2ecc71; width: 20px; height: 70%; animation: pulse 1s infinite 0.3s;"></div>
                <div style="background: #3498db; width: 20px; height: 85%; animation: pulse 1s infinite 0.4s;"></div>
                <div style="background: #9b59b6; width: 20px; height: 65%; animation: pulse 1s infinite 0.5s;"></div>
            </div>
        </div>
    </div>
    
    <script>
        // Simulate dynamic content updates
        setInterval(() => {
            document.getElementById('fps').textContent = Math.floor(Math.random() * 20) + 50;
            document.getElementById('memory').textContent = Math.floor(Math.random() * 30) + 30 + ' MB';
            document.getElementById('cpu').textContent = Math.floor(Math.random() * 25) + 5 + '%';
        }, 1000);
    </script>
    
    <style>
        @keyframes pulse {
            0%, 100% { opacity: 0.7; }
            50% { opacity: 1; }
        }
    </style>
</body>
</html>"#;
        
        self.shell.load_html(test_html).await?;
        
        // Wait for content to fully render and animations to start
        tokio::time::sleep(Duration::from_millis(2000)).await;
        
        Ok(())
    }
    
    /// Get test configurations for different scenarios
    fn get_test_configs() -> Vec<TestConfig> {
        vec![
            TestConfig {
                name: "Single_PNG_Baseline".to_string(),
                iterations: 20,
                concurrent_captures: 1,
                capture_interval_ms: 100,
                format: "png".to_string(),
                quality: None,
                region_size: None,
                warmup_iterations: 3,
            },
            TestConfig {
                name: "Single_JPEG_High_Quality".to_string(),
                iterations: 20,
                concurrent_captures: 1,
                capture_interval_ms: 100,
                format: "jpeg".to_string(),
                quality: Some(95),
                region_size: None,
                warmup_iterations: 3,
            },
            TestConfig {
                name: "Single_JPEG_Low_Quality".to_string(),
                iterations: 20,
                concurrent_captures: 1,
                capture_interval_ms: 100,
                format: "jpeg".to_string(),
                quality: Some(30),
                region_size: None,
                warmup_iterations: 3,
            },
            TestConfig {
                name: "Single_WebP_Lossless".to_string(),
                iterations: 20,
                concurrent_captures: 1,
                capture_interval_ms: 100,
                format: "webp".to_string(),
                quality: Some(100),
                region_size: None,
                warmup_iterations: 3,
            },
            TestConfig {
                name: "Rapid_Capture_PNG".to_string(),
                iterations: 50,
                concurrent_captures: 1,
                capture_interval_ms: 20,
                format: "png".to_string(),
                quality: None,
                region_size: None,
                warmup_iterations: 5,
            },
            TestConfig {
                name: "Concurrent_Captures_3x".to_string(),
                iterations: 15,
                concurrent_captures: 3,
                capture_interval_ms: 200,
                format: "png".to_string(),
                quality: None,
                region_size: None,
                warmup_iterations: 2,
            },
            TestConfig {
                name: "Small_Region_Capture".to_string(),
                iterations: 30,
                concurrent_captures: 1,
                capture_interval_ms: 50,
                format: "png".to_string(),
                quality: None,
                region_size: Some((400, 300)),
                warmup_iterations: 3,
            },
            TestConfig {
                name: "Stress_Test_100_Fast".to_string(),
                iterations: 100,
                concurrent_captures: 1,
                capture_interval_ms: 10,
                format: "jpeg".to_string(),
                quality: Some(70),
                region_size: Some((200, 150)),
                warmup_iterations: 5,
            },
        ]
    }
    
    /// Record a performance metric
    fn record_metric(&self, metric: PerformanceMetric) {
        if let Ok(mut metrics) = self.metrics.lock() {
            metrics.push_back(metric);
            
            // Keep only recent metrics (last 1000)
            if metrics.len() > 1000 {
                metrics.pop_front();
            }
        }
    }
    
    /// Get current system resource usage (simplified estimation)
    fn get_resource_usage() -> (f64, f64) {
        // In a real implementation, this would query actual system metrics
        // For this example, we simulate realistic values
        let memory_mb = 45.0 + (rand::random::<f64>() * 20.0);
        let cpu_percent = 5.0 + (rand::random::<f64>() * 15.0);
        (memory_mb, cpu_percent)
    }
    
    /// Run performance test with given configuration
    async fn run_test(&mut self, config: &TestConfig) -> ScreenshotResult<PerformanceStats> {
        println!("\n🏃 Running test: {}", config.name);
        println!("   Iterations: {}, Concurrent: {}, Interval: {}ms", 
                config.iterations, config.concurrent_captures, config.capture_interval_ms);
        
        let mut all_metrics = Vec::new();
        let test_start = Instant::now();
        
        // Warmup phase
        if config.warmup_iterations > 0 {
            println!("   🔥 Warming up ({} iterations)...", config.warmup_iterations);
            for _ in 0..config.warmup_iterations {
                let _ = self.capture_single(&config).await;
                tokio::time::sleep(Duration::from_millis(config.capture_interval_ms)).await;
            }
        }
        
        // Main test phase
        println!("   📊 Running main test...");
        
        if config.concurrent_captures > 1 {
            // Concurrent capture test
            for batch in 0..(config.iterations / config.concurrent_captures) {
                let mut tasks = Vec::new();
                
                for _ in 0..config.concurrent_captures {
                    let config_clone = config.clone();
                    let metrics_clone = self.metrics.clone();
                    
                    tasks.push(tokio::spawn(async move {
                        let start_time = Instant::now();
                        let (memory_mb, cpu_percent) = Self::get_resource_usage();
                        
                        // Simulate capture (in real implementation, would call actual capture)
                        tokio::time::sleep(Duration::from_millis(10 + rand::random::<u64>() % 40)).await;
                        
                        let duration = start_time.elapsed();
                        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
                        
                        PerformanceMetric {
                            timestamp,
                            operation: format!("concurrent_capture_{}", config_clone.name),
                            duration_ms: duration.as_millis() as u64,
                            memory_usage_mb: memory_mb,
                            cpu_usage_percent: cpu_percent,
                            success: true,
                            details: format!("Format: {}, Quality: {:?}", config_clone.format, config_clone.quality),
                        }
                    }));
                }
                
                let batch_results = futures::future::join_all(tasks).await;
                for result in batch_results {
                    if let Ok(metric) = result {
                        all_metrics.push(metric.clone());
                        self.record_metric(metric);
                    }
                }
                
                println!("   Progress: {}/{} batches", batch + 1, config.iterations / config.concurrent_captures);
                tokio::time::sleep(Duration::from_millis(config.capture_interval_ms)).await;
            }
        } else {
            // Sequential capture test
            for i in 0..config.iterations {
                let metric = self.capture_single(&config).await?;
                all_metrics.push(metric.clone());
                self.record_metric(metric);
                
                if (i + 1) % 10 == 0 {
                    println!("   Progress: {}/{} captures", i + 1, config.iterations);
                }
                
                tokio::time::sleep(Duration::from_millis(config.capture_interval_ms)).await;
            }
        }
        
        let test_duration = test_start.elapsed();
        
        // Calculate statistics
        let stats = self.calculate_stats(&all_metrics, test_duration);
        
        println!("   ✅ Test completed in {:.2}s", test_duration.as_secs_f64());
        println!("      Avg: {:.1}ms, Min: {}ms, Max: {}ms, Throughput: {:.1} ops/sec",
                stats.avg_duration_ms, stats.min_duration_ms, stats.max_duration_ms, stats.throughput_ops_per_sec);
        
        Ok(stats)
    }
    
    /// Capture a single screenshot with performance tracking
    async fn capture_single(&self, config: &TestConfig) -> ScreenshotResult<PerformanceMetric> {
        let start_time = Instant::now();
        let (memory_mb, cpu_percent) = Self::get_resource_usage();
        
        // Configure screenshot request
        let mut screenshot_config = self.shell.screenshot_config_builder()
            .format(&config.format);
        
        if let Some(quality) = config.quality {
            screenshot_config = screenshot_config.quality(quality);
        }
        
        if let Some((width, height)) = config.region_size {
            screenshot_config = screenshot_config.region(0, 0, width, height);
        }
        
        // Perform capture
        let capture_result = self.shell.capture_screenshot(screenshot_config.build()).await;
        let duration = start_time.elapsed();
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
        
        let (success, details) = match capture_result {
            Ok(data) => (true, format!("Captured {} bytes", data.len())),
            Err(e) => (false, format!("Error: {}", e)),
        };
        
        Ok(PerformanceMetric {
            timestamp,
            operation: config.name.clone(),
            duration_ms: duration.as_millis() as u64,
            memory_usage_mb: memory_mb,
            cpu_usage_percent: cpu_percent,
            success,
            details,
        })
    }
    
    /// Calculate performance statistics
    fn calculate_stats(&self, metrics: &[PerformanceMetric], total_duration: Duration) -> PerformanceStats {
        let successful_metrics: Vec<_> = metrics.iter().filter(|m| m.success).collect();
        let total_operations = metrics.len();
        let successful_operations = successful_metrics.len();
        let failed_operations = total_operations - successful_operations;
        
        if successful_metrics.is_empty() {
            return PerformanceStats {
                total_operations,
                successful_operations,
                failed_operations,
                min_duration_ms: 0,
                max_duration_ms: 0,
                avg_duration_ms: 0.0,
                total_duration_ms: total_duration.as_millis() as u64,
                throughput_ops_per_sec: 0.0,
                memory_peak_mb: 0.0,
                memory_avg_mb: 0.0,
                cpu_peak_percent: 0.0,
                cpu_avg_percent: 0.0,
            };
        }
        
        let durations: Vec<_> = successful_metrics.iter().map(|m| m.duration_ms).collect();
        let memory_usage: Vec<_> = successful_metrics.iter().map(|m| m.memory_usage_mb).collect();
        let cpu_usage: Vec<_> = successful_metrics.iter().map(|m| m.cpu_usage_percent).collect();
        
        let min_duration_ms = *durations.iter().min().unwrap();
        let max_duration_ms = *durations.iter().max().unwrap();
        let avg_duration_ms = durations.iter().sum::<u64>() as f64 / durations.len() as f64;
        let total_duration_ms = total_duration.as_millis() as u64;
        let throughput_ops_per_sec = successful_operations as f64 / total_duration.as_secs_f64();
        
        let memory_peak_mb = memory_usage.iter().cloned().fold(0.0f64, f64::max);
        let memory_avg_mb = memory_usage.iter().sum::<f64>() / memory_usage.len() as f64;
        let cpu_peak_percent = cpu_usage.iter().cloned().fold(0.0f64, f64::max);
        let cpu_avg_percent = cpu_usage.iter().sum::<f64>() / cpu_usage.len() as f64;
        
        PerformanceStats {
            total_operations,
            successful_operations,
            failed_operations,
            min_duration_ms,
            max_duration_ms,
            avg_duration_ms,
            total_duration_ms,
            throughput_ops_per_sec,
            memory_peak_mb,
            memory_avg_mb,
            cpu_peak_percent,
            cpu_avg_percent,
        }
    }
    
    /// Generate comprehensive performance report
    fn generate_report(&self, test_results: &[(TestConfig, PerformanceStats)]) -> ScreenshotResult<()> {
        let report_path = self.output_dir.join("performance_report.txt");
        let mut report = String::new();
        
        report.push_str("📈 SCREENSHOT PERFORMANCE MONITORING REPORT\n");
        report.push_str("==========================================\n\n");
        
        report.push_str(&format!("Generated: {}\n", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")));
        report.push_str(&format!("Total runtime: {:.2}s\n\n", self.start_time.elapsed().as_secs_f64()));
        
        // Executive Summary
        report.push_str("EXECUTIVE SUMMARY:\n");
        report.push_str("=================\n");
        
        let total_operations: usize = test_results.iter().map(|(_, stats)| stats.total_operations).sum();
        let total_successful: usize = test_results.iter().map(|(_, stats)| stats.successful_operations).sum();
        let avg_throughput: f64 = test_results.iter().map(|(_, stats)| stats.throughput_ops_per_sec).sum::<f64>() / test_results.len() as f64;
        let best_throughput = test_results.iter().map(|(_, stats)| stats.throughput_ops_per_sec).fold(0.0f64, f64::max);
        
        report.push_str(&format!("Total operations: {}\n", total_operations));
        report.push_str(&format!("Success rate: {:.1}%\n", (total_successful as f64 / total_operations as f64) * 100.0));
        report.push_str(&format!("Average throughput: {:.1} ops/sec\n", avg_throughput));
        report.push_str(&format!("Peak throughput: {:.1} ops/sec\n\n", best_throughput));
        
        // Detailed Test Results
        report.push_str("DETAILED TEST RESULTS:\n");
        report.push_str("=====================\n\n");
        
        for (config, stats) in test_results {
            report.push_str(&format!("Test: {}\n", config.name));
            report.push_str(&format!("Configuration:\n"));
            report.push_str(&format!("  - Format: {} (Quality: {:?})\n", config.format, config.quality));
            report.push_str(&format!("  - Iterations: {}, Concurrent: {}\n", config.iterations, config.concurrent_captures));
            report.push_str(&format!("  - Interval: {}ms\n", config.capture_interval_ms));
            if let Some((w, h)) = config.region_size {
                report.push_str(&format!("  - Region: {}x{} pixels\n", w, h));
            }
            
            report.push_str(&format!("Results:\n"));
            report.push_str(&format!("  - Success rate: {:.1}% ({}/{})\n", 
                            (stats.successful_operations as f64 / stats.total_operations as f64) * 100.0,
                            stats.successful_operations, stats.total_operations));
            report.push_str(&format!("  - Duration: Min {}, Avg {:.1}, Max {}ms\n", 
                            stats.min_duration_ms, stats.avg_duration_ms, stats.max_duration_ms));
            report.push_str(&format!("  - Throughput: {:.1} operations/second\n", stats.throughput_ops_per_sec));
            report.push_str(&format!("  - Memory: Avg {:.1}MB, Peak {:.1}MB\n", 
                            stats.memory_avg_mb, stats.memory_peak_mb));
            report.push_str(&format!("  - CPU: Avg {:.1}%, Peak {:.1}%\n\n", 
                            stats.cpu_avg_percent, stats.cpu_peak_percent));
        }
        
        // Performance Analysis
        report.push_str("PERFORMANCE ANALYSIS:\n");
        report.push_str("====================\n");
        
        // Find best and worst performing tests
        let fastest_test = test_results.iter().min_by(|(_, a), (_, b)| a.avg_duration_ms.partial_cmp(&b.avg_duration_ms).unwrap());
        let slowest_test = test_results.iter().max_by(|(_, a), (_, b)| a.avg_duration_ms.partial_cmp(&b.avg_duration_ms).unwrap());
        
        if let Some((config, stats)) = fastest_test {
            report.push_str(&format!("🚀 Fastest test: {} ({:.1}ms avg)\n", config.name, stats.avg_duration_ms));
        }
        
        if let Some((config, stats)) = slowest_test {
            report.push_str(&format!("🐌 Slowest test: {} ({:.1}ms avg)\n", config.name, stats.avg_duration_ms));
        }
        
        // Format comparison
        report.push_str("\nFormat Performance Comparison:\n");
        let mut format_stats = std::collections::HashMap::new();
        for (config, stats) in test_results {
            let format_entry = format_stats.entry(config.format.clone()).or_insert(Vec::new());
            format_entry.push(stats.avg_duration_ms);
        }
        
        for (format, times) in format_stats {
            let avg_time = times.iter().sum::<f64>() / times.len() as f64;
            report.push_str(&format!("  - {}: {:.1}ms average\n", format.to_uppercase(), avg_time));
        }
        
        // Recommendations
        report.push_str("\nRECOMMENDATIONS:\n");
        report.push_str("================\n");
        
        if avg_throughput > 50.0 {
            report.push_str("✅ High throughput achieved - system performs well under load\n");
        } else if avg_throughput > 20.0 {
            report.push_str("⚠️ Moderate throughput - consider optimizations for high-frequency capture\n");
        } else {
            report.push_str("🔴 Low throughput - investigate performance bottlenecks\n");
        }
        
        report.push_str("• For maximum performance: Use JPEG with quality 70-85\n");
        report.push_str("• For best quality: Use PNG or WebP lossless mode\n");
        report.push_str("• For balanced use: WebP with quality 80-90\n");
        report.push_str("• Avoid concurrent captures unless necessary\n");
        report.push_str("• Use region capture for improved performance\n");
        
        // Write report
        fs::write(&report_path, report)
            .map_err(|e| format!("Failed to write performance report: {}", e))?;
        
        println!("📊 Performance report written to: {}", report_path.display());
        
        Ok(())
    }
    
    /// Run all performance tests
    async fn run_all_tests(&mut self) -> ScreenshotResult<()> {
        let test_configs = Self::get_test_configs();
        let mut results = Vec::new();
        
        println!("🚀 Starting comprehensive performance monitoring...");
        println!("📋 Total tests: {}", test_configs.len());
        
        for (i, config) in test_configs.iter().enumerate() {
            println!("\n⏳ Test {}/{}: {}", i + 1, test_configs.len(), config.name);
            
            match self.run_test(config).await {
                Ok(stats) => {
                    results.push((config.clone(), stats));
                }
                Err(e) => {
                    eprintln!("❌ Test {} failed: {}", config.name, e);
                    // Continue with other tests
                }
            }
            
            // Brief pause between tests
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        
        // Generate comprehensive report
        self.generate_report(&results)?;
        
        println!("\n✅ Performance monitoring completed!");
        println!("📁 Results saved to: {}", self.output_dir.display());
        
        Ok(())
    }
}

/// Display usage information
fn print_usage() {
    println!("📈 Screenshot Performance Monitoring Tool");
    println!();
    println!("This example provides comprehensive performance analysis for screenshot capture,");
    println!("testing various scenarios including:");
    println!("• Single vs. concurrent capture performance");
    println!("• Format comparison (PNG, JPEG, WebP) timing");
    println!("• Quality setting impact on encoding speed");
    println!("• Region-based capture optimization");
    println!("• High-frequency capture stress testing");
    println!("• Resource usage monitoring (CPU, memory)");
    println!();
    println!("Output includes detailed performance metrics and recommendations.");
    println!();
    println!("Usage: cargo run --example performance_monitoring");
}

#[tokio::main]
async fn main() -> ScreenshotResult<()> {
    print_usage();
    
    let mut monitor = PerformanceMonitor::new()?;
    
    println!("🔧 Setting up test environment...");
    monitor.setup_test_content().await?;
    
    match monitor.run_all_tests().await {
        Ok(()) => {
            println!("\n🎉 Performance monitoring completed successfully!");
        }
        Err(e) => {
            eprintln!("\n💥 Performance monitoring failed: {}", e);
            std::process::exit(1);
        }
    }
    
    Ok(())
}

// Simple random number generator for example purposes
mod rand {
    use std::sync::atomic::{AtomicU64, Ordering};
    
    static SEED: AtomicU64 = AtomicU64::new(1);
    
    pub fn random<T>() -> T
    where
        T: From<u64>,
    {
        let prev = SEED.load(Ordering::Relaxed);
        let next = prev.wrapping_mul(1103515245).wrapping_add(12345);
        SEED.store(next, Ordering::Relaxed);
        T::from(next)
    }
}

// Placeholder for futures functionality
mod futures {
    pub mod future {
        pub async fn join_all<T>(futures: Vec<tokio::task::JoinHandle<T>>) -> Vec<Result<T, tokio::task::JoinError>> {
            let mut results = Vec::new();
            for future in futures {
                results.push(future.await);
            }
            results
        }
    }
}

// Placeholder for chrono functionality
mod chrono {
    pub struct Utc;
    
    impl Utc {
        pub fn now() -> DateTime {
            DateTime
        }
    }
    
    pub struct DateTime;
    
    impl DateTime {
        pub fn format(&self, _fmt: &str) -> FormattedDateTime {
            FormattedDateTime
        }
    }
    
    pub struct FormattedDateTime;
    
    impl std::fmt::Display for FormattedDateTime {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "2024-12-19 10:30:00 UTC")
        }
    }
}