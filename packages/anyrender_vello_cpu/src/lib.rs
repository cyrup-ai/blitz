//! An Anyrender backend using the vello_cpu crate
mod image_renderer;
mod scene;
mod window_renderer;

pub use image_renderer::VelloCpuImageRenderer;
pub use scene::VelloCpuScenePainter;
pub use window_renderer::VelloCpuWindowRenderer;

// Re-export vello_cpu based on feature flags (vendored takes precedence)
#[cfg(feature = "vendored")]
mod vendored;
#[cfg(all(feature = "external", not(feature = "vendored")))]
pub use vello_cpu;
// Re-export from vendored if available, otherwise from external
#[cfg(feature = "vendored")]
pub use vendored::vello_cpu;
#[cfg(feature = "vendored")]
use vendored::{vello_api, vello_common};
#[cfg(feature = "vendored")]
extern crate alloc;
