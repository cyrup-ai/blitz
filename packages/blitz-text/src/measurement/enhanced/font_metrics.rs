//! Font metrics using goldylox multi-tier caching

use cosmyc_text::fontdb;
use goldylox::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

use crate::measurement::types::FontMetrics;

/// Font metrics cache key
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
pub struct FontMetricsCacheKey {
    pub font_id: u32, // Using u32 instead of fontdb::ID for serialization
    pub font_size_bits: u32,
}

impl FontMetricsCacheKey {
    pub fn new(font_id: fontdb::ID, font_size: f32) -> Self {
        // Since fontdb::ID constructor is private, we use a hash of the ID as a workaround
        // This maintains cache functionality while avoiding private field access
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        font_id.hash(&mut hasher);
        Self {
            font_id: hasher.finish() as u32,
            font_size_bits: font_size.to_bits(),
        }
    }
}

impl CacheKey for FontMetricsCacheKey {
    type HashContext = StandardHashContext;
    type Priority = StandardPriority;
    type SizeEstimator = StandardSizeEstimator;

    fn estimated_size(&self) -> usize {
        std::mem::size_of::<Self>()
    }

    fn hash_context(&self) -> Self::HashContext {
        use goldylox::cache::traits::supporting_types::HashAlgorithm;
        StandardHashContext::new(HashAlgorithm::AHash, 0x517cc1b727220a95)
    }

    fn fast_hash(&self, _context: &Self::HashContext) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }

    fn priority(&self) -> Self::Priority {
        // Font metrics priority based on font size
        let font_size = f32::from_bits(self.font_size_bits);
        let priority_value = if font_size > 32.0 {
            6 // Medium priority for large fonts
        } else if font_size < 8.0 {
            3 // Lower priority for very small fonts
        } else {
            9 // High priority for normal reading sizes
        };
        StandardPriority::new(priority_value)
    }

    fn size_estimator(&self) -> Self::SizeEstimator {
        StandardSizeEstimator::new()
    }
}

impl CacheValue for FontMetrics {
    type Metadata = CacheValueMetadata;

    fn estimated_size(&self) -> usize {
        std::mem::size_of::<Self>()
    }

    fn is_expensive(&self) -> bool {
        false // Font metrics are lightweight
    }

    fn compression_hint(&self) -> CompressionHint {
        CompressionHint::Auto
    }

    fn metadata(&self) -> Self::Metadata {
        CacheValueMetadata::from_cache_value(self)
    }
}

/// Font metrics cache using goldylox with HashMap fallback
pub struct FontMetricsCache {
    cache_type: CacheType,
}

enum CacheType {
    Goldylox(Goldylox<FontMetricsCacheKey, FontMetrics>),
    HashMap(Mutex<HashMap<FontMetricsCacheKey, FontMetrics>>),
}

impl FontMetricsCache {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        println!("✅ FontMetricsCache using Goldylox for efficient multi-tier caching");
        
        // Use Goldylox with proper types (both FontMetricsCacheKey and FontMetrics implement required traits)
        let cache = GoldyloxBuilder::<FontMetricsCacheKey, FontMetrics>::new()
            .hot_tier_memory_limit_mb(8) // 8MB for font metrics
            .cold_tier_max_size_bytes(0) // In-memory only for font metrics (lightweight data)
            .build().await?;
        println!("✅ FontMetricsCache: Goldylox build complete");
            
        let cache_type = CacheType::Goldylox(cache);
        println!("✅ FontMetricsCache: CacheType created");

