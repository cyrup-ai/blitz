//! Screenshot capture functionality for Blitz rendering engine
//!
//! This module provides screenshot capture capabilities that integrate with the anyrender
//! graphics backend to capture rendered content to various image formats (PNG, JPEG, WebP).

use std::sync::Arc;

use tokio::sync::oneshot;
use wgpu::{
    BufferDescriptor, BufferUsages, CommandEncoderDescriptor, TexelCopyBufferInfo,
    TexelCopyBufferLayout,
};

// Re-export thiserror for error handling
use thiserror::Error;

// Feature-gated imports for image encoding
#[cfg(feature = "png")]
use png;

#[cfg(feature = "jpeg")]
use mozjpeg_sys;
#[cfg(feature = "jpeg")]
use libc;

#[cfg(feature = "webp")]
use libwebp_sys;

/// Image format enumeration for screenshot encoding
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageFormat {
    /// PNG format - the only supported format
    Png,
    #[cfg(feature = "jpeg")]
    /// JPEG format
    Jpeg,
    #[cfg(feature = "webp")]
    /// WebP format
    WebP,
}

impl Default for ImageFormat {
    fn default() -> Self {
        Self::Png
    }
}

/// Rectangle for defining screenshot regions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rectangle {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Rectangle {
    /// Create a new rectangle with the specified dimensions
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self { x, y, width, height }
    }

    /// Check if the rectangle has valid dimensions (non-zero area)
    pub fn is_valid(&self) -> bool {
        self.width > 0 && self.height > 0
    }

    /// Check if this rectangle fits within the given bounds
    pub fn fits_within(&self, max_width: u32, max_height: u32) -> bool {
        self.x.saturating_add(self.width) <= max_width && 
        self.y.saturating_add(self.height) <= max_height
    }
}

/// Configuration for screenshot capture
#[derive(Debug, Clone)]
pub struct ScreenshotConfig {
    /// Image format for encoding
    pub format: ImageFormat,
    /// Quality setting (0-100, where 100 is highest quality)
    pub quality: u8,
    /// Optional region to capture (None = full texture)
    pub region: Option<Rectangle>,
}

impl Default for ScreenshotConfig {
    fn default() -> Self {
        Self {
            format: ImageFormat::default(),
            quality: 90,
            region: None,
        }
    }
}

impl ScreenshotConfig {
    /// Create a new screenshot configuration builder
    pub fn builder() -> ScreenshotConfigBuilder {
        ScreenshotConfigBuilder::default()
    }
}

/// Builder pattern for screenshot configuration
#[derive(Debug, Clone)]
pub struct ScreenshotConfigBuilder {
    format: ImageFormat,
    quality: u8,
    region: Option<Rectangle>,
}

impl Default for ScreenshotConfigBuilder {
    fn default() -> Self {
        Self {
            format: ImageFormat::default(),
            quality: 90,
            region: None,
        }
    }
}

impl ScreenshotConfigBuilder {
    /// Set the image format
    pub fn format(mut self, format: ImageFormat) -> Self {
        self.format = format;
        self
    }

    /// Set the quality (0-100, clamped internally)
    pub fn quality(mut self, quality: u8) -> Self {
        self.quality = quality.min(100);
        self
    }

    /// Set the capture region
    pub fn region(mut self, region: Option<Rectangle>) -> Self {
        self.region = region;
        self
    }

    /// Build the configuration
    pub fn build(self) -> ScreenshotConfig {
        ScreenshotConfig {
            format: self.format,
            quality: self.quality,
            region: self.region,
        }
    }
}

/// Screenshot capture request types
pub enum ScreenshotRequest {
    /// One-time screenshot capture
    OneTime {
        config: ScreenshotConfig,
        callback: Option<Box<dyn Fn(ScreenshotResult) + Send + Sync>>,
    },
}

impl std::fmt::Debug for ScreenshotRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OneTime { config, callback } => f
                .debug_struct("OneTime")
                .field("config", config)
                .field("callback", &callback.as_ref().map(|_| "<callback>"))
                .finish(),
        }
    }
}



/// Screenshot operation errors
#[derive(Error, Debug)]
pub enum ScreenshotError {
    #[error("Invalid region: {0}")]
    InvalidRegion(String),

    #[error("Encoding failed: {0}")]
    EncodingFailed(String),

    #[error("WGPU error: {0}")]
    WgpuError(String),

    #[error("Format not supported: {0}")]
    UnsupportedFormat(String),

    #[error("Buffer mapping failed")]
    BufferMappingFailed,

    #[error("Channel communication error: {0}")]
    ChannelError(String),

