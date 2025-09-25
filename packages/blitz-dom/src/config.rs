use std::sync::Arc;


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
    // text_system is now managed internally by BaseDocument - no longer in config
}

#[cfg(test)]
impl DocumentConfig {
    /// Create test-friendly DocumentConfig following established dummy provider pattern
    pub fn for_testing() -> Self {
        Self {
            viewport: Some(blitz_traits::shell::Viewport::default()),
            shell_provider: Some(std::sync::Arc::new(
                blitz_traits::shell::DummyShellProvider
            )),
            ..Default::default()
        }
    }
}

// create_dummy_text_system and create_test_text_system functions removed entirely - no longer needed
