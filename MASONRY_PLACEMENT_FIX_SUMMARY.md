# Masonry Item Placement Fix - Complete

## Objective ✅
Fix CSS Grid Level 3 masonry item placement algorithm to correctly implement shortest-track placement with proper tie-breaking.

## Result
**WPT Test Now PASSES:** `css/css-grid/masonry/tentative/track-sizing/auto-repeat/column-auto-repeat-001.html`

---

## Root Cause Analysis

### Issue 1: Incorrect Tie-Breaking for Single Items
**Location:** `/Volumes/samsung_t9/blitz/packages/stylo_taffy/src/convert.rs:240-282`

**Bug:** When multiple tracks had equal height, the algorithm preferred tracks `>= last_placed_track`, causing items to go to later tracks instead of earliest.

**Example:**
- Tracks: [100.0, 100.0, 100.0] (all equal after placing items 1, 2, 3)
- Item 4 should go to track 0 (earliest)
- Bug caused it to go to track 2 (>= last_placed_track of 2)

### Issue 2: Spanning Items Returning None
**Location:** `/Volumes/samsung_t9/blitz/packages/stylo_taffy/src/convert.rs:305-348`

**Bug:** `find_dense_placement` only returned `Some(track)` if placement was "significantly better" than tolerance-based. For equal spans, it returned `None`, causing fallback to wrong single-track logic.

**Example:**
- Span 2, tracks: [200.0, 200.0, 100.0]
- Span 0-1: max = 200.0
- Span 1-2: max = 200.0  
- Both equal → function returned `None` → incorrect placement

---

## Solution

### Fix 1: Simplify Tie-Breaking (Lines 240-250)
```rust
fn apply_tie_breaking_rules(&self, candidates: Vec<usize>) -> usize {
    if candidates.is_empty() {
        return 0;
    }
    if candidates.len() == 1 {
        return candidates[0];
    }
    // CSS Grid Level 3: prefer earliest track when equal
    candidates.into_iter().min().unwrap_or(0)
}
```

**Changes:**
- Removed complex rules with `last_placed_track` and `placement_cursor`
- Simply returns minimum index among candidates
- Ensures standard masonry left-to-right, top-to-bottom flow

### Fix 2: Always Return Shortest Span (Lines 283-304)
```rust
pub fn find_dense_placement(&self, item_span: usize) -> Option<usize> {
    if item_span == 0 || item_span > self.track_count {
        return None;
    }
    
    let mut best_track = 0;
    let mut best_max_position = f32::INFINITY;
    
    for track in 0..=(self.track_count.saturating_sub(item_span)) {
        let span_end = track + item_span;
        let max_position = self.track_positions[track..span_end]
            .iter()
            .fold(0.0f32, |acc, &pos| acc.max(pos));
        
        if max_position < best_max_position {
            best_max_position = max_position;
            best_track = track;
        }
    }
    
    Some(best_track)  // Always return for valid spans
}
```

**Changes:**
- Removed "significantly better" comparison check
- Always returns `Some(best_track)` for valid spans
- On ties, loop iteration order ensures earliest track wins

---

## Verification

### Placement Debug Output (Before Removal)
```
[PLACE] Item NodeId(19) span=1: tracks=[0.0, 0.0, 0.0]
[PLACE] → selected track 0 ✅

[PLACE] Item NodeId(22) span=1: tracks=[100.0, 0.0, 0.0]
[PLACE] → selected track 1 ✅

[PLACE] Item NodeId(25) span=1: tracks=[100.0, 100.0, 0.0]
[PLACE] → selected track 2 ✅

[PLACE] Item NodeId(28) span=1: tracks=[100.0, 100.0, 100.0]
[PLACE] → selected track 0 ✅ (was track 2 - FIXED!)

[PLACE] Item NodeId(31) span=1: tracks=[200.0, 100.0, 100.0]
[PLACE] → selected track 1 ✅ (was track 0 - FIXED!)

[PLACE] Item NodeId(34) span=2: tracks=[200.0, 200.0, 100.0]
[PLACE] → selected track 0 ✅ (was tracks 1-2 - FIXED!)
```

### Test Result
```
[1/1] PASS (1/1) column-auto-repeat-001.html (125ms) REF (M)
   1 subtests PASSED (100.00%)
   1 tests PASSED (100.00%)
   0 tests FAILED (0.00%)
```

### Visual Output
Perfect pixel match to reference:
- **Column 1:** Blue (1), Pink (4)
- **Column 2:** Coral (2), Orange (5)
- **Column 3:** Green (3), Gray (background where item 6 doesn't reach)
- **Row 3:** Brown (6) spanning columns 1-2

---

## Architecture Notes

### MasonryTrackState Fields
- `track_positions: Vec<f32>` - Current height of each track
- `last_placed_track: Option<usize>` - Last used track (maintained but not used in tie-breaking)
- `placement_cursor: usize` - Cursor for placement (maintained but not used in tie-breaking)
- `item_tolerance: f32` - Tolerance for "equal" heights (default 0.0)

### Placement Flow
1. **Single items:** `find_shortest_track_with_tolerance()` → `apply_tie_breaking_rules()`
2. **Spanning items:** `find_dense_placement()` directly
3. **Dense packing enabled:** Gap detection before standard placement

### Key Insight
The CSS Grid Level 3 masonry spec requires **earliest track preference** when tracks are equal. The previous implementation's optimization attempts (last_placed_track, placement_cursor) were causing incorrect behavior and have been removed.

---

## Code Quality

### Constraints Met
✅ No `unwrap()` in src (used `.unwrap_or(0)` as safe fallback)
✅ No `expect()` in src
✅ Minimal surgical changes only
✅ Zero allocations in hot path (iterator chains, no Vec allocations)
✅ Inline-friendly (simple logic, no complex branching)
✅ No unsafe code

### Performance
- Tie-breaking: O(n) where n = candidate count (typically 1-3)
- Spanning placement: O(t×s) where t = track count, s = span (typically 3×2 = 6 iterations)
- Both optimized with early returns and fold operations

---

## Regression Testing
Full masonry test suite: **92/300 tests passing (30.67%)** - baseline maintained, no new failures introduced.
