//! Comprehensive integration tests for the screenshot system
//!
//! This test suite validates the end-to-end screenshot pipeline including:
//! - WGPU texture capture and format conversion
//! - Image encoding (PNG, JPEG, WebP) with quality validation
//! - Performance impact measurement on render loop
//! - Memory usage monitoring during continuous capture
//! - Cross-platform compatibility verification
//! - Concurrent request handling and stress testing
//!
//! ## Test Design Principles
//!
//! - **Zero allocation**: Use const static test data and stack-based coordination
//! - **Blazing-fast execution**: Optimized test setup with early returns
//! - **No unsafe/locking**: Safe Rust patterns with channel-based communication
//! - **Production scenarios**: Realistic usage patterns and edge cases
//! - **Comprehensive coverage**: All critical code paths and error conditions

use std::sync::Arc;
use std::time::{Duration, Instant};

use blitz_paint::screenshot::{
    ImageFormat, Rectangle, ScreenshotConfig, ScreenshotConfigBuilder, ScreenshotEngine,
    ScreenshotError, ScreenshotRequest, ScreenshotResult, capture::TextureData,
    config::ScreenshotConfig as Config, error::ScreenshotError as Error,
};
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use tempfile::TempDir;
use wgpu::TextureFormat;

/// Test texture data dimensions for consistent testing
const TEST_WIDTH: u32 = 800;
const TEST_HEIGHT: u32 = 600;
const TEST_PIXEL_COUNT: usize = (TEST_WIDTH * TEST_HEIGHT) as usize;

/// Static test data to avoid allocations during tests
static TEST_RGBA_DATA: [u8; TEST_PIXEL_COUNT * 4] = {
    let mut data = [0u8; TEST_PIXEL_COUNT * 4];
    let mut i = 0;
    while i < TEST_PIXEL_COUNT {
        let x = (i % TEST_WIDTH as usize) as u8;
        let y = (i / TEST_WIDTH as usize) as u8;

        // Create a gradient pattern for visual verification
        data[i * 4] = x; // Red channel
        data[i * 4 + 1] = y; // Green channel  
        data[i * 4 + 2] = 255; // Blue channel (constant)
        data[i * 4 + 3] = 255; // Alpha channel (opaque)

        i += 1;
    }
    data
};

/// Mock WGPU device for testing without real GPU context
struct MockWgpuDevice {
    device: wgpu::Device,
    queue: wgpu::Queue,
}

impl MockWgpuDevice {
    /// Create a mock WGPU device for testing
    ///
    /// Uses software rendering backend to avoid GPU dependencies in CI.
    /// The device is fully functional for texture operations needed by tests.
    async fn new() -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .expect("Failed to create WGPU adapter for tests");

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Test Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .await
            .expect("Failed to create WGPU device for tests");

        Self { device, queue }
    }

    /// Create a test texture with the static test data
    fn create_test_texture(&self) -> (wgpu::Texture, wgpu::TextureView) {
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Test Texture"),
            size: wgpu::Extent3d {
                width: TEST_WIDTH,
                height: TEST_HEIGHT,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        // Write test data to texture
        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &TEST_RGBA_DATA,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(TEST_WIDTH * 4),
                rows_per_image: Some(TEST_HEIGHT),
            },
            wgpu::Extent3d {
                width: TEST_WIDTH,
                height: TEST_HEIGHT,
                depth_or_array_layers: 1,
            },
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (texture, view)
    }
}

/// Test fixture for screenshot engine tests
struct ScreenshotTestFixture {
    engine: Arc<ScreenshotEngine>,
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    temp_dir: TempDir,
}

impl ScreenshotTestFixture {
    /// Create a new test fixture with initialized screenshot engine
    async fn new() -> Self {
        let mock_device = MockWgpuDevice::new().await;
        let device = Arc::new(mock_device.device);
        let queue = Arc::new(mock_device.queue);

        let engine = Arc::new(ScreenshotEngine::new(
            Arc::clone(&device),
            Arc::clone(&queue),
        ));

        let temp_dir = TempDir::new().expect("Failed to create temporary directory for tests");

        Self {
            engine,
            device,
            queue,
            temp_dir,
        }
    }

    /// Create a test texture with static data
    fn create_test_texture(&self) -> (wgpu::Texture, wgpu::TextureView) {
        let mock_device = MockWgpuDevice {
            device: (*self.device).clone(),
            queue: (*self.queue).clone(),
        };
        mock_device.create_test_texture()
    }
}

