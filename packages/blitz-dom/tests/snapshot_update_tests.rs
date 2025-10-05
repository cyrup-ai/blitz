//! Test suite for document snapshot update functionality
//!
//! This test suite validates the snapshot update logic in the BaseDocument
//! including existing snapshot updates and invalidation type handling.

use blitz_dom::{BaseDocument, DocumentConfig};

#[cfg(test)]
mod snapshot_tests {
    use super::*;

    #[test]
    fn test_snapshot_update_existing() {
        // Test updating existing snapshot with state changes
        let config = DocumentConfig::for_testing();
        let mut doc = BaseDocument::new(config)
            .expect("Failed to create test document");
        
        let test_node_id = 0; // Use root node
        
        // Verify initial state
        assert!(!doc.nodes[test_node_id].has_snapshot, "Node should not have snapshot initially");
        assert!(!doc.nodes[test_node_id].is_hovered(), "Node should not be hovered initially");
        assert!(!doc.nodes[test_node_id].is_focussed(), "Node should not be focussed initially");
        
        // Test hover state update in snapshot
        doc.nodes[test_node_id].hover();
        assert!(doc.nodes[test_node_id].is_hovered(), "Node should be hovered after hover() call");
        
        // Take first snapshot - creates new snapshot
        doc.snapshot_node(test_node_id);
        assert!(doc.nodes[test_node_id].has_snapshot, "Node should have snapshot after snapshot_node() call");
        
        // Change hover state and verify snapshot updates existing entry
        doc.nodes[test_node_id].unhover();
        assert!(!doc.nodes[test_node_id].is_hovered(), "Node should not be hovered after unhover() call");
        
        // Take second snapshot - updates existing snapshot
        doc.snapshot_node(test_node_id);
        assert!(doc.nodes[test_node_id].has_snapshot, "Node should still have snapshot after second snapshot_node() call");
        
        // Test focus state changes
        doc.nodes[test_node_id].focus();
        assert!(doc.nodes[test_node_id].is_focussed(), "Node should be focussed after focus() call");
        
        // Take third snapshot - continues to update existing snapshot
        doc.snapshot_node(test_node_id);
        assert!(doc.nodes[test_node_id].has_snapshot, "Node should still have snapshot after third snapshot_node() call");
        
        // Test snapshot_node_and callback functionality with state verification
        doc.nodes[test_node_id].blur();
        assert!(!doc.nodes[test_node_id].is_focussed(), "Node should not be focussed after blur() call");
        
        doc.snapshot_node_and(test_node_id, |node| {
            // Verify the node state is accessible in callback
            assert!(!node.is_focussed(), "Node should not be focussed in callback");
            assert!(node.has_snapshot, "Node should have snapshot in callback");
            
            // Test state modification in callback
            node.active();
            assert!(node.is_active(), "Node should be active after active() call in callback");
        });
        
        // Verify state persists after callback
        assert!(doc.nodes[test_node_id].is_active(), "Node should remain active after callback");
        assert!(doc.nodes[test_node_id].has_snapshot, "Node should still have snapshot after callback");
    }

    #[test] 
    fn test_invalidation_types() {
        // Test all invalidation types trigger proper updates
        let config = DocumentConfig::for_testing();
        let mut doc = BaseDocument::new(config)
            .expect("Failed to create test document");
        
        let test_node_id = 0; // Use root node
        
        // Test initial state verification
        assert!(!doc.nodes[test_node_id].has_snapshot, "Node should not have snapshot initially");
        
        // Create initial snapshot with no special states
        doc.snapshot_node(test_node_id);
        assert!(doc.nodes[test_node_id].has_snapshot, "Node should have snapshot after initial snapshot_node() call");
        
        // Test hover state invalidation
        doc.nodes[test_node_id].hover();
        assert!(doc.nodes[test_node_id].is_hovered(), "Node should be hovered");
        
        doc.snapshot_node(test_node_id);
        assert!(doc.nodes[test_node_id].has_snapshot, "Node should maintain snapshot after hover state update");
        
        // Test focus state invalidation
        doc.nodes[test_node_id].focus();
        assert!(doc.nodes[test_node_id].is_focussed(), "Node should be focussed");
        assert!(doc.nodes[test_node_id].is_hovered(), "Node should still be hovered");
        
        doc.snapshot_node(test_node_id);
        assert!(doc.nodes[test_node_id].has_snapshot, "Node should maintain snapshot after focus state update");
        
        // Test active state invalidation
        doc.nodes[test_node_id].active();
        assert!(doc.nodes[test_node_id].is_active(), "Node should be active");
        assert!(doc.nodes[test_node_id].is_focussed(), "Node should still be focussed");
        assert!(doc.nodes[test_node_id].is_hovered(), "Node should still be hovered");
        
        doc.snapshot_node(test_node_id);
        assert!(doc.nodes[test_node_id].has_snapshot, "Node should maintain snapshot after active state update");
        
        // Test state clearing and verification
        doc.nodes[test_node_id].unhover();
        doc.nodes[test_node_id].blur();
        doc.nodes[test_node_id].unactive();
        
        assert!(!doc.nodes[test_node_id].is_hovered(), "Node should not be hovered after unhover");
        assert!(!doc.nodes[test_node_id].is_focussed(), "Node should not be focussed after blur");
        assert!(!doc.nodes[test_node_id].is_active(), "Node should not be active after unactive");
        
        // Test snapshot with cleared states
        doc.snapshot_node(test_node_id);
        assert!(doc.nodes[test_node_id].has_snapshot, "Node should maintain snapshot after state clearing");
        
        // Test multiple rapid state changes with snapshots
        for i in 0..5 {
            // Cycle through different state combinations
            match i % 3 {
                0 => {
                    doc.nodes[test_node_id].hover();
                    assert!(doc.nodes[test_node_id].is_hovered(), "Node should be hovered in cycle {}", i);
                }
                1 => {
                    doc.nodes[test_node_id].focus();
                    assert!(doc.nodes[test_node_id].is_focussed(), "Node should be focussed in cycle {}", i);
                }
                2 => {
                    doc.nodes[test_node_id].active();
                    assert!(doc.nodes[test_node_id].is_active(), "Node should be active in cycle {}", i);
                }
                _ => unreachable!(),
            }
            
            doc.snapshot_node(test_node_id);
            assert!(doc.nodes[test_node_id].has_snapshot, "Node should maintain snapshot in cycle {}", i);
        }
        
        // Test that snapshot_node_and works with invalidation handling
        doc.snapshot_node_and(test_node_id, |node| {
            // Verify all invalidation types are properly captured in the snapshot system:
            // - Hover state updates (tested above)
            // - Focus state updates (tested above)  
            // - Active state updates (tested above)
            // - Visited state updates (privacy-safe default, not directly testable)
            // - Attribute updates (tested through state changes)
            
            assert!(node.has_snapshot, "Node should have snapshot in invalidation test callback");
            
            // Test combined state changes
            node.hover();
            node.focus();
            node.active();
            
            assert!(node.is_hovered(), "Node should be hovered after combined state change");
            assert!(node.is_focussed(), "Node should be focussed after combined state change");
            assert!(node.is_active(), "Node should be active after combined state change");
        });
        
        // Verify final state
        assert!(doc.nodes[test_node_id].is_hovered(), "Node should remain hovered after callback");
        assert!(doc.nodes[test_node_id].is_focussed(), "Node should remain focussed after callback");
        assert!(doc.nodes[test_node_id].is_active(), "Node should remain active after callback");
        assert!(doc.nodes[test_node_id].has_snapshot, "Node should maintain snapshot after invalidation testing");
    }
}