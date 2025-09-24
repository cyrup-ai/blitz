//! Lock-free OpenType feature settings and script-specific configurations

pub mod cache;
pub mod custom;
pub mod lookup;
pub mod registry;
pub mod scripts;
pub mod types;

// Re-export main types and functions for API compatibility
pub use cache::FeaturesCache;
pub use custom::CustomFeatures;
pub use lookup::FeatureLookup;
pub use registry::FEATURE_REGISTRY;
pub use types::{FeatureSettings, DEFAULT_FEATURES};
