# Investigation Findings: wgpu_texture Layout Issues

## Date: Oct 14, 2025

## Current Status
- ✅ WPT masonry test PASSES (column-auto-repeat-001.html)
- ⚠️ wgpu_texture example has layout issues
- ✅ Build succeeds without errors

## Visual Issues Observed (from screenshot)
1. **#overlay** appears on LEFT side, should be on RIGHT (`right: 0`)
2. **#underlay** text visible, should be BEHIND everything (`z-index: -10`)
3. Header and basic grid structure appear correct

## Key Findings from Investigation

### Finding 1: Taffy's `into_option()` Behavior
**Source:** `/Volumes/samsung_t9/blitz/tmp/taffy/src/style/dimension.rs:278-283`

```rust
pub fn into_option(self) -> Option<f32> {
    match self.0.tag() {
        CompactLength::LENGTH_TAG => Some(self.0.value()),
        _ => None,  // Returns None for PERCENT_TAG and AUTO_TAG
    }
}
```

**Implication:**
- `into_option()` returns `Some` ONLY for absolute pixel lengths
- Percentages (`33%`, `100%`) return `None`
- Auto values return `None`

This means `#overlay` and `#underlay` with `width: 33%; height: 100%` will have:
- `explicit_width = None`
- `explicit_height = None`

So they will go through content-based sizing, which is WRONG for absolutely positioned elements with percentages.

### Finding 2: Absolutely Positioned Elements  
**CSS for #overlay and #underlay:**
```css
position: absolute;
width: 33%;
height: 100%;
```

**Problem:** These elements have `position: absolute` with percentage dimensions. In CSS:
- Percentages on absolutely positioned elements resolve against the containing block
- They do NOT use intrinsic sizing
- They should NOT be going through `calculate_item_intrinsic_size_for_masonry` at all

### Finding 3: The Real Root Cause
`calculate_item_intrinsic_size_for_masonry` is being called for:
1. Masonry grid items ✅ (correct)
2. Subgrid items for intrinsic sizing ✅ (correct)
3. **Regular grid items in grid coordination** ❓ (needs investigation)

The function name is misleading - it's actually a general-purpose intrinsic sizing function.

## Current Implementation Analysis

### What My Code Does:
```rust
// If both width and height are explicit pixels
if explicit_width.is_some() && explicit_height.is_some() {
    content_size = Size::ZERO  // Skip content calculation
} else {
    content_size = calculate_content_intrinsic_size(...)  // Calculate from content
}

merged_size = Size {
    width: explicit_width.unwrap_or(content_size.width),
    height: explicit_height.unwrap_or(content_size.height),
}
```

### Why It's Correct for Masonry:
- Masonry items without explicit pixel sizes need content-based sizing ✅
- Partial explicit dimensions are merged correctly ✅
- WPT test passes ✅

### Why It Might Be Wrong for #overlay/#underlay:
- These have `position: absolute` with percentages
- `explicit_width = None, explicit_height = None` (percentages don't count as "explicit" in Taffy)
- Code calculates content size for them
- **But they shouldn't use content sizing at all** - they need containing block resolution

## Hypothesis: The Bug Isn't in intrinsic_sizing.rs

The layout issues with #overlay and #underlay might be caused by:

1. **Grid layout treating absolutely positioned children as grid items**
   - They should be taken out of flow
   - They should use containing block for percentage resolution
   - They shouldn't participate in grid sizing

2. **Z-index stacking context issues**
   - `z-index: -10` not working on #underlay
   - Might be rendering order problem

3. **Positioning issues unrelated to sizing**
   - `right: 0` not being applied to #overlay
   - Might be inset calculation problem

## Next Steps (Planned)

### Step A: Verify If Absolutely Positioned Elements Should Use This Path
- Check if Taffy's grid layout filters out absolutely positioned children
- Understand when `calculate_item_intrinsic_size_for_masonry` gets called
- Add targeted logging ONLY for absolutely positioned elements

### Step B: Test With Simpler HTML
- Create minimal test case with just absolute positioning
- Remove masonry/grid complexity
- Isolate the actual problem

### Step C: Check Commit History
- Review what OTHER files were changed in the masonry PR
- The bug might be in grid layout logic, not intrinsic sizing
- Check if any positioning/inset code was modified

## Questions That Need Answers

1. **Should absolutely positioned elements go through intrinsic sizing at all?**
   - In CSS spec: NO
   - In Taffy implementation: UNCLEAR

2. **Is the grid layout code treating absolute children correctly?**
   - Should they be filtered out before sizing?
   - Should they use a different code path?

3. **What OTHER code changed in the masonry PR that might affect regular grid?**
   - Grid coordination?
   - Track sizing?
   - Placement logic?

## Conclusion

My `intrinsic_sizing.rs` changes are **probably correct** for their intended purpose (masonry and partial explicit dimensions). The layout bugs in wgpu_texture might be:

1. **Unrelated** to intrinsic sizing (positioning/z-index issues)
2. **Caused by other masonry changes** that affect grid layout
3. **Caused by absolute children being processed incorrectly**

Need to investigate broader context before making more changes to intrinsic_sizing.rs.
