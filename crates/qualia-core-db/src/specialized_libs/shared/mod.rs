//! Shared utilities for specialized libraries
//!
//! This module provides zero-heap utilities that enforce the core architectural
//! boundary: no Vec/String/Box in hot paths. All operations use fixed-size arrays
//! and caller-supplied buffers.

pub mod zero_heap;

pub use zero_heap::{
    FixedArray, FixedQueue, FixedStack, RingBuffer, MAX_FIXED_ARRAY_SIZE, MAX_RING_BUFFER_SIZE,
};
