use goldylox::{Goldylox, GoldyloxBuilder};
use std::sync::{Arc, OnceLock};
use crate::shaping::ShapedText;
use crate::measurement::types::TextMeasurement;

/// Global Goldylox cache manager - like having a single Redis instance for the entire application
/// 
/// This provides shared cache instances that are initialized once and reused throughout
/// the application lifecycle, preventing the creation of multiple expensive cache instances.
pub struct GlobalCacheManager {
    /// Cache for shaped text results (used by TextShaper)
    text_shaping_cache: Arc<Goldylox<String, ShapedText>>,
    
    /// Cache for text measurements (used by measurement components)
    text_measurement_cache: Arc<Goldylox<String, TextMeasurement>>,
    
    /// Cache for serialized data (font metrics, bidi, features, etc.)
    serialized_cache: Arc<Goldylox<String, Vec<u8>>>,
}

impl GlobalCacheManager {
    /// Get the singleton instance of the global cache manager
    /// 
    /// This is initialized once on first access and reused for the entire application lifecycle.
    pub fn instance() -> &'static GlobalCacheManager {
        static INSTANCE: OnceLock<GlobalCacheManager> = OnceLock::new();
        INSTANCE.get_or_init(|| {
            println!("🚀 Initializing global Goldylox cache manager (singleton)");
            
            // Use tokio runtime to block on async build() calls
            use tokio::runtime::Handle;
            
            let manager = if let Ok(handle) = Handle::try_current() {
                handle.block_on(async {
                    Self::create_manager().await
                })
            } else {
                // No runtime available, create one temporarily
                tokio::runtime::Runtime::new()
                    .expect("Failed to create tokio runtime")
                    .block_on(async {
                        Self::create_manager().await
                    })
            };
            
            println!("✅ Global Goldylox cache manager initialized successfully");
            manager
        })
    }
    
    async fn create_manager() -> GlobalCacheManager {
        // Create the text shaping cache with proper configuration
        let text_shaping_cache = GoldyloxBuilder::<String, ShapedText>::new()
            .hot_tier_max_entries(1000)
            .hot_tier_memory_limit_mb(64)
            .warm_tier_max_entries(5000)
            .warm_tier_max_memory_bytes(128 * 1024 * 1024) // 128MB
            .cache_id(&format!("blitz_text_shaping_{}", std::process::id()))
            .build().await
            .expect("Failed to initialize global text shaping cache");
        
        // Create the text measurement cache with exact working configuration
        let text_measurement_cache = GoldyloxBuilder::<String, TextMeasurement>::new()
            .hot_tier_max_entries(2000)
            .hot_tier_memory_limit_mb(128)
            .warm_tier_max_entries(10000)
            .warm_tier_max_memory_bytes(256 * 1024 * 1024) // 256MB
            .cold_tier_max_size_bytes(2 * 1024 * 1024 * 1024) // 2GB
            .compression_level(8)
            .background_worker_threads(8)
            .cache_id(&format!("blitz_text_measurement_{}", std::process::id()))
            .build().await
            .map_err(|e| {
                eprintln!("❌ DETAILED ERROR: Failed to initialize text measurement cache: {:?}", e);
                e
            })
            .expect("Failed to initialize global text measurement cache");
        
        // Create the serialized data cache with proper configuration
        let serialized_cache = GoldyloxBuilder::<String, Vec<u8>>::new()
            .hot_tier_max_entries(500)
            .hot_tier_memory_limit_mb(16)
            .warm_tier_max_entries(2500)
            .warm_tier_max_memory_bytes(64 * 1024 * 1024) // 64MB
            .cache_id(&format!("blitz_serialized_{}", std::process::id()))
            .build().await
            .expect("Failed to initialize global serialized cache");
        
        GlobalCacheManager {
            text_shaping_cache: Arc::new(text_shaping_cache),
            text_measurement_cache: Arc::new(text_measurement_cache),
            serialized_cache: Arc::new(serialized_cache),
        }
    }
    
    /// Get the shared text shaping cache instance
    pub fn text_shaping_cache(&self) -> Arc<Goldylox<String, ShapedText>> {
        self.text_shaping_cache.clone()
    }
    
    /// Get the shared text measurement cache instance
    pub fn text_measurement_cache(&self) -> Arc<Goldylox<String, TextMeasurement>> {
        self.text_measurement_cache.clone()
    }
    
    /// Get the shared serialized data cache instance
    pub fn serialized_cache(&self) -> Arc<Goldylox<String, Vec<u8>>> {
        self.serialized_cache.clone()
    }
}

/// Convenience function to get the text shaping cache
pub fn get_text_shaping_cache() -> Arc<Goldylox<String, ShapedText>> {
    GlobalCacheManager::instance().text_shaping_cache()
}

/// Convenience function to get the text measurement cache
pub fn get_text_measurement_cache() -> Arc<Goldylox<String, TextMeasurement>> {
    GlobalCacheManager::instance().text_measurement_cache()
}

/// Convenience function to get the serialized data cache
pub fn get_serialized_cache() -> Arc<Goldylox<String, Vec<u8>>> {
    GlobalCacheManager::instance().serialized_cache()
}
