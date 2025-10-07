//! Gap detection for CSS Grid Level 3 masonry dense packing
//!
//! Implements the gap detection algorithm that allows items to backfill gaps
//! while maintaining track size compatibility to avoid re-layout.

/// Represents a gap opportunity for dense packing placement
/// Per CSS Grid Level 3: gaps must match track sizes to avoid re-layout
#[derive(Debug, Clone)]
pub struct GapOpportunity {
    /// Starting track index of the gap
    pub track_index: usize,

    /// Position in masonry axis where item would be placed
    pub gap_position: f32,

    /// Total available space in the gap (masonry axis)
    pub gap_size: f32,

    /// Total track size (grid axis) - must match item's normal placement
    pub track_total_size: f32,

    /// Number of tracks spanned by this gap
    pub span: usize,
}

/// Detect gap opportunities for dense packing placement
/// Returns gaps sorted by position (earliest first) for CSS spec compliance
pub fn detect_compatible_gaps(
    masonry_state: &stylo_taffy::MasonryTrackState,
    track_sizes: &[f32],
    item_span: usize,
    item_masonry_size: f32,
    normal_placement_track_size: f32,
    item_tolerance: f32,
) -> Vec<GapOpportunity> {
    let mut gaps = Vec::new();

    // Find maximum track position (the "leading edge" of the layout)
    let max_position = masonry_state
        .track_positions
        .iter()
        .copied()
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or(0.0);

    // Check each potential starting position for gaps
    for start_track in 0..=(masonry_state.track_count.saturating_sub(item_span)) {
        // Calculate span range
        let end_track = start_track + item_span;

        // Get track positions for all spanned tracks
        let spanned_positions: Vec<f32> = (start_track..end_track)
            .map(|i| masonry_state.get_track_position(i))
            .collect();

        // Gap position is the maximum position among spanned tracks
        let gap_position = spanned_positions
            .iter()
            .copied()
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(0.0);

        // Gap size is distance from gap_position to max_position
        let gap_size = max_position - gap_position;

        // Check if gap is large enough for item (with tolerance)
        if gap_size < (item_masonry_size - item_tolerance) {
            continue; // Gap too small
        }

        // Calculate total track size for spanned tracks (grid axis)
        let track_total_size: f32 = (start_track..end_track)
            .map(|i| track_sizes.get(i).copied().unwrap_or(0.0))
            .sum();

        // CRITICAL: Track size must match normal placement per CSS spec
        // "the spanned tracks have the same total used size as the tracks
        // into which it is currently placed"
        let size_difference = (track_total_size - normal_placement_track_size).abs();
        if size_difference > 0.1 {
            continue; // Track sizes don't match, would require re-layout
        }

        // Check if this is actually a gap (not the current leading position)
        if gap_size > item_tolerance {
            gaps.push(GapOpportunity {
                track_index: start_track,
                gap_position,
                gap_size,
                track_total_size,
                span: item_span,
            });
        }
    }

    // Sort by position (earliest first) per CSS spec
    gaps.sort_by(|a, b| {
        a.gap_position
            .partial_cmp(&b.gap_position)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.track_index.cmp(&b.track_index))
    });

    gaps
}
