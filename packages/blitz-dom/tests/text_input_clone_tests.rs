#[cfg(test)]
mod text_input_clone_tests {
    use blitz_dom::node::element::TextInputData;
    use blitz_text::{FontSystem, Cursor, Selection, Edit};
    
    #[test]
    fn test_clone_preserves_text_and_state() {
        let mut font_system = FontSystem::new();
        let mut original = TextInputData::new(&mut font_system, false);
        
        // Set some text content
        original.set_text(&mut font_system, "Hello, World!");
        
        // Set cursor position (using Editor API)
        original.editor.set_cursor(Cursor::new(0, 5)); // Position at comma
        
        // Set selection
        original.editor.set_selection(Selection::Normal(Cursor::new(0, 7))); // Select "Wo"
        
        // Clone the TextInputData
        let cloned = original.clone();
        
        // Verify text content is preserved
        let original_text = original.editor.copy_selection().unwrap_or_else(|| {
            original.editor.with_buffer(|buffer| {
                buffer.lines.iter().map(|line| line.text()).collect::<Vec<_>>().join("\n")
            })
        });
        let cloned_text = cloned.editor.copy_selection().unwrap_or_else(|| {
            cloned.editor.with_buffer(|buffer| {
                buffer.lines.iter().map(|line| line.text()).collect::<Vec<_>>().join("\n")
            })
        });
        
        // Verify state preservation
        assert_eq!(cloned.editor.cursor(), original.editor.cursor());
        assert_eq!(cloned.editor.selection(), original.editor.selection());
        assert_eq!(cloned.is_multiline, original.is_multiline);
        
        // Text should be identical (this tests BufferRef.clone())
        assert_eq!(cloned_text, original_text);
    }
}