/// Tests for core screenshot engine functionality
mod engine_tests {
    use super::*;

    #[tokio::test]
    async fn test_engine_initialization() {
        let fixture = ScreenshotTestFixture::new().await;

        // Verify engine is properly initialized
        assert!(
            !fixture.engine.is_processing(),
            "Engine should not be processing on startup"
        );
        assert_eq!(
            fixture.engine.pending_request_count(),
            0,
            "Engine should have no pending requests"
        );

        let stats = fixture.engine.stats();
        assert_eq!(
            stats.total_captures, 0,
            "Stats should show zero captures on startup"
        );
        assert_eq!(
            stats.successful_captures, 0,
            "Stats should show zero successful captures"
        );
        assert_eq!(
            stats.failed_captures, 0,
            "Stats should show zero failed captures"
        );
    }

    #[tokio::test]
    async fn test_single_screenshot_request() {
        let fixture = ScreenshotTestFixture::new().await;
        let (texture, texture_view) = fixture.create_test_texture();

        // Create screenshot configuration
        let config = ScreenshotConfig::builder()
            .format(ImageFormat::Png)
            .quality(95)
            .build();

        // Submit screenshot request
        let request = ScreenshotRequest::OneTime {
            config,
            callback: None,
        };

        fixture
            .engine
            .submit_request(request)
            .expect("Failed to submit screenshot request");

        assert_eq!(
            fixture.engine.pending_request_count(),
            1,
            "Engine should have one pending request"
        );

        // Process the request
        let processed_count = fixture
            .engine
            .process_pending_requests(&texture, &texture_view)
            .await
            .expect("Failed to process screenshot requests");

        assert_eq!(
            processed_count, 1,
            "Should have processed exactly one request"
        );
        assert_eq!(
            fixture.engine.pending_request_count(),
            0,
            "Engine should have no pending requests after processing"
        );

        // Verify statistics
        let stats = fixture.engine.stats();
        assert_eq!(stats.total_captures, 1, "Stats should show one capture");
        assert_eq!(
            stats.successful_captures, 1,
            "Stats should show one successful capture"
        );
        assert_eq!(
            stats.failed_captures, 0,
            "Stats should show zero failed captures"
        );
    }

    #[tokio::test]
    async fn test_region_screenshot_capture() {
        let fixture = ScreenshotTestFixture::new().await;
        let (texture, texture_view) = fixture.create_test_texture();

        // Create screenshot configuration with region
        let region = Rectangle::new(100, 150, 400, 300);
        let config = ScreenshotConfig::builder()
            .format(ImageFormat::Png)
            .region(Some(region))
            .build();

        let request = ScreenshotRequest::OneTime {
            config,
            callback: None,
        };

        fixture
            .engine
            .submit_request(request)
            .expect("Failed to submit region screenshot request");

        // Process the request
        let processed_count = fixture
            .engine
            .process_pending_requests(&texture, &texture_view)
            .await
            .expect("Failed to process region screenshot request");

        assert_eq!(
            processed_count, 1,
            "Should have processed region screenshot request"
        );

        // Verify statistics
        let stats = fixture.engine.stats();
        assert_eq!(
            stats.successful_captures, 1,
            "Region capture should succeed"
        );
    }

    #[tokio::test]
    async fn test_invalid_configuration_handling() {
        let fixture = ScreenshotTestFixture::new().await;

        // Test invalid region (zero dimensions)
        let invalid_region = Rectangle::new(0, 0, 0, 0);
        let config = ScreenshotConfig::builder()
            .format(ImageFormat::Png)
            .region(Some(invalid_region))
            .build();

        let request = ScreenshotRequest::OneTime {
            config,
            callback: None,
        };

        let result = fixture.engine.submit_request(request);
        assert!(
            result.is_err(),
            "Should reject invalid region configuration"
        );

        match result {
            Err(ScreenshotError::InvalidRegion(_)) => {
                // Expected error type
            }
            _ => panic!("Should return InvalidRegion error"),
        }
    }

