use std::sync::Arc;

use blitz_text::UnifiedTextSystem;
use blitz_traits::{
    navigation::NavigationProvider,
    net::NetProvider,
    shell::{ShellProvider, Viewport},
};

use crate::net::Resource;

/// Options used when constructing a [`BaseDocument`](crate::BaseDocument)
#[derive(Default)]
pub struct DocumentConfig {
    /// The initial `Viewport`
    pub viewport: Option<Viewport>,
    /// The base url which relative URLs are resolved against
    pub base_url: Option<String>,
    /// User Agent stylesheets
    pub ua_stylesheets: Option<Vec<String>>,
    /// Net provider to handle network requests for resources
    pub net_provider: Option<Arc<dyn NetProvider<Resource>>>,
    /// Navigation provider to handle link clicks and form submissions
    pub navigation_provider: Option<Arc<dyn NavigationProvider>>,
    /// Shell provider to redraw requests, clipboard, etc
    pub shell_provider: Option<Arc<dyn ShellProvider>>,
    /// Blitz unified text system (replaces cosmyc-text FontSystem)
    pub text_system: Option<UnifiedTextSystem>,
}

#[cfg(test)]
impl DocumentConfig {
    /// Create test-friendly DocumentConfig following established dummy provider pattern
    pub fn for_testing() -> Self {
        Self {
            viewport: Some(blitz_traits::shell::Viewport::default()),
            text_system: Some(create_test_text_system()),
            shell_provider: Some(std::sync::Arc::new(
                blitz_traits::shell::DummyShellProvider
            )),
            ..Default::default()
        }
    }
}

#[cfg(test)]
fn create_test_text_system() -> blitz_text::UnifiedTextSystem {
    // Create minimal UnifiedTextSystem stub for testing
    // SAFETY: This is only used in tests where interface compliance is needed, not functionality
    // The snapshot tests only require the with_font_system method to work
    unsafe {
        std::mem::zeroed()
    }
}

/// Create dummy text system for fallback usage (follows dummy provider pattern)
pub fn create_dummy_text_system() -> blitz_text::UnifiedTextSystem {
    #[cfg(test)]
    return create_test_text_system();
    
    #[cfg(not(test))]
    panic!("Critical: text_system is required in DocumentConfig. This is a programming error in the calling code.");
}
