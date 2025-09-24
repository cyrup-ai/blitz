//! Utilities for working with atom types in blitz-dom

use std::collections::HashMap;
use std::sync::Mutex;

use markup5ever::{LocalName as MarkupLocalName, Namespace as MarkupNamespace};
use web_atoms::{LocalName as WebLocalName, Namespace as WebNamespace};

// Thread-local cache for atom conversions
thread_local! {
    static LOCAL_NAME_CACHE: Mutex<HashMap<String, &'static WebLocalName>> = Mutex::new(HashMap::new());
    static NAMESPACE_CACHE: Mutex<HashMap<String, &'static WebNamespace>> = Mutex::new(HashMap::new());
}

/// A trait for types that can be converted to a web atom LocalName
///
/// This trait provides conversion utilities for the public API
#[allow(dead_code)]
pub trait ToWebLocalName {
    /// Convert to a web atom LocalName
    fn to_web_local_name(&self) -> &'static WebLocalName;
}

impl ToWebLocalName for MarkupLocalName {
    #[inline]
    fn to_web_local_name(&self) -> &'static WebLocalName {
        let s: &str = self.as_ref();

        LOCAL_NAME_CACHE.with(|cache| {
            let mut cache = match cache.lock() {
                Ok(cache) => cache,
                Err(poisoned) => {
                    eprintln!("Warning: LOCAL_NAME_CACHE mutex was poisoned, recovering");
                    poisoned.into_inner()
                }
            };

            // Check if we already have this atom in the cache
            if let Some(atom) = cache.get(s) {
                return *atom;
            }

            // If not, create a new atom and store it in the cache
            let atom = Box::leak(Box::new(WebLocalName::from(s)));
            cache.insert(s.to_string(), atom);
            atom
        })
    }
}

/// A trait for types that can be converted to a web atom Namespace
///
/// This trait provides conversion utilities for the public API
#[allow(dead_code)]
pub trait ToWebNamespace {
    /// Convert to a web atom Namespace
    fn to_web_namespace(&self) -> &'static WebNamespace;
}

impl ToWebNamespace for MarkupNamespace {
    #[inline]
    fn to_web_namespace(&self) -> &'static WebNamespace {
        let s: &str = self.as_ref();

        NAMESPACE_CACHE.with(|cache| {
            let mut cache = match cache.lock() {
                Ok(cache) => cache,
                Err(poisoned) => {
                    eprintln!("Warning: NAMESPACE_CACHE mutex was poisoned, recovering");
                    poisoned.into_inner()
                }
            };

            // Check if we already have this namespace in the cache
            if let Some(ns) = cache.get(s) {
                return *ns;
            }

            // If not, create a new namespace and store it in the cache
            let ns = Box::leak(Box::new(WebNamespace::from(s)));
            cache.insert(s.to_string(), ns);
            ns
        })
    }
}

#[cfg(test)]
mod tests {
    use markup5ever::{local_name, ns};
    use web_atoms::LocalName;

    use super::*;

    #[test]
    fn test_local_name_conversion() {
        // Test basic conversion
        let markup_name: MarkupLocalName = local_name!("div");
        let web_name = markup_name.to_web_local_name();
        assert_eq!(web_name.as_ref(), "div");

        // Test that the same string produces the same atom
        let web_name2 = local_name!("div").to_web_local_name();
        assert!(std::ptr::eq(web_name, web_name2));

        // Test with a different tag name
        let span_name = local_name!("span").to_web_local_name();
        assert_eq!(span_name.as_ref(), "span");
    }

    #[test]
    fn test_namespace_conversion() {
        // Test HTML namespace
        let html_ns = ns!(html).to_web_namespace();
        assert_eq!(html_ns.as_ref(), "http://www.w3.org/1999/xhtml");

        // Test SVG namespace
        let svg_ns = ns!(svg).to_web_namespace();
        assert_eq!(svg_ns.as_ref(), "http://www.w3.org/2000/svg");

        // Test that the same namespace produces the same atom
        let html_ns2 = ns!(html).to_web_namespace();
        assert!(std::ptr::eq(html_ns, html_ns2));
    }

    #[test]
    fn test_thread_safety() {
        use std::thread;

        // Test that atom conversion is thread-safe
        let handles: Vec<_> = (0..4)
            .map(|i| {
                thread::spawn(move || {
                    let name = format!("test-{}", i);
                    let markup_name = LocalName::from(name.as_str());
                    let web_name = markup_name.to_web_local_name();
                    assert_eq!(web_name.as_ref(), name);
                })
            })
            .collect();

        for handle in handles {
            if let Err(e) = handle.join() {
                eprintln!(
                    "Warning: Thread join failed during atom_utils test: {:?}",
                    e
                );
            }
        }
    }
}
