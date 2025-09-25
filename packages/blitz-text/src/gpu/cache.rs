//! GPU cache implementation using goldylox

use goldylox::prelude::*;
use serde::{Deserialize, Serialize};

/// GPU cache statistics
#[derive(Debug, Default, Clone)]
pub struct GpuCacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub size_bytes: u64,
}

/// GPU cache configuration
#[derive(Debug, Clone)]
pub struct GpuCacheConfig {
    pub max_entries: usize,
    pub max_memory_mb: u32,
    pub compression_enabled: bool,
}

/// GPU resource key for goldylox caching
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct GpuResourceKey {
    pub resource_type: String,
    pub identifier: u64,
    pub size_hash: u64,
}

impl Default for GpuResourceKey {
    fn default() -> Self {
        Self {
            resource_type: String::new(),
            identifier: 0,
            size_hash: 0,
        }
    }
}

// GpuResourceKey is no longer needed - goldylox uses String keys directly

/// GPU resource value for goldylox caching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuResource {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub format: String,
    #[serde(skip, default = "std::time::Instant::now")]
    pub created_at: std::time::Instant,
}

impl Default for GpuResource {
    fn default() -> Self {
        Self {
            data: Vec::new(),
            width: 0,
            height: 0,
            format: String::from("RGBA"),
            created_at: std::time::Instant::now(),
        }
    }
}

impl CacheValue for GpuResource {
    type Metadata = CacheValueMetadata;

    fn estimated_size(&self) -> usize {
        std::mem::size_of::<GpuResource>() + self.data.len() + self.format.len()
    }

    fn is_expensive(&self) -> bool {
        self.data.len() > 1024 * 1024 // Large GPU resources are expensive
    }

    fn compression_hint(&self) -> CompressionHint {
        CompressionHint::Force // GPU data compresses well
    }

    fn metadata(&self) -> Self::Metadata {
        CacheValueMetadata::from_cache_value(self)
    }
}

/// Enhanced GPU cache wrapping glyphon::Cache with goldylox for extended functionality
pub struct EnhancedGpuCache {
    /// Primary glyphon cache for GPU texture operations (None in headless mode)
    glyphon_cache: Option<glyphon::Cache>,
    /// Secondary goldylox cache for application-level GPU resource caching
    resource_cache: Goldylox<String, GpuResource>,
}

impl EnhancedGpuCache {
    pub fn new(device: &wgpu::Device) -> Result<Self, Box<dyn std::error::Error>> {
        let glyphon_cache = Some(glyphon::Cache::new(device));

        let resource_cache = GoldyloxBuilder::<String, GpuResource>::new()
            .hot_tier_max_entries(2000)
            .hot_tier_memory_limit_mb(128)
            .warm_tier_max_entries(8000)
            .warm_tier_max_memory_bytes(256 * 1024 * 1024) // 256MB
            .cold_tier_max_size_bytes(512 * 1024 * 1024) // 512MB
            .compression_level(6)
            .background_worker_threads(4)
            .cache_id("enhanced_gpu_cache")
            .build()?;

        Ok(Self {
            glyphon_cache,
            resource_cache,
        })
    }

    /// Create a headless GPU cache that doesn't require GPU context
    /// This creates placeholder caches that can be used for basic operations
    /// but cannot perform actual GPU caching until initialized with a device later.
    pub fn headless() -> Self {
        // No GPU context in headless mode
        let glyphon_cache = None;

        // Create a basic goldylox cache without GPU-specific configuration
        let resource_cache = GoldyloxBuilder::<String, GpuResource>::new()
            .hot_tier_max_entries(100)
            .hot_tier_memory_limit_mb(16)
            .warm_tier_max_entries(400)
            .warm_tier_max_memory_bytes(32 * 1024 * 1024) // 32MB
            .cold_tier_max_size_bytes(64 * 1024 * 1024) // 64MB
            .compression_level(3)
            .background_worker_threads(1)
            .cache_id("headless_gpu_cache")
            .build()
            .unwrap_or_else(|_| {
                // If cache creation fails, create an ultra-minimal cache
                GoldyloxBuilder::<String, GpuResource>::new()
                    .cache_id("ultra_minimal_headless_gpu_cache")
                    .build()
                    .unwrap_or_else(|_| {
                        // If even that fails, create the absolute minimal cache
                        GoldyloxBuilder::<String, GpuResource>::new()
                            .build()
                            .expect("Failed to create even the most basic cache - this should never happen")
                    })
            });

        Self {
            glyphon_cache,
            resource_cache,
        }
    }