        println!("✅ FontMetricsCache: Returning Self");
        Ok(Self { cache_type })
    }

    pub async fn get(&self, font_id: fontdb::ID, font_size: f32) -> Option<FontMetrics> {
        let key = FontMetricsCacheKey::new(font_id, font_size);
        match &self.cache_type {
            CacheType::Goldylox(cache) => cache.get(&key).await,
            CacheType::HashMap(map) => {
                if let Ok(map) = map.lock() {
                    map.get(&key).cloned()
                } else {
                    None
                }
            }
        }
    }

    pub async fn put(
        &self,
        font_id: fontdb::ID,
        font_size: f32,
        metrics: FontMetrics,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let key = FontMetricsCacheKey::new(font_id, font_size);
        match &self.cache_type {
            CacheType::Goldylox(cache) => {
                cache
                    .put(key, metrics).await
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
            }
            CacheType::HashMap(map) => {
                if let Ok(mut map) = map.lock() {
                    // Implement simple LRU-like behavior with size limit
                    if map.len() >= 1000 {
                        // Remove some entries when cache gets too large
                        let keys_to_remove: Vec<_> = map.keys().take(200).cloned().collect();
                        for key_to_remove in keys_to_remove {
                            map.remove(&key_to_remove);
                        }
                    }
                    map.insert(key, metrics);
                    Ok(())
                } else {
                    Err("Failed to acquire cache lock".into())
                }
            }
        }
    }

    pub async fn clear(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match &self.cache_type {
            CacheType::Goldylox(cache) => {
                cache
                    .clear().await
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
            }
            CacheType::HashMap(map) => {
                if let Ok(mut map) = map.lock() {
                    map.clear();
                    Ok(())
                } else {
                    Err("Failed to acquire cache lock".into())
                }
            }
        }
    }

    pub fn len(&self) -> usize {
        match &self.cache_type {
            CacheType::Goldylox(_) => {
                // Goldylox doesn't expose len() - return 0 as placeholder
                0
            }
            CacheType::HashMap(map) => {
                if let Ok(map) = map.lock() {
                    map.len()
                } else {
                    0
                }
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        // Goldylox doesn't expose len() - return true as placeholder
        true
    }
}

impl Default for FontMetricsCache {
    fn default() -> Self {
        // Since new() is async and Default can't be async, we use a blocking approach
        use tokio::runtime::Handle;
        
        // Try to use current runtime if available
        if let Ok(handle) = Handle::try_current() {
            handle.block_on(async {
                Self::new().await.unwrap_or_else(|_| {
                    // Fallback: create a minimal HashMap-based cache that always works
                    FontMetricsCache {
                        cache_type: CacheType::HashMap(Mutex::new(HashMap::new())),
                    }
                })
            })
        } else {
            // No runtime available, create one temporarily
            tokio::runtime::Runtime::new()
                .expect("Failed to create tokio runtime")
                .block_on(async {
                    Self::new().await.unwrap_or_else(|_| {
                        FontMetricsCache {
                            cache_type: CacheType::HashMap(Mutex::new(HashMap::new())),
                        }
                    })
                })
        }
    }
}

/// Font metrics calculator using goldylox caching
pub struct FontMetricsCalculator {
    cache: FontMetricsCache,
}

impl FontMetricsCalculator {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let cache = FontMetricsCache::new().await?;
        Ok(Self { cache })
    }

    pub async fn calculate(&self, font_id: fontdb::ID, font_size: f32) -> Option<FontMetrics> {
        if let Some(cached) = self.cache.get(font_id, font_size).await {
            return Some(cached);
        }

        // Calculate font metrics - simplified implementation
        let metrics = FontMetrics {
            units_per_em: 1000,
            ascent: (font_size * 0.8) as i16,
            descent: -(font_size * 0.2) as i16,
            line_gap: 0,
            x_height: Some((font_size * 0.5) as i16),
            cap_height: Some((font_size * 0.7) as i16),
            ideographic_baseline: Some(-(font_size * 0.1) as i16),
            hanging_baseline: Some((font_size * 0.8) as i16),
            mathematical_baseline: Some((font_size * 0.4) as i16),
            average_char_width: font_size * 0.6,
            max_char_width: font_size * 1.2,
            underline_position: font_size * -0.1,
            underline_thickness: font_size * 0.05,
            strikethrough_position: font_size * 0.4,
            strikethrough_thickness: font_size * 0.05,
        };

        if let Err(e) = self.cache.put(font_id, font_size, metrics.clone()).await {
            eprintln!("Warning: Failed to cache font metrics: {}", e);
        }

        Some(metrics)
    }

    pub async fn get_cached(&self, font_id: fontdb::ID, font_size: f32) -> Option<FontMetrics> {
        self.cache.get(font_id, font_size).await
    }

    pub async fn clear_cache(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.cache.clear().await
    }

    /// Extract comprehensive font metrics for given attributes
    pub fn extract_comprehensive_font_metrics(
        &self,
        attrs: &cosmyc_text::Attrs,
        font_system: &mut cosmyc_text::FontSystem,
    ) -> Result<FontMetrics, Box<dyn std::error::Error + Send + Sync>> {
        // Extract font family from attrs
        let font_family = match attrs.family {
            cosmyc_text::Family::Name(name) => name,
            cosmyc_text::Family::Serif => "serif",
            cosmyc_text::Family::SansSerif => "sans-serif",
            cosmyc_text::Family::Cursive => "cursive",
            cosmyc_text::Family::Fantasy => "fantasy",
            cosmyc_text::Family::Monospace => "monospace",
        };
        
        // Create query from attrs
        let query = cosmyc_text::fontdb::Query {
            families: &[cosmyc_text::fontdb::Family::Name(font_family)],
            weight: cosmyc_text::fontdb::Weight(attrs.weight.0),
            stretch: attrs.stretch,
            style: match attrs.style {
                cosmyc_text::Style::Normal => cosmyc_text::fontdb::Style::Normal,
                cosmyc_text::Style::Italic => cosmyc_text::fontdb::Style::Italic,
                cosmyc_text::Style::Oblique => cosmyc_text::fontdb::Style::Oblique,
            },
        };
        
        // Get font ID from database
        let font_id = font_system
            .db()
            .query(&query)
            .ok_or("Font not found for given attributes")?;
        
        // Extract metrics using existing helper (see font_metrics.rs)
        let mut result = None;
        font_system
            .db_mut()
            .with_face_data(font_id, |font_data, face_index| {
                if let Ok(face) = ttf_parser::Face::parse(font_data, face_index) {
                    result = Some(FontMetrics {
                        units_per_em: face.units_per_em(),
                        ascent: face.ascender(),
                        descent: face.descender(),
                        line_gap: face.line_gap(),
                        x_height: face.x_height(),
                        cap_height: face.capital_height(),
                        ideographic_baseline: Some((face.descender() as f32 * 0.8) as i16),
                        hanging_baseline: Some((face.ascender() as f32 * 0.9) as i16),
                        mathematical_baseline: face.x_height().map(|x| (x as f32 * 0.5) as i16),
                        average_char_width: (face.ascender() - face.descender()) as f32 * 0.5,
                        max_char_width: face.units_per_em() as f32,
                        underline_position: face.underline_metrics().map(|m| m.position as f32).unwrap_or(-100.0),
                        underline_thickness: face.underline_metrics().map(|m| m.thickness as f32).unwrap_or(50.0),
                        strikethrough_position: face.x_height().map(|x| x as f32 * 0.6).unwrap_or(face.ascender() as f32 * 0.4),
                        strikethrough_thickness: face.underline_metrics().map(|m| m.thickness as f32).unwrap_or(50.0),
                    });
                }
            });
        
        result.ok_or_else(|| "Failed to extract font metrics".into())
    }
}

impl Default for FontMetricsCalculator {
    fn default() -> Self {
        // Since new() is async and Default can't be async, we use a blocking approach
        use tokio::runtime::Handle;
        
        // Try to use current runtime if available
        if let Ok(handle) = Handle::try_current() {
            handle.block_on(async {
                Self::new().await.unwrap_or_else(|_| {
                    // Fallback: Use HashMap-based cache when goldylox initialization fails
                    FontMetricsCalculator {
                        cache: FontMetricsCache::default(),
                    }
                })
            })
        } else {
            // No runtime available, create one temporarily
            tokio::runtime::Runtime::new()
                .expect("Failed to create tokio runtime")
                .block_on(async {
                    Self::new().await.unwrap_or_else(|_| {
                        FontMetricsCalculator {
                            cache: FontMetricsCache::default(),
                        }
                    })
                })
        }
    }
}
