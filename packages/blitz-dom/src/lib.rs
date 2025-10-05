//! The core DOM abstraction in Blitz
//!
//! This crate implements a flexible headless DOM ([`BaseDocument`]), which is designed to emebedded in and "driven" by external code. Most users will want
//! to use a wrapper:
//!
//!  - [`HtmlDocument`](https://docs.rs/blitz-html/latest/blitz_html/struct.HtmlDocument.html) from the [blitz-html](https://docs.rs/blitz-html) crate.
//!    Allows you to parse HTML (or XHTML) into a Blitz [`BaseDocument`], and can be combined with a markdown-to-html converter like [comrak](https://docs.rs/comrak)
//!    or [pulldown-cmark](https://docs.rs/pulldown-cmark) to render/process markdown.
//!  - [`DioxusDocument`](https://docs.rs/dioxus-native/latest/dioxus_native/struct.DioxusDocument.html) from the [dioxus-native](https://docs.rs/dioxus-native) crate.
//!    Combines a [`BaseDocument`] with a Dioxus `VirtualDom` to enable dynamic rendering and event handling.
//!
//! It includes: A DOM tree respresentation, CSS parsing and resolution, layout and event handling. Additional functionality is available in
//! separate crates, including html parsing ([blitz-html](https://docs.rs/blitz-html)), networking ([blitz-net](https://docs.rs/blitz-html)),
//! rendering ([blitz-paint](https://docs.rs/blitz-paint)) and windowing ([blitz-shell](https://docs.rs/blitz-shell)).
//!
//! Most of the functionality in this crates is provided through the  struct.
//!
//! `blitz-dom` has a native Rust API that is designed for higher-level abstractions to be built on top (although it can also be used directly).
//!
//! The goal behind this crate is that any implementor can interact with the DOM and render it out using any renderer
//! they want.

// TODO: Document features
// ## Feature flags
//  - `default`: Enables the features listed below.
//  - `tracing`: Enables tracing support.

pub const DEFAULT_CSS: &str = include_str!("../assets/default.css");
pub(crate) const BULLET_FONT: &[u8] = include_bytes!("../assets/moz-bullet-font.otf");

/// The DOM implementation.
///
/// This is the primary entry point for this crate.
mod document;

/// The nodes themsleves, and their data.
pub mod node;

pub mod atom_utils;
mod config;
mod debug;
mod events;
mod form;
/// Integration of taffy and the DOM.
pub mod layout;
mod mutator;
pub mod navigation;
mod query_selector;
/// Implementations that interact with servo's style engine
mod stylo;
pub mod stylo_to_cursor_icon;
/// High-performance text system singleton
mod text_system_singleton;
mod traversal;
mod url;

pub mod net;
pub mod util;

#[cfg(feature = "accessibility")]
mod accessibility;

pub use config::DocumentConfig;
pub use document::{BaseDocument, Document};
pub use markup5ever::{
    LocalName, Namespace, NamespaceStaticSet, Prefix, PrefixStaticSet, QualName, local_name,
    namespace_prefix, namespace_url, ns,
};
pub use mutator::DocumentMutator;
pub use node::{Attribute, ElementData, Node, NodeData, TextNodeData};
// FontContext has been replaced with cosmyc-text FontSystem
pub use style::Atom;
pub use style::invalidation::element::restyle_hints::RestyleHint;
pub type SelectorList = selectors::SelectorList<style::selector_parser::SelectorImpl>;
pub use events::{EventDriver, EventHandler, NoopEventHandler};
pub use navigation::BlitzNavigationProvider;
pub use text_system_singleton::{TextSystemSingleton, TextSystemSingletonError};

use std::sync::Arc;
use blitz_traits::navigation::NavigationProvider;

/// Create a default navigation provider for production use
pub fn create_default_navigation_provider() -> Arc<dyn NavigationProvider> {
    Arc::new(BlitzNavigationProvider::default())
}

/// Create a DocumentConfig with production-ready defaults where possible
/// 
/// Note: net_provider and shell_provider must still be provided by the caller
/// as they require external dependencies (blitz-net and blitz-shell crates)
pub fn create_production_config_base() -> DocumentConfig {
    DocumentConfig {
        navigation_provider: Some(create_default_navigation_provider()),
        ..Default::default()
    }
}

/// Helper to create a BaseDocument with proper error handling
/// 
/// Returns a Result to handle missing required providers
pub fn create_document(config: DocumentConfig) -> Result<BaseDocument, &'static str> {
    BaseDocument::new(config)
}

/// Create a minimal DocumentConfig suitable for testing
/// 
/// Uses the default navigation provider but requires net and shell providers
/// to be added for full functionality
pub fn create_minimal_config() -> DocumentConfig {
    DocumentConfig {
        navigation_provider: Some(create_default_navigation_provider()),
        ..Default::default()
    }
}