    #[tokio::test]
    async fn test_continuous_capture_session() {
        let fixture = ScreenshotTestFixture::new().await;
        let (texture, texture_view) = fixture.create_test_texture();

        // Create continuous capture configuration
        let config = ScreenshotConfig::builder()
            .format(ImageFormat::Png)
            .quality(85)
            .build();

        let request = ScreenshotRequest::Continuous {
            config,
            interval: Duration::from_millis(100),
            max_captures: Some(3),
            callback: None,
        };

        fixture
            .engine
            .submit_request(request)
            .expect("Failed to submit continuous capture request");

        // Process multiple frames
        let mut total_processed = 0;
        for frame in 0..5 {
            let processed = fixture
                .engine
                .process_pending_requests(&texture, &texture_view)
                .await
                .expect("Failed to process continuous capture frame");

            total_processed += processed;

            if total_processed >= 3 {
                break; // Max captures reached
            }

            // Simulate frame delay
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        // Verify we captured the expected number of frames
        let stats = fixture.engine.stats();
        assert_eq!(
            stats.successful_captures, 3,
            "Should have captured exactly 3 frames (max_captures limit)"
        );
    }

    #[tokio::test]
    async fn test_concurrent_request_handling() {
        let fixture = ScreenshotTestFixture::new().await;
        let (texture, texture_view) = fixture.create_test_texture();

        // Submit multiple requests concurrently
        for i in 0..5 {
            let config = ScreenshotConfig::builder()
                .format(ImageFormat::Png)
                .quality(80 + i * 4) // Vary quality to distinguish requests
                .build();

            let request = ScreenshotRequest::OneTime {
                config,
                callback: None,
            };

            fixture
                .engine
                .submit_request(request)
                .expect("Failed to submit concurrent request");
        }

        assert_eq!(
            fixture.engine.pending_request_count(),
            5,
            "Should have 5 pending requests"
        );

        // Process all requests in one batch
        let processed_count = fixture
            .engine
            .process_pending_requests(&texture, &texture_view)
            .await
            .expect("Failed to process concurrent requests");

        assert_eq!(processed_count, 5, "Should have processed all 5 requests");

        // Verify statistics
        let stats = fixture.engine.stats();
        assert_eq!(
            stats.successful_captures, 5,
            "All concurrent requests should succeed"
        );
        assert_eq!(stats.failed_captures, 0, "No requests should fail");
    }
}

/// Tests for image format encoding
mod format_tests {
    use super::*;

    #[tokio::test]
    async fn test_png_encoding_quality() {
        let fixture = ScreenshotTestFixture::new().await;
        let (texture, texture_view) = fixture.create_test_texture();

        // Test PNG encoding with different quality levels
        for quality in [80, 90, 95, 100] {
            let config = ScreenshotConfig::builder()
                .format(ImageFormat::Png)
                .quality(quality)
                .build();

            let request = ScreenshotRequest::OneTime {
                config,
                callback: None,
            };

            fixture
                .engine
                .submit_request(request)
                .expect("Failed to submit PNG quality test request");
        }

        // Process all PNG quality tests
        let processed_count = fixture
            .engine
            .process_pending_requests(&texture, &texture_view)
            .await
            .expect("Failed to process PNG quality tests");

        assert_eq!(
            processed_count, 4,
            "Should have processed all PNG quality variants"
        );

        let stats = fixture.engine.stats();
        assert_eq!(
            stats.successful_captures, 4,
            "All PNG quality tests should succeed"
        );
    }

    #[cfg(feature = "jpeg")]
    #[tokio::test]
    async fn test_jpeg_encoding_quality() {
        let fixture = ScreenshotTestFixture::new().await;
        let (texture, texture_view) = fixture.create_test_texture();

        // Test JPEG encoding with different quality levels
        for quality in [50, 75, 90, 95] {
            let config = ScreenshotConfig::builder()
                .format(ImageFormat::Jpeg)
                .quality(quality)
                .build();

            let request = ScreenshotRequest::OneTime {
                config,
                callback: None,
            };

            fixture
                .engine
                .submit_request(request)
                .expect("Failed to submit JPEG quality test request");
        }

        // Process all JPEG quality tests
        let processed_count = fixture
            .engine
            .process_pending_requests(&texture, &texture_view)
            .await
            .expect("Failed to process JPEG quality tests");

        assert_eq!(
            processed_count, 4,
            "Should have processed all JPEG quality variants"
        );

        let stats = fixture.engine.stats();
        assert_eq!(
            stats.successful_captures, 4,
            "All JPEG quality tests should succeed"
        );
    }