    pub fn init(&mut self, _max_entries: usize) -> Result<(), Box<dyn std::error::Error>> {
        // Configuration is handled in constructor with goldylox
        Ok(())
    }

    /// Initialize GPU cache with device context (for transitioning from headless mode)
    pub fn init_with_gpu(&mut self, device: &wgpu::Device) -> Result<(), Box<dyn std::error::Error>> {
        if self.glyphon_cache.is_none() {
            self.glyphon_cache = Some(glyphon::Cache::new(device));
        }
        Ok(())
    }

    /// Get glyphon cache reference (for GPU components)
    pub fn glyphon_cache(&self) -> Option<&glyphon::Cache> {
        self.glyphon_cache.as_ref()
    }

    /// Get GPU resource from resource cache
    pub fn get(&self, key: &str) -> Option<GpuResource> {
        self.resource_cache.get(&key.to_string())
    }

    /// Put GPU resource into resource cache
    pub fn put(&self, key: String, value: GpuResource) -> Result<(), Box<dyn std::error::Error>> {
        self.resource_cache
            .put(key, value)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }

    /// Clear resource cache
    pub fn clear(&self) {
        if let Err(e) = self.resource_cache.clear() {
            eprintln!("Warning: Failed to clear GPU resource cache: {}", e);
        }
    }

    pub fn len(&self) -> usize {
        // Goldylox doesn't expose len() - return 0 as placeholder
        0
    }

    /// Get cache statistics
    pub fn get_stats(&self) -> GpuCacheStats {
        GpuCacheStats::default()
    }

    /// Optimize cache performance
    pub fn optimize_cache(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Goldylox handles optimization internally
        Ok(())
    }

    /// Check if cache should be optimized
    pub fn should_optimize(&self) -> bool {
        false // Goldylox handles this internally
    }

    /// Set cache configuration
    pub fn set_config(&self, _config: GpuCacheConfig) {
        // Configuration is set at construction time with goldylox
    }

    /// Reset cache statistics
    pub fn reset_stats(&self) {
        // Statistics are managed internally by goldylox
    }

    /// Get resource cache reference (for advanced operations)
    pub fn resource_cache(&self) -> &Goldylox<String, GpuResource> {
        &self.resource_cache
    }
}

// Note: Default implementation removed as EnhancedGpuCache::new() now requires &wgpu::Device parameter

/// Texture atlas cache using goldylox
pub struct TextureAtlasCache {
    cache: Goldylox<String, GpuResource>,
}

impl TextureAtlasCache {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let cache = GoldyloxBuilder::<String, GpuResource>::new()
            .hot_tier_max_entries(1000)
            .hot_tier_memory_limit_mb(64)
            .warm_tier_max_entries(4000)
            .warm_tier_max_memory_bytes(128 * 1024 * 1024) // 128MB
            .cold_tier_max_size_bytes(256 * 1024 * 1024) // 256MB
            .compression_level(7)
            .background_worker_threads(2)
            .cache_id("texture_atlas_cache")
            .build()?;

        Ok(Self { cache })
    }

    pub fn init(&mut self, _max_entries: usize) -> Result<(), Box<dyn std::error::Error>> {
        // Configuration is handled in constructor with goldylox
        Ok(())
    }

    pub fn get(&self, key: &str) -> Option<GpuResource> {
        self.cache.get(&key.to_string())
    }

    pub fn put(&self, key: String, value: GpuResource) -> Result<(), Box<dyn std::error::Error>> {
        self.cache
            .put(key, value)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }

    pub fn clear(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.cache
            .clear()
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }
}

impl Default for TextureAtlasCache {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| panic!("Failed to create texture atlas cache"))
    }
}

/// Cache optimization result - simplified
#[derive(Debug, Clone)]
pub struct CacheOptimizationResult {
    pub entries_removed: usize,
    pub memory_freed: usize,
    pub fragmentation_reduced: f64,
}

impl Default for CacheOptimizationResult {
    fn default() -> Self {
        Self {
            entries_removed: 0,
            memory_freed: 0,
            fragmentation_reduced: 0.0,
        }
    }
}