    #[error("Deprecated API: {0}")]
    DeprecatedApi(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Result type for screenshot operations
pub type ScreenshotResult = Result<Vec<u8>, ScreenshotError>;

/// Main screenshot engine for processing capture requests
pub struct ScreenshotEngine {
    /// WGPU device reference
    device: Arc<wgpu::Device>,
    /// WGPU queue reference
    queue: Arc<wgpu::Queue>,
    /// Queue of pending screenshot requests
    pending_requests: Vec<ScreenshotRequest>,
    /// Processing state flag to prevent concurrent processing
    is_processing: bool,
}

impl ScreenshotEngine {
    /// Create a new screenshot engine
    pub fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Self {
        Self {
            device,
            queue,
            pending_requests: Vec::new(),
            is_processing: false,
        }
    }

    /// Submit a screenshot request for processing
    pub fn submit_request(&mut self, request: ScreenshotRequest) -> Result<(), ScreenshotError> {
        // Validate request configuration
        match &request {
            ScreenshotRequest::OneTime { config, .. } => {
                if let Some(region) = &config.region {
                    if !region.is_valid() {
                        return Err(ScreenshotError::InvalidRegion(
                            format!("Region has zero area: {}x{}", region.width, region.height)
                        ));
                    }
                }
            }
        }

        self.pending_requests.push(request);
        Ok(())
    }

    /// Process all pending screenshot requests
    pub async fn process_pending_requests(
        &mut self,
        texture: &wgpu::Texture,
        texture_view: &wgpu::TextureView,
    ) -> Result<usize, ScreenshotError> {
        if self.is_processing {
            return Ok(0);
        }

        self.is_processing = true;

        let mut processed_count = 0;
        let requests = std::mem::take(&mut self.pending_requests);

        for request in requests {
            match self.process_single_request(texture, texture_view, request).await {
                Ok(_) => {
                    processed_count += 1;
                }
                Err(e) => {
                    // Log error but continue processing other requests
                    eprintln!("Screenshot processing error: {}", e);
                }
            }
        }

        self.is_processing = false;
        Ok(processed_count)
    }

    /// Get current number of pending requests
    pub fn pending_request_count(&self) -> usize {
        self.pending_requests.len()
    }



    /// Clear all pending requests
    pub fn clear_pending_requests(&mut self) {
        self.pending_requests.clear();
    }

    /// Process a single screenshot request
    async fn process_single_request(
        &self,
        texture: &wgpu::Texture,
        texture_view: &wgpu::TextureView,
        request: ScreenshotRequest,
    ) -> Result<(), ScreenshotError> {
        match request {
            ScreenshotRequest::OneTime { config, callback } => {
                let result = self.capture_screenshot(texture, texture_view, &config).await;
                if let Some(cb) = callback {
                    cb(result);
                }
                Ok(())
            }
        }
    }

    /// Capture a screenshot with the given configuration
    async fn capture_screenshot(
        &self,
        texture: &wgpu::Texture,
        _texture_view: &wgpu::TextureView,
        config: &ScreenshotConfig,
    ) -> ScreenshotResult {
        let texture_size = texture.size();
        
        // Determine capture region
        let region = if let Some(r) = &config.region {
            if !r.fits_within(texture_size.width, texture_size.height) {
                return Err(ScreenshotError::InvalidRegion(
                    format!("Region {}+{}+{}x{} exceeds texture bounds {}x{}",
                        r.x, r.y, r.width, r.height,
                        texture_size.width, texture_size.height)
                ));
            }
            *r
        } else {
            Rectangle::new(0, 0, texture_size.width, texture_size.height)
        };

        // Capture texture region to RGBA buffer
        let rgba_buffer = self.capture_texture_region(texture, region).await?;

        // Encode to requested format
        self.encode_image(&rgba_buffer, region.width, region.height, config).await
    }

    /// Capture a region of the texture to RGBA8 buffer
    async fn capture_texture_region(
        &self,
        texture: &wgpu::Texture,
        region: Rectangle,
    ) -> Result<Vec<u8>, ScreenshotError> {
        let padded_byte_width = (region.width * 4).next_multiple_of(256);
        let buffer_size = padded_byte_width as u64 * region.height as u64;

        // Create GPU buffer for texture data
        let gpu_buffer = self.device.create_buffer(&BufferDescriptor {
            label: Some("Screenshot capture buffer"),
            size: buffer_size,
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create command encoder for texture copy
        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Screenshot texture copy"),
        });

        // Copy texture region to buffer
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            TexelCopyBufferInfo {
                buffer: &gpu_buffer,
                layout: TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_byte_width),
                    rows_per_image: None,
                },
            },
            wgpu::Extent3d {
                width: region.width,
                height: region.height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit([encoder.finish()]);

        // Map buffer and read data
        let buffer_slice = gpu_buffer.slice(..);
        
        let (sender, receiver) = oneshot::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            if sender.send(result).is_err() {
                eprintln!("Failed to send buffer mapping result");
            }
        });