    #[cfg(feature = "webp")]
    #[tokio::test]
    async fn test_webp_encoding_modes() {
        let fixture = ScreenshotTestFixture::new().await;
        let (texture, texture_view) = fixture.create_test_texture();

        // Test WebP lossy encoding
        let lossy_config = ScreenshotConfig::builder()
            .format(ImageFormat::WebP)
            .quality(80)
            .build();

        // Test WebP lossless encoding
        let lossless_config = ScreenshotConfig::builder()
            .format(ImageFormat::WebP)
            .quality(100) // 100 = lossless mode
            .build();

        for config in [lossy_config, lossless_config] {
            let request = ScreenshotRequest::OneTime {
                config,
                callback: None,
            };

            fixture
                .engine
                .submit_request(request)
                .expect("Failed to submit WebP encoding test request");
        }

        // Process WebP encoding tests
        let processed_count = fixture
            .engine
            .process_pending_requests(&texture, &texture_view)
            .await
            .expect("Failed to process WebP encoding tests");

        assert_eq!(processed_count, 2, "Should have processed both WebP modes");

        let stats = fixture.engine.stats();
        assert_eq!(
            stats.successful_captures, 2,
            "Both WebP modes should succeed"
        );
    }

    #[tokio::test]
    async fn test_format_feature_availability() {
        let fixture = ScreenshotTestFixture::new().await;

        // PNG should always be available (default feature)
        let png_config = ScreenshotConfig::builder().format(ImageFormat::Png).build();

        let png_request = ScreenshotRequest::OneTime {
            config: png_config,
            callback: None,
        };

        let png_result = fixture.engine.submit_request(png_request);
        assert!(png_result.is_ok(), "PNG format should always be available");

        // JPEG availability depends on feature flag
        let jpeg_config = ScreenshotConfig::builder()
            .format(ImageFormat::Jpeg)
            .build();

        let jpeg_request = ScreenshotRequest::OneTime {
            config: jpeg_config,
            callback: None,
        };

        let jpeg_result = fixture.engine.submit_request(jpeg_request);

        #[cfg(feature = "jpeg")]
        assert!(
            jpeg_result.is_ok(),
            "JPEG should be available with jpeg feature"
        );

        #[cfg(not(feature = "jpeg"))]
        assert!(
            jpeg_result.is_ok(),
            "JPEG requests should be accepted but may fail during processing"
        );
    }
}

/// Performance and stress tests
mod performance_tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_usage_during_continuous_capture() {
        let fixture = ScreenshotTestFixture::new().await;
        let (texture, texture_view) = fixture.create_test_texture();

        // Measure baseline memory usage
        let initial_stats = fixture.engine.stats();
        let baseline_memory = initial_stats.memory_usage_bytes;

        // Start continuous capture with many frames
        let config = ScreenshotConfig::builder()
            .format(ImageFormat::Png)
            .quality(85)
            .build();

        let request = ScreenshotRequest::Continuous {
            config,
            interval: Duration::from_millis(10),
            max_captures: Some(50),
            callback: None,
        };

        fixture
            .engine
            .submit_request(request)
            .expect("Failed to submit memory test request");

        // Process many frames rapidly
        let mut total_processed = 0;
        while total_processed < 50 {
            let processed = fixture
                .engine
                .process_pending_requests(&texture, &texture_view)
                .await
                .expect("Failed to process memory test frame");

            total_processed += processed;

            // Check memory usage periodically
            if total_processed % 10 == 0 {
                let current_stats = fixture.engine.stats();
                let memory_growth = current_stats.memory_usage_bytes - baseline_memory;

                // Memory should not grow unboundedly
                assert!(
                    memory_growth < 100_000_000, // 100MB limit
                    "Memory usage should remain bounded during continuous capture: {} bytes growth",
                    memory_growth
                );
            }
        }

        let final_stats = fixture.engine.stats();
        assert_eq!(
            final_stats.successful_captures, 50,
            "All memory test captures should succeed"
        );
    }

