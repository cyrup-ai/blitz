//! Production NavigationProvider implementation for handling navigation events

use std::sync::{Arc, Mutex};
use blitz_traits::navigation::{NavigationProvider, NavigationOptions};
use url::Url;

/// Production navigation provider that maintains navigation history and handles navigation events
#[derive(Debug, Clone)]
pub struct BlitzNavigationProvider {
    /// Navigation history stack
    history: Arc<Mutex<Vec<Url>>>,
    /// Current position in navigation history
    current_index: Arc<Mutex<usize>>,
}

impl BlitzNavigationProvider {
    /// Create a new BlitzNavigationProvider with empty history
    pub fn new() -> Self {
        Self {
            history: Arc::new(Mutex::new(Vec::new())),
            current_index: Arc::new(Mutex::new(0)),
        }
    }

    /// Get the current URL from navigation history
    pub fn current_url(&self) -> Option<Url> {
        if let (Ok(history), Ok(index)) = (self.history.lock(), self.current_index.lock()) {
            history.get(*index).cloned()
        } else {
            None
        }
    }

    /// Check if navigation can go back in history
    pub fn can_go_back(&self) -> bool {
        if let Ok(index) = self.current_index.lock() {
            *index > 0
        } else {
            false
        }
    }

    /// Check if navigation can go forward in history
    pub fn can_go_forward(&self) -> bool {
        if let (Ok(history), Ok(index)) = (self.history.lock(), self.current_index.lock()) {
            *index + 1 < history.len()
        } else {
            false
        }
    }

    /// Navigate back in history
    pub fn go_back(&self) -> Option<Url> {
        if let (Ok(history), Ok(mut index)) = (self.history.lock(), self.current_index.lock()) {
            if *index > 0 {
                *index -= 1;
                return history.get(*index).cloned();
            }
        }
        None
    }

    /// Navigate forward in history
    pub fn go_forward(&self) -> Option<Url> {
        if let (Ok(history), Ok(mut index)) = (self.history.lock(), self.current_index.lock()) {
            if *index + 1 < history.len() {
                *index += 1;
                return history.get(*index).cloned();
            }
        }
        None
    }
}

impl Default for BlitzNavigationProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl NavigationProvider for BlitzNavigationProvider {
    fn navigate_to(&self, options: NavigationOptions) {
        // Validate URL before navigation
        let url = options.url.clone();
        
        // Handle navigation based on URL scheme
        match url.scheme() {
            "http" | "https" | "file" | "data" => {
                // Add to navigation history
                if let (Ok(mut history), Ok(mut index)) = (self.history.lock(), self.current_index.lock()) {
                    // If we're not at the end of history, truncate forward entries
                    if *index + 1 < history.len() {
                        history.truncate(*index + 1);
                    }
                    
                    // Add new URL to history
                    history.push(url.clone());
                    *index = history.len() - 1;
                }
                
                // Convert to HTTP request if needed for actual navigation
                let _request = options.into_request();
                
                // In a full implementation, this would emit navigation events
                // to parent application via callback or event system
                #[cfg(feature = "tracing")]
                tracing::info!("Navigating to: {}", url);
            }
            scheme => {
                // Unsupported scheme
                #[cfg(feature = "tracing")]
                tracing::warn!("Unsupported navigation scheme: {}", scheme);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_navigation_provider_creation() {
        let provider = BlitzNavigationProvider::new();
        assert!(provider.current_url().is_none());
        assert!(!provider.can_go_back());
        assert!(!provider.can_go_forward());
    }

    #[test] 
    fn test_navigation_history() {
        let provider = BlitzNavigationProvider::new();
        let url1 = Url::parse("https://example.com").unwrap();
        let url2 = Url::parse("https://example.org").unwrap();
        
        let options1 = NavigationOptions::new(url1.clone(), "text/html".to_string(), 1);
        let options2 = NavigationOptions::new(url2.clone(), "text/html".to_string(), 1);
        
        provider.navigate_to(options1);
        provider.navigate_to(options2);
        
        assert_eq!(provider.current_url(), Some(url2));
        assert!(provider.can_go_back());
        assert!(!provider.can_go_forward());
        
        let back_url = provider.go_back();
        assert_eq!(back_url, Some(url1));
        assert!(provider.can_go_forward());
    }
}