        // Wait for mapping to complete
        let _ = self.device.poll(wgpu::PollType::Wait);
        
        let mapping_result = receiver.await
            .map_err(|_| ScreenshotError::ChannelError("Buffer mapping channel closed".to_string()))?;
            
        mapping_result.map_err(|_| ScreenshotError::BufferMappingFailed)?;

        let data = buffer_slice.get_mapped_range();
        let mut cpu_buffer = Vec::with_capacity((region.width * region.height * 4) as usize);

        // Copy data row by row to handle padding
        for row in 0..region.height {
            let start = (row * padded_byte_width) as usize;
            let end = start + (region.width * 4) as usize;
            cpu_buffer.extend_from_slice(&data[start..end]);
        }

        // Clean up
        drop(data);
        gpu_buffer.unmap();

        Ok(cpu_buffer)
    }

    /// Encode RGBA buffer to specified image format
    async fn encode_image(
        &self,
        rgba_buffer: &[u8],
        width: u32,
        height: u32,
        config: &ScreenshotConfig,
    ) -> ScreenshotResult {
        let format = config.format.clone();
        let quality = config.quality;

        // Yield to allow other async tasks to run, then encode directly
        tokio::task::yield_now().await;

        match format {
            ImageFormat::Png => encode_png(rgba_buffer, width, height, quality),
            #[cfg(feature = "jpeg")]
            ImageFormat::Jpeg => encode_jpeg(rgba_buffer, width, height, quality),
            #[cfg(feature = "webp")]
            ImageFormat::WebP => encode_webp(rgba_buffer, width, height, quality),
        }
    }


}