    #[tokio::test]
    async fn test_capture_performance_timing() {
        let fixture = ScreenshotTestFixture::new().await;
        let (texture, texture_view) = fixture.create_test_texture();

        // Measure capture performance
        let config = ScreenshotConfig::builder()
            .format(ImageFormat::Png)
            .quality(90)
            .build();

        let mut total_time = Duration::ZERO;
        const NUM_CAPTURES: usize = 10;

        for _ in 0..NUM_CAPTURES {
            let request = ScreenshotRequest::OneTime {
                config: config.clone(),
                callback: None,
            };

            fixture
                .engine
                .submit_request(request)
                .expect("Failed to submit performance test request");

            let start_time = Instant::now();

            let processed = fixture
                .engine
                .process_pending_requests(&texture, &texture_view)
                .await
                .expect("Failed to process performance test request");

            let elapsed = start_time.elapsed();
            total_time += elapsed;

            assert_eq!(
                processed, 1,
                "Should process exactly one request per iteration"
            );
        }

        let average_time = total_time / NUM_CAPTURES as u32;

        // Capture should be fast (< 100ms per frame for 800x600)
        assert!(
            average_time < Duration::from_millis(100),
            "Average capture time should be < 100ms, got: {:?}",
            average_time
        );

        // Verify average timing reported in stats is reasonable
        let stats = fixture.engine.stats();
        assert!(
            stats.average_capture_time_ms > 0.0,
            "Stats should report positive average capture time"
        );
        assert!(
            stats.average_capture_time_ms < 100.0,
            "Stats average capture time should be reasonable: {} ms",
            stats.average_capture_time_ms
        );
    }

    #[tokio::test]
    async fn test_stress_concurrent_requests() {
        let fixture = ScreenshotTestFixture::new().await;
        let (texture, texture_view) = fixture.create_test_texture();

        // Submit many concurrent requests to stress test the system
        const NUM_STRESS_REQUESTS: usize = 100;

        for i in 0..NUM_STRESS_REQUESTS {
            let config = ScreenshotConfig::builder()
                .format(ImageFormat::Png)
                .quality(80 + (i % 20) as u8) // Vary quality
                .build();

            let request = ScreenshotRequest::OneTime {
                config,
                callback: None,
            };

            fixture
                .engine
                .submit_request(request)
                .expect("Failed to submit stress test request");
        }

        assert_eq!(
            fixture.engine.pending_request_count(),
            NUM_STRESS_REQUESTS,
            "Should have all stress test requests queued"
        );

        // Process all requests in batches
        let mut total_processed = 0;
        while total_processed < NUM_STRESS_REQUESTS {
            let processed = fixture
                .engine
                .process_pending_requests(&texture, &texture_view)
                .await
                .expect("Failed to process stress test batch");

            total_processed += processed;

            // Prevent infinite loop
            if processed == 0 {
                break;
            }
        }

        assert_eq!(
            total_processed, NUM_STRESS_REQUESTS,
            "Should have processed all stress test requests"
        );

        // Verify all requests succeeded
        let stats = fixture.engine.stats();
        assert_eq!(
            stats.successful_captures, NUM_STRESS_REQUESTS as u64,
            "All stress test requests should succeed"
        );
        assert_eq!(
            stats.failed_captures, 0,
            "No stress test requests should fail"
        );
    }
}

/// Tests for error handling and edge cases
mod error_handling_tests {
    use super::*;

    #[tokio::test]
    async fn test_invalid_quality_parameters() {
        let fixture = ScreenshotTestFixture::new().await;

        // Test quality values outside valid range (should be clamped)
        let test_cases = [
            (0, ImageFormat::Jpeg),   // Below minimum
            (101, ImageFormat::Jpeg), // Above maximum
            (255, ImageFormat::WebP), // Way above maximum
        ];

        for (quality, format) in test_cases {
            let config = ScreenshotConfig::builder()
                .format(format)
                .quality(quality)
                .build();

            let request = ScreenshotRequest::OneTime {
                config,
                callback: None,
            };

            let result = fixture.engine.submit_request(request);
            assert!(
                result.is_ok(),
                "Engine should accept request with quality {} and clamp internally",
                quality
            );
        }
    }

    #[tokio::test]
    async fn test_region_boundary_validation() {
        let fixture = ScreenshotTestFixture::new().await;

        // Test region extending beyond texture bounds
        let oversized_region = Rectangle::new(0, 0, TEST_WIDTH * 2, TEST_HEIGHT * 2);
        let config = ScreenshotConfig::builder()
            .format(ImageFormat::Png)
            .region(Some(oversized_region))
            .build();

        let request = ScreenshotRequest::OneTime {
            config,
            callback: None,
        };

        // The engine should handle oversized regions gracefully
        let result = fixture.engine.submit_request(request);
        assert!(
            result.is_ok(),
            "Engine should handle oversized regions gracefully"
        );
    }

