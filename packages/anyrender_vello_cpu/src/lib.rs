//! An Anyrender backend using the vello_cpu crate
mod image_renderer;
mod scene;
mod window_renderer;

pub use image_renderer::VelloCpuImageRenderer;
pub use scene::VelloCpuScenePainter;
pub use window_renderer::VelloCpuWindowRenderer;

// Re-export vello_cpu based on feature flags (vendored uses GitHub forks)
#[cfg(feature = "vendored")]
pub use vello_cpu_fork as vello_cpu;
#[cfg(all(feature = "external", not(feature = "vendored")))]
pub use vello_cpu;

// Use external GitHub fork dependencies when vendored feature is enabled
#[cfg(feature = "vendored")]
extern crate alloc;