/// Encode RGBA buffer to PNG format
#[cfg(feature = "png")]
fn encode_png(buffer: &[u8], width: u32, height: u32, _quality: u8) -> ScreenshotResult {
    let mut png_data = Vec::new();
    
    let mut encoder = png::Encoder::new(&mut png_data, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    
    let mut writer = encoder.write_header()
        .map_err(|e| ScreenshotError::EncodingFailed(format!("PNG header write failed: {}", e)))?;
    
    writer.write_image_data(buffer)
        .map_err(|e| ScreenshotError::EncodingFailed(format!("PNG data write failed: {}", e)))?;
    
    writer.finish()
        .map_err(|e| ScreenshotError::EncodingFailed(format!("PNG finish failed: {}", e)))?;

    Ok(png_data)
}

/// Fallback PNG encoding when png feature is not enabled
#[cfg(not(feature = "png"))]
fn encode_png(_buffer: &[u8], _width: u32, _height: u32, _quality: u8) -> ScreenshotResult {
    Err(ScreenshotError::UnsupportedFormat(
        "PNG support not enabled (missing 'png' feature)".to_string()
    ))
}

/// Encode RGBA buffer to JPEG format
#[cfg(feature = "jpeg")]
fn encode_jpeg(buffer: &[u8], width: u32, height: u32, quality: u8) -> ScreenshotResult {
    // Convert RGBA to RGB (JPEG doesn't support alpha)
    let rgb_buffer: Vec<u8> = buffer.chunks(4)
        .flat_map(|pixel| &pixel[0..3])
        .copied()
        .collect();

    // Use mozjpeg for encoding with proper error handling
    use std::ffi::c_int;
    
    unsafe {
        let mut cinfo: mozjpeg_sys::jpeg_compress_struct = std::mem::zeroed();
        let mut jerr: mozjpeg_sys::jpeg_error_mgr = std::mem::zeroed();
        
        // Set up error handling
        cinfo.common.err = mozjpeg_sys::jpeg_std_error(&mut jerr);
        mozjpeg_sys::jpeg_create_compress(&mut cinfo);
        
        // Set up memory destination
        let mut dest_buffer: *mut u8 = std::ptr::null_mut();
        let mut dest_size: std::os::raw::c_ulong = 0;
        
        mozjpeg_sys::jpeg_mem_dest(&mut cinfo, &mut dest_buffer, &mut dest_size);
        
        // Configure compression parameters
        cinfo.image_width = width;
        cinfo.image_height = height;
        cinfo.input_components = 3;
        cinfo.in_color_space = mozjpeg_sys::J_COLOR_SPACE::JCS_RGB;
        
        mozjpeg_sys::jpeg_set_defaults(&mut cinfo);
        mozjpeg_sys::jpeg_set_quality(&mut cinfo, quality.max(1).min(100) as c_int, 1);
        
        // Start compression
        mozjpeg_sys::jpeg_start_compress(&mut cinfo, 1);
        
        // Write scanlines
        let row_stride = (width * 3) as usize;
        let mut row = 0;
        while cinfo.next_scanline < cinfo.image_height {
            let row_start = row * row_stride;
            let row_end = (row_start + row_stride).min(rgb_buffer.len());
            
            if row_end > rgb_buffer.len() {
                mozjpeg_sys::jpeg_destroy_compress(&mut cinfo);
                if !dest_buffer.is_null() {
                    libc::free(dest_buffer as *mut libc::c_void);
                }
                return Err(ScreenshotError::EncodingFailed("Buffer underrun in JPEG encoding".to_string()));
            }
            
            let row_ptr = rgb_buffer.as_ptr().add(row_start);
            let row_ptrs = [row_ptr as *const u8];
            let written_lines = mozjpeg_sys::jpeg_write_scanlines(&mut cinfo, row_ptrs.as_ptr(), 1);
            
            if written_lines != 1 {
                mozjpeg_sys::jpeg_destroy_compress(&mut cinfo);
                if !dest_buffer.is_null() {
                    libc::free(dest_buffer as *mut libc::c_void);
                }
                return Err(ScreenshotError::EncodingFailed("Failed to write JPEG scanline".to_string()));
            }
            
            row += 1;
        }
        
        // Finish compression
        mozjpeg_sys::jpeg_finish_compress(&mut cinfo);
        mozjpeg_sys::jpeg_destroy_compress(&mut cinfo);
        
        // Copy data to owned Vec
        if dest_buffer.is_null() || dest_size == 0 {
            return Err(ScreenshotError::EncodingFailed("JPEG encoding produced no data".to_string()));
        }
        
        let jpeg_data = std::slice::from_raw_parts(dest_buffer, dest_size as usize).to_vec();
        libc::free(dest_buffer as *mut libc::c_void);
        
        Ok(jpeg_data)
    }
}

/// Encode RGBA buffer to WebP format
#[cfg(feature = "webp")]
fn encode_webp(buffer: &[u8], width: u32, height: u32, quality: u8) -> ScreenshotResult {
    use std::ffi::c_int;
    
    // Validate input parameters
    if buffer.len() != (width * height * 4) as usize {
        return Err(ScreenshotError::EncodingFailed(
            format!("Buffer size {} doesn't match expected size {} for {}x{} RGBA image",
                buffer.len(), width * height * 4, width, height)
        ));
    }
    
    unsafe {
        let stride = (width * 4) as c_int;
        let mut output_buffer: *mut u8 = std::ptr::null_mut();
        
        let encoded_size = if quality == 100 {
            // Lossless encoding
            libwebp_sys::WebPEncodeLosslessRGBA(
                buffer.as_ptr(),
                width as c_int,
                height as c_int,
                stride,
                &mut output_buffer,
            )
        } else {
            // Lossy encoding with quality validation
            let clamped_quality = quality.max(1).min(100) as f32;
            libwebp_sys::WebPEncodeRGBA(
                buffer.as_ptr(),
                width as c_int,
                height as c_int,
                stride,
                clamped_quality,
                &mut output_buffer,
            )
        };
        
        if encoded_size == 0 || output_buffer.is_null() {
            // Clean up any allocated memory
            if !output_buffer.is_null() {
                libwebp_sys::WebPFree(output_buffer as *mut std::ffi::c_void);
            }
            return Err(ScreenshotError::EncodingFailed("WebP encoding failed".to_string()));
        }
        
        // Copy data to owned Vec and free WebP-allocated memory
        let webp_data = std::slice::from_raw_parts(output_buffer, encoded_size).to_vec();
        libwebp_sys::WebPFree(output_buffer as *mut std::ffi::c_void);
        
        Ok(webp_data)
    }
}

// Legacy Screenshot struct for backwards compatibility (marked as deprecated)
#[deprecated(note = "Use ScreenshotEngine directly instead")]
pub struct Screenshot {
    engine: ScreenshotEngine,
}

#[allow(deprecated)]
impl Screenshot {
    pub fn new() -> Result<Self, ScreenshotError> {
        // This is a stub for backwards compatibility
        // Real implementations should use ScreenshotEngine directly
        Err(ScreenshotError::DeprecatedApi("Use ScreenshotEngine directly".to_string()))
    }
    
    /// Access the internal engine (deprecated - use ScreenshotEngine directly)
    #[deprecated(note = "Use ScreenshotEngine directly instead")]
    pub fn engine(&self) -> &ScreenshotEngine {
        &self.engine
    }
}

// Default implementation removed - deprecated struct should not have convenient construction