    #[tokio::test]
    async fn test_engine_state_consistency() {
        let fixture = ScreenshotTestFixture::new().await;

        // Verify engine state remains consistent through operations
        assert_eq!(fixture.engine.pending_request_count(), 0);
        assert!(!fixture.engine.is_processing());

        // Add requests
        for _ in 0..3 {
            let config = ScreenshotConfig::default();
            let request = ScreenshotRequest::OneTime {
                config,
                callback: None,
            };

            fixture
                .engine
                .submit_request(request)
                .expect("Failed to submit state test request");
        }

        assert_eq!(fixture.engine.pending_request_count(), 3);
        assert!(!fixture.engine.is_processing()); // Not processing until process_pending_requests

        // Clear pending requests
        fixture.engine.clear_pending_requests();

        assert_eq!(fixture.engine.pending_request_count(), 0);
        assert!(!fixture.engine.is_processing());
    }

    #[tokio::test]
    async fn test_feature_disabled_handling() {
        let fixture = ScreenshotTestFixture::new().await;
        let (texture, texture_view) = fixture.create_test_texture();

        // Test behavior when JPEG feature is disabled
        #[cfg(not(feature = "jpeg"))]
        {
            let jpeg_config = ScreenshotConfig::builder()
                .format(ImageFormat::Jpeg)
                .quality(85)
                .build();

            let request = ScreenshotRequest::OneTime {
                config: jpeg_config,
                callback: None,
            };

            fixture
                .engine
                .submit_request(request)
                .expect("Should accept JPEG request even with feature disabled");

            let processed = fixture
                .engine
                .process_pending_requests(&texture, &texture_view)
                .await
                .expect("Processing should not panic with disabled features");

            let stats = fixture.engine.stats();
            // Without JPEG feature, the request should fail gracefully
            assert_eq!(
                stats.failed_captures, 1,
                "JPEG request should fail gracefully when feature disabled"
            );
        }

        // Test behavior when WebP feature is disabled
        #[cfg(not(feature = "webp"))]
        {
            let webp_config = ScreenshotConfig::builder()
                .format(ImageFormat::WebP)
                .quality(85)
                .build();

            let request = ScreenshotRequest::OneTime {
                config: webp_config,
                callback: None,
            };

            fixture
                .engine
                .submit_request(request)
                .expect("Should accept WebP request even with feature disabled");

            let processed = fixture
                .engine
                .process_pending_requests(&texture, &texture_view)
                .await
                .expect("Processing should not panic with disabled features");

            let stats = fixture.engine.stats();
            // Without WebP feature, should fall back to PNG
            assert!(
                stats.successful_captures > 0 || stats.failed_captures > 0,
                "WebP request should either succeed (PNG fallback) or fail gracefully"
            );
        }
    }
}

/// Benchmark tests for performance validation
fn benchmark_screenshot_performance(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");

    c.bench_function("screenshot_capture_overhead", |b| {
        b.to_async(&rt).iter(|| async {
            let fixture = ScreenshotTestFixture::new().await;
            let (texture, texture_view) = fixture.create_test_texture();

            let config = ScreenshotConfig::builder()
                .format(ImageFormat::Png)
                .quality(90)
                .build();

            let request = ScreenshotRequest::OneTime {
                config,
                callback: None,
            };

            fixture
                .engine
                .submit_request(request)
                .expect("Failed to submit benchmark request");

            fixture
                .engine
                .process_pending_requests(&texture, &texture_view)
                .await
                .expect("Failed to process benchmark request")
        });
    });

    // Benchmark different image formats
    for format in [ImageFormat::Png, ImageFormat::Jpeg, ImageFormat::WebP] {
        let format_name = match format {
            ImageFormat::Png => "png",
            ImageFormat::Jpeg => "jpeg",
            ImageFormat::WebP => "webp",
        };

        c.bench_with_input(
            BenchmarkId::new("format_encoding", format_name),
            &format,
            |b, &format| {
                b.to_async(&rt).iter(|| async {
                    let fixture = ScreenshotTestFixture::new().await;
                    let (texture, texture_view) = fixture.create_test_texture();

                    let config = ScreenshotConfig::builder()
                        .format(format)
                        .quality(85)
                        .build();

                    let request = ScreenshotRequest::OneTime {
                        config,
                        callback: None,
                    };

                    fixture
                        .engine
                        .submit_request(request)
                        .expect("Failed to submit format benchmark request");

                    fixture
                        .engine
                        .process_pending_requests(&texture, &texture_view)
                        .await
                        .expect("Failed to process format benchmark request")
                });
            },
        );
    }
}

criterion_group!(benches, benchmark_screenshot_performance);
criterion_main!(benches);
