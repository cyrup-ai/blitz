//! High-performance text system singleton with zero-allocation access patterns
//!
//! This module provides a lock-free, thread-safe singleton for the UnifiedTextSystem
//! that eliminates initialization overhead during render loops and prevents multiple
//! initialization attempts that cause cache failures.

use tokio::sync::OnceCell;
use blitz_text::{UnifiedTextSystem, text_system::config::TextSystemError, cosmyc};
use wgpu::{Device, Queue, TextureFormat, MultisampleState, DepthStencilState};

/// Custom error type for text system singleton operations
#[derive(Debug, Clone)]
pub enum TextSystemSingletonError {
    /// Text system has not been initialized yet
    NotInitialized,
    /// Text system initialization failed
    InitializationFailed(String),
    /// GPU context is invalid or unavailable
    InvalidGpuContext,
}

impl std::fmt::Display for TextSystemSingletonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotInitialized => write!(f, "Text system not initialized - call initialize_once() first"),
            Self::InitializationFailed(msg) => write!(f, "Text system initialization failed: {}", msg),
            Self::InvalidGpuContext => write!(f, "Invalid or unavailable GPU context"),
        }
    }
}

impl std::error::Error for TextSystemSingletonError {}

impl From<TextSystemError> for TextSystemSingletonError {
    fn from(err: TextSystemError) -> Self {
        Self::InitializationFailed(err.to_string())
    }
}

/// Lock-free, thread-safe singleton for UnifiedTextSystem
/// 
/// Uses OnceLock for zero-allocation initialization and ThreadLocal patterns
/// within UnifiedTextSystem for thread-safe access without contention.
pub struct TextSystemSingleton;

impl TextSystemSingleton {
    /// Global singleton instance using async-aware initialization
    /// 
    /// OnceCell ensures initialization happens exactly once across all threads
    /// atomically, preventing race conditions during async initialization.
    fn instance() -> &'static OnceCell<UnifiedTextSystem> {
        static INSTANCE: OnceCell<UnifiedTextSystem> = OnceCell::const_new();
        &INSTANCE
    }

    /// Initialize the text system singleton with GPU context
    /// 
    /// This method is idempotent and race-free - calling it multiple times is safe.
    /// Only ONE thread will ever execute the initialization closure, eliminating
    /// the race condition that caused multiple goldylox cache initializations.
    /// 
    /// # Arguments
    /// * `device` - WGPU device for GPU operations
    /// * `queue` - WGPU command queue
    /// * `format` - Surface texture format
    /// * `multisample` - Multisampling configuration
    /// * `depth_stencil` - Optional depth/stencil configuration
    /// 
    /// # Returns
    /// * `Ok(())` - Initialization successful or already initialized
    /// * `Err(TextSystemSingletonError)` - Initialization failed
    #[inline]
    pub async fn initialize_once(
        device: &Device,
        queue: &Queue,
        format: TextureFormat,
        multisample: MultisampleState,
        depth_stencil: Option<DepthStencilState>,
    ) -> Result<(), TextSystemSingletonError> {
        let instance = Self::instance();
        
        // Atomic async initialization - only ONE thread executes the closure
        instance.get_or_try_init(|| async {
            println!("🚀 TextSystemSingleton::initialize_once - initializing for first time");
            UnifiedTextSystem::new(
                device,
                queue,
                format,
                multisample,
                depth_stencil,
            ).await.map_err(TextSystemSingletonError::from)
        }).await?;
        
        println!("✅ TextSystemSingleton initialized successfully");
        Ok(())
    }

    /// Check if the text system singleton is initialized
    /// 
    /// Zero-allocation status check for conditional initialization logic.
    #[inline]
    pub fn is_initialized() -> bool {
        Self::instance().get().is_some()
    }

    /// Access the text system singleton with a closure
    /// 
    /// Provides safe, zero-allocation access to the singleton for read operations.
    /// The UnifiedTextSystem uses ThreadLocal patterns internally for thread safety.
    /// 
    /// # Arguments
    /// * `f` - Closure to execute with text system reference
    /// 
    /// # Returns
    /// * `Ok(R)` - Result of closure execution
    /// * `Err(TextSystemSingletonError::NotInitialized)` - Singleton not initialized
    #[inline]
    pub fn with_text_system<R, F>(f: F) -> Result<R, TextSystemSingletonError>
    where
        F: FnOnce(&UnifiedTextSystem) -> R,
    {
        match Self::instance().get() {
            Some(text_system) => Ok(f(text_system)),
            None => Err(TextSystemSingletonError::NotInitialized),
        }
    }

    /// Access the text system singleton with mutable closure
    /// 
    /// Provides safe access for operations that need mutable access.
    /// The UnifiedTextSystem uses interior mutability patterns (RefCell, ThreadLocal)
    /// so mutable operations work through immutable references.
    /// 
    /// # Arguments
    /// * `f` - Closure to execute with text system reference
    /// 
    /// # Returns
    /// * `Ok(R)` - Result of closure execution
    /// * `Err(TextSystemSingletonError::NotInitialized)` - Singleton not initialized
    #[inline]
    pub fn with_text_system_mut<R, F>(f: F) -> Result<R, TextSystemSingletonError>
    where
        F: FnOnce(&UnifiedTextSystem) -> R,
    {
        // UnifiedTextSystem uses interior mutability (ThreadLocal<RefCell<_>>)
        // so mutable operations work through immutable references
        Self::with_text_system(f)
    }

    /// Get direct reference to the text system (advanced usage)
    /// 
    /// Returns a direct reference for cases where closure-based access
    /// is not suitable. Use with caution in performance-critical paths.
    #[inline]
    pub fn get() -> Option<&'static UnifiedTextSystem> {
        Self::instance().get()
    }

    /// Check if singleton can be initialized (for testing)
    /// 
    /// Returns true if the singleton is not yet initialized, false if it is.
    /// This is useful for conditional test logic that respects singleton state.
    #[cfg(test)]
    pub fn can_initialize() -> bool {
        !Self::is_initialized()
    }
}

