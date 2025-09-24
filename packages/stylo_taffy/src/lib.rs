//! Conversion functions from Stylo types to Taffy types
//!
//! This crate is an implementation detail of [`blitz-dom`](https://docs.rs/blitz-dom), but can also be
//! used standalone, and serves as useful reference for anyone wanting to integrate [`stylo`](::style) with [`taffy`]

mod wrapper;
pub use wrapper::{TaffyStyloStyle, TaffyStyloStyleMut};

pub mod convert;
// Re-export grid context types for public use
pub use convert::{GridArea, GridAxis, GridContext, MasonryPlacementState, MasonryTrackState};
#[doc(inline)]
pub use convert::{to_taffy_style, to_taffy_style_with_device, to_taffy_style_with_grid_context};
