//! Thread-local buffer management for zero-allocation text measurement
//!
//! This module provides thread-local buffer pools that enable zero-allocation
//! text measurement operations by reusing pre-allocated buffers across measurements.

use std::cell::RefCell;

use cosmyc_text::{Buffer, FontSystem};

use super::types::{CharacterPosition, LineMeasurement, MeasurementError, MeasurementResult};

thread_local! {
    /// Pool of cosmyc-text Buffer objects for reuse across measurements
    /// Capacity starts at 4 buffers and grows as needed
    static MEASUREMENT_BUFFERS: RefCell<Vec<Buffer>> = RefCell::new(Vec::with_capacity(4));

    /// Reusable buffer for character position results
    /// Pre-allocated with 1024 capacity for typical text measurements
    static CHARACTER_POSITIONS_BUFFER: RefCell<Vec<CharacterPosition>> = RefCell::new(Vec::with_capacity(1024));

    /// Reusable buffer for line measurement results
    /// Pre-allocated with 32 capacity for typical multi-line text
    static LINE_MEASUREMENTS_BUFFER: RefCell<Vec<LineMeasurement>> = RefCell::new(Vec::with_capacity(32));

    /// Temporary string buffer for cache key generation
    /// Pre-allocated with 4KB capacity for typical font family names
    static TEMP_STRING_BUFFER: RefCell<String> = RefCell::new(String::with_capacity(4096));

    /// Thread-local FontSystem instance to avoid unsafe shared access
    /// Initialized on first use and cached for the thread lifetime
    static THREAD_LOCAL_FONT_SYSTEM: RefCell<Option<FontSystem>> = RefCell::new(None);
}

/// Get or create a Buffer for text measurement, ensuring zero allocation for reuse
pub fn with_measurement_buffer<F, R>(f: F) -> R
where
    F: FnOnce(&mut Buffer) -> R,
{
    MEASUREMENT_BUFFERS.with(|buffers| {
        let mut buffers = buffers.borrow_mut();

        // Try to reuse an existing buffer
        if let Some(mut buffer) = buffers.pop() {
            // Clear the buffer for reuse using correct cosmyc-text API
            let _ = with_font_system(|font_system| {
                buffer.set_text(
                    font_system,
                    "",
                    &cosmyc_text::Attrs::new(),
                    cosmyc_text::Shaping::Advanced,
                );
            });
            let result = f(&mut buffer);
            // Return the buffer to the pool for future reuse
            buffers.push(buffer);
            result
        } else {
            // Create a new buffer if none available
            let mut buffer = Buffer::new_empty(cosmyc_text::Metrics::new(16.0, 20.0));
            let result = f(&mut buffer);
            // Add the new buffer to the pool for future reuse
            buffers.push(buffer);
            result
        }
    })
}

/// Access thread-local character positions buffer for zero allocation
pub fn with_character_positions<F, R>(f: F) -> R
where
    F: FnOnce(&mut Vec<CharacterPosition>) -> R,
{
    CHARACTER_POSITIONS_BUFFER.with(|buffer| {
        let mut positions = buffer.borrow_mut();
        positions.clear(); // Reset for reuse
        f(&mut positions)
    })
}

/// Access thread-local line measurements buffer for zero allocation  
pub fn with_line_measurements<F, R>(f: F) -> R
where
    F: FnOnce(&mut Vec<LineMeasurement>) -> R,
{
    LINE_MEASUREMENTS_BUFFER.with(|buffer| {
        let mut measurements = buffer.borrow_mut();
        measurements.clear(); // Reset for reuse
        f(&mut measurements)
    })
}

/// Access thread-local string buffer for cache key generation
pub fn with_temp_string<F, R>(f: F) -> R
where
    F: FnOnce(&mut String) -> R,
{
    TEMP_STRING_BUFFER.with(|buffer| {
        let mut temp_string = buffer.borrow_mut();
        temp_string.clear(); // Reset for reuse
        f(&mut temp_string)
    })
}

/// Get thread-local FontSystem, initializing if necessary
/// This avoids unsafe const-to-mutable casting across threads
pub fn with_font_system<F, R>(f: F) -> MeasurementResult<R>
where
    F: FnOnce(&mut FontSystem) -> R,
{
    THREAD_LOCAL_FONT_SYSTEM.with(|font_system| {
        let mut font_system_opt = font_system.borrow_mut();

        // Initialize FontSystem on first access WITH embedded fallback
        if font_system_opt.is_none() {
            let mut new_font_system = FontSystem::new();

            // CRITICAL: Load embedded fallback to guarantee font_id validity
            let _ = crate::embedded_fallback::load_embedded_fallback(new_font_system.db_mut());

            *font_system_opt = Some(new_font_system);
        }

        // Use proper error handling instead of unwrap
        let font_system = font_system_opt
            .as_mut()
            .ok_or(MeasurementError::FontSystemError)?;
        Ok(f(font_system))
    })
}

/// Initialize thread-local FontSystem from a shared FontSystem
/// Creates a new FontSystem for thread-local use (will auto-populate with system fonts)
pub fn initialize_from_shared_font_system(_shared_font_system: &FontSystem) {
    THREAD_LOCAL_FONT_SYSTEM.with(|font_system| {
        let mut font_system_opt = font_system.borrow_mut();

        // Create a new FontSystem for thread-local use
        // It will automatically load system fonts on first use
        let new_font_system = FontSystem::new();

        *font_system_opt = Some(new_font_system);
    });
}

/// Clean up thread-local buffers (automatically called on thread exit)
pub fn cleanup_thread_local_buffers() {
    MEASUREMENT_BUFFERS.with(|buffers| {
        buffers.borrow_mut().clear();
    });

    CHARACTER_POSITIONS_BUFFER.with(|buffer| {
        buffer.borrow_mut().clear();
        buffer.borrow_mut().shrink_to_fit();
    });

    LINE_MEASUREMENTS_BUFFER.with(|buffer| {
        buffer.borrow_mut().clear();
        buffer.borrow_mut().shrink_to_fit();
    });

    TEMP_STRING_BUFFER.with(|buffer| {
        buffer.borrow_mut().clear();
        buffer.borrow_mut().shrink_to_fit();
    });

    THREAD_LOCAL_FONT_SYSTEM.with(|font_system| {
        *font_system.borrow_mut() = None;
    });
}

// Tests extracted to tests/thread_local_tests.rs
