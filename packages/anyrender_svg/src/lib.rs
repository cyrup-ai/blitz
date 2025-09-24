// Copyright 2023 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Render an SVG into any impl of [`anyrender::PaintScene`].
//!
//! This currently lacks support for some important SVG features. Known missing features include: masking, filter effects, group backgrounds
//! path shape-rendering, and patterns.

// LINEBENDER LINT SET - lib.rs - v1
// See https://linebender.org/wiki/canonical-lints/
// These lints aren't included in Cargo.toml because they
// shouldn't apply to examples and tests
#![warn(unused_crate_dependencies)]
#![warn(clippy::print_stdout, clippy::print_stderr)]
// END LINEBENDER LINT SET
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
// The following lints are part of the Linebender standard set,
// but resolving them has been deferred for now.
// Feel free to send a PR that solves one or more of these.
#![allow(missing_docs, clippy::shadow_unrelated, clippy::missing_errors_doc)]
#![cfg_attr(test, allow(unused_crate_dependencies))] // Some dev dependencies are only used in tests

mod error;
mod render;
mod util;

use anyrender::PaintScene;
pub use error::Error;
use kurbo::Affine;
pub use usvg;

/// Append an SVG to an [`anyrender::PaintScene`].
///
/// This will draw a red box over (some) unsupported elements.
pub fn render_svg_str<S: PaintScene>(
    scene: &mut S,
    svg: &str,
    transform: Affine,
) -> Result<(), Error> {
    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_str(svg, &opt)?;
    render_svg_tree(scene, &tree, transform);
    Ok(())
}

/// Append an SVG to an [`anyrender::PaintScene`] (with custom error handling).
///
/// See the [module level documentation](crate#unsupported-features) for a list of some unsupported svg features
pub fn render_svg_str_with<S: PaintScene, F: FnMut(&mut S, &usvg::Node)>(
    scene: &mut S,
    svg: &str,
    transform: Affine,
    error_handler: &mut F,
) -> Result<(), Error> {
    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_str(svg, &opt)?;
    render_svg_tree_with(scene, &tree, transform, error_handler);
    Ok(())
}

/// Append a [`usvg::Tree`] to an [`anyrender::PaintScene`].
///
/// This will draw a red box over (some) unsupported elements.
pub fn render_svg_tree<S: PaintScene>(scene: &mut S, svg: &usvg::Tree, transform: Affine) {
    render_svg_tree_with(scene, svg, transform, &mut util::default_error_handler);
}

/// Append a [`usvg::Tree`] to an [`anyrender::PaintScene`] (with custom error handling).
///
/// See the [module level documentation](crate#unsupported-features) for a list of some unsupported svg features
pub fn render_svg_tree_with<S: PaintScene, F: FnMut(&mut S, &usvg::Node)>(
    scene: &mut S,
    svg: &usvg::Tree,
    transform: Affine,
    error_handler: &mut F,
) {
    render::render_group(
        scene,
        svg.root(),
        Affine::IDENTITY,
        transform,
        error_handler,
    );
}

#[cfg(test)]
mod tests {
    // CI will fail unless cargo nextest can execute at least one test per workspace.
    // Delete this dummy test once we have an actual real test.
    #[test]
    fn dummy_test_until_we_have_a_real_test() {}
}