/// Convenience functions for common text system operations
impl TextSystemSingleton {
    /// Access font system through the singleton
    /// 
    /// Provides zero-allocation access to the underlying font system
    /// using the ThreadLocal pattern for thread safety.
    #[inline]
    pub fn with_font_system<R, F>(f: F) -> Result<R, TextSystemSingletonError>
    where
        F: FnOnce(&mut cosmyc::FontSystem) -> R,
    {
        Self::with_text_system(|text_system| {
            text_system.with_font_system(f)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_not_initialized() {
        let err = TextSystemSingletonError::NotInitialized;
        let display_str = err.to_string();
        assert!(display_str.contains("not initialized"));
        assert!(display_str.contains("initialize_once()"));
    }

    #[test]
    fn test_error_display_initialization_failed() {
        let err = TextSystemSingletonError::InitializationFailed("test error".to_string());
        let display_str = err.to_string();
        assert!(display_str.contains("initialization failed"));
        assert!(display_str.contains("test error"));
    }

    #[test]
    fn test_error_display_invalid_gpu_context() {
        let err = TextSystemSingletonError::InvalidGpuContext;
        let display_str = err.to_string();
        assert!(display_str.contains("Invalid"));
        assert!(display_str.contains("GPU context"));
    }

    #[test]
    fn test_error_from_text_system_error() {
        use blitz_text::text_system::config::TextSystemError;
        let text_system_err = TextSystemError::Configuration("test config error".to_string());
        let singleton_err = TextSystemSingletonError::from(text_system_err);
        
        match singleton_err {
            TextSystemSingletonError::InitializationFailed(msg) => {
                assert!(msg.contains("test config error"));
            }
            _ => panic!("Expected InitializationFailed variant"),
        }
    }

    #[test]
    fn test_singleton_access_before_initialization() {
        // This test works regardless of singleton state by testing the error path
        let result = TextSystemSingleton::with_text_system(|_| "test");
        
        // If singleton is not initialized, we should get NotInitialized error
        // If it is initialized (from other tests), the closure should execute
        match result {
            Ok(_) => {
                // Singleton was already initialized - this is fine
                assert!(TextSystemSingleton::is_initialized());
            }
            Err(TextSystemSingletonError::NotInitialized) => {
                // Singleton not initialized - this is the expected error path
                assert!(!TextSystemSingleton::is_initialized());
            }
            Err(other) => panic!("Unexpected error: {:?}", other),
        }
    }

    #[test]
    fn test_with_text_system_mut_delegates_correctly() {
        // Test that with_text_system_mut properly delegates to with_text_system
        let result1 = TextSystemSingleton::with_text_system(|_| 42);
        let result2 = TextSystemSingleton::with_text_system_mut(|_| 42);
        
        // Both should have the same result type and behavior
        match (result1, result2) {
            (Ok(val1), Ok(val2)) => assert_eq!(val1, val2),
            (Err(_), Err(_)) => {}, // Both failed as expected
            _ => panic!("with_text_system and with_text_system_mut should behave identically"),
        }
    }

    #[test]
    fn test_get_method_consistency() {
        // Test that get() method is consistent with is_initialized()
        let is_init = TextSystemSingleton::is_initialized();
        let get_result = TextSystemSingleton::get();
        
        assert_eq!(is_init, get_result.is_some());
    }
}