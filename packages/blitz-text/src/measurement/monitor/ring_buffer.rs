//! Lock-free ring buffer implementation for performance monitoring
//!
//! This module provides a high-performance lock-free ring buffer specifically
//! designed for collecting performance samples with zero allocations.

use std::alloc::{alloc, dealloc, Layout};
use std::mem::MaybeUninit;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Lock-free ring buffer for performance history
#[repr(align(64))]
pub struct LockFreeRingBuffer<T: Copy> {
    /// Preallocated buffer (no heap allocations after init)
    buffer: NonNull<MaybeUninit<T>>,
    /// Buffer capacity (power of 2 for fast modulo)
    capacity: usize,
    /// Write index (only written by producer)
    write_idx: AtomicUsize,
    /// Read index (only written by consumer)
    read_idx: AtomicUsize,
    /// Layout for deallocation
    layout: Layout,
}

impl<T: Copy> LockFreeRingBuffer<T> {
    /// Create new ring buffer with preallocated capacity
    pub fn new(capacity: usize) -> Result<Self, String> {
        // Ensure capacity is power of 2 for fast modulo
        let capacity = capacity.next_power_of_two();
        let layout =
            Layout::array::<MaybeUninit<T>>(capacity).map_err(|_| "Layout error".to_string())?;

        let ptr = unsafe { alloc(layout) };
        if ptr.is_null() {
            return Err("Allocation failed".to_string());
        }

        let buffer = NonNull::new(ptr as *mut MaybeUninit<T>).ok_or("Null pointer".to_string())?;

        Ok(Self {
            buffer,
            capacity,
            write_idx: AtomicUsize::new(0),
            read_idx: AtomicUsize::new(0),
            layout,
        })
    }

    /// Push item (lock-free, single producer)
    #[inline(always)]
    pub fn push(&self, item: T) -> Result<(), T> {
        let write_idx = self.write_idx.load(Ordering::Relaxed);
        let read_idx = self.read_idx.load(Ordering::Acquire);

        // Check if buffer is full
        if write_idx.wrapping_sub(read_idx) >= self.capacity {
            return Err(item);
        }

        unsafe {
            let slot = self.buffer.as_ptr().add(write_idx & (self.capacity - 1));
            slot.write(MaybeUninit::new(item));
        }

        self.write_idx
            .store(write_idx.wrapping_add(1), Ordering::Release);
        Ok(())
    }

    /// Pop item (lock-free, single consumer)
    #[inline(always)]
    pub fn pop(&self) -> Option<T> {
        let read_idx = self.read_idx.load(Ordering::Relaxed);
        let write_idx = self.write_idx.load(Ordering::Acquire);

        if read_idx == write_idx {
            return None;
        }

        let item = unsafe {
            let slot = self.buffer.as_ptr().add(read_idx & (self.capacity - 1));
            slot.read().assume_init()
        };

        self.read_idx
            .store(read_idx.wrapping_add(1), Ordering::Release);
        Some(item)
    }

    /// Get current length (approximate)
    #[inline(always)]
    pub fn len(&self) -> usize {
        let write_idx = self.write_idx.load(Ordering::Relaxed);
        let read_idx = self.read_idx.load(Ordering::Relaxed);
        write_idx.wrapping_sub(read_idx)
    }

    /// Check if buffer is empty
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get buffer capacity
    #[inline(always)]
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

impl<T: Copy> Drop for LockFreeRingBuffer<T> {
    fn drop(&mut self) {
        unsafe {
            dealloc(self.buffer.as_ptr() as *mut u8, self.layout);
        }
    }
}

unsafe impl<T: Copy + Send> Send for LockFreeRingBuffer<T> {}
unsafe impl<T: Copy + Send> Sync for LockFreeRingBuffer<T> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ring_buffer_basic() {
        let buffer = LockFreeRingBuffer::new(4).unwrap();

        // Test push and pop
        assert!(buffer.push(1).is_ok());
        assert!(buffer.push(2).is_ok());

        assert_eq!(buffer.pop(), Some(1));
        assert_eq!(buffer.pop(), Some(2));
        assert_eq!(buffer.pop(), None);
    }

    #[test]
    fn test_ring_buffer_capacity() {
        let buffer = LockFreeRingBuffer::new(2).unwrap();

        // Fill to capacity
        assert!(buffer.push(1).is_ok());
        assert!(buffer.push(2).is_ok());

        // Should be full now
        assert_eq!(buffer.push(3), Err(3));

        // Pop one and try again
        assert_eq!(buffer.pop(), Some(1));
        assert!(buffer.push(3).is_ok());
    }
}
