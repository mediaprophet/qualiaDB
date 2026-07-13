//! Zero-Heap Utilities
//!
//! This module provides stack-allocated, fixed-size data structures that
//! respect the zero-heap mandate for hot paths in the QualiaDB engine.
//!
//! ## Core Principles
//! - No heap allocation (no Vec, String, Box)
//! - Fixed-size arrays only
//! - Caller-supplied output buffers
//! - Deterministic memory usage
//! - Compatible with 48-byte NQuin structure

/// Maximum size for fixed arrays (configurable per use case)
pub const MAX_FIXED_ARRAY_SIZE: usize = 64;

/// Maximum size for ring buffers (must be power of 2 for efficiency)
pub const MAX_RING_BUFFER_SIZE: usize = 256;

/// Fixed-size array wrapper
///
/// Provides array-like interface with bounds checking.
/// Zero-heap alternative to Vec.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FixedArray<T, const N: usize> {
    data: [T; N],
    len: usize,
}

impl<T: Default + Copy, const N: usize> FixedArray<T, N> {
    /// Creates a new empty fixed array
    pub fn new() -> Self {
        Self {
            data: [T::default(); N],
            len: 0,
        }
    }

    /// Creates a fixed array from an array
    pub fn from_array(data: [T; N]) -> Self {
        Self { data, len: N }
    }

    /// Pushes a value if space is available
    pub fn push(&mut self, value: T) -> Result<(), &'static str> {
        if self.len >= N {
            return Err("FixedArray overflow");
        }
        self.data[self.len] = value;
        self.len += 1;
        Ok(())
    }

    /// Returns the current length
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns true if empty
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns true if full
    pub fn is_full(&self) -> bool {
        self.len >= N
    }

    /// Gets a value by index
    pub fn get(&self, index: usize) -> Option<T> {
        if index < self.len {
            Some(self.data[index])
        } else {
            None
        }
    }

    /// Clears the array
    pub fn clear(&mut self) {
        self.len = 0;
    }

    /// Returns the underlying array slice
    pub fn as_slice(&self) -> &[T] {
        &self.data[..self.len]
    }

    /// Returns the underlying mutable array slice
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        &mut self.data[..self.len]
    }
}

impl<T: Default + Copy, const N: usize> Default for FixedArray<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

/// Fixed-size stack
///
/// LIFO data structure with fixed capacity.
/// Zero-heap alternative to Vec used as a stack.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FixedStack<T, const N: usize> {
    data: [T; N],
    top: usize,
}

impl<T: Default + Copy, const N: usize> FixedStack<T, N> {
    /// Creates a new empty stack
    pub fn new() -> Self {
        Self {
            data: [T::default(); N],
            top: 0,
        }
    }

    /// Pushes a value onto the stack
    pub fn push(&mut self, value: T) -> Result<(), &'static str> {
        if self.top >= N {
            return Err("Stack overflow");
        }
        self.data[self.top] = value;
        self.top += 1;
        Ok(())
    }

    /// Pops a value from the stack
    pub fn pop(&mut self) -> Option<T> {
        if self.top == 0 {
            None
        } else {
            self.top -= 1;
            Some(self.data[self.top])
        }
    }

    /// Peeks at the top value without removing it
    pub fn peek(&self) -> Option<T> {
        if self.top == 0 {
            None
        } else {
            Some(self.data[self.top - 1])
        }
    }

    /// Returns the current depth
    pub fn depth(&self) -> usize {
        self.top
    }

    /// Returns true if empty
    pub fn is_empty(&self) -> bool {
        self.top == 0
    }

    /// Returns true if full
    pub fn is_full(&self) -> bool {
        self.top >= N
    }

    /// Clears the stack
    pub fn clear(&mut self) {
        self.top = 0;
    }
}

impl<T: Default + Copy, const N: usize> Default for FixedStack<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

/// Ring buffer (circular buffer)
///
/// Fixed-size FIFO queue with power-of-2 capacity for efficient indexing.
/// Zero-heap alternative to VecDeque.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct RingBuffer<T, const N: usize> {
    data: [T; N],
    head: usize,
    tail: usize,
    count: usize,
}

impl<T: Default + Copy, const N: usize> RingBuffer<T, N> {
    /// Creates a new empty ring buffer
    pub fn new() -> Self {
        assert!(N.is_power_of_two(), "Ring buffer size must be power of 2");
        Self {
            data: [T::default(); N],
            head: 0,
            tail: 0,
            count: 0,
        }
    }

    /// Enqueues a value
    pub fn enqueue(&mut self, value: T) -> Result<(), &'static str> {
        if self.count >= N {
            return Err("Ring buffer full");
        }
        self.data[self.tail] = value;
        self.tail = (self.tail + 1) & (N - 1);
        self.count += 1;
        Ok(())
    }

    /// Dequeues a value
    pub fn dequeue(&mut self) -> Option<T> {
        if self.count == 0 {
            None
        } else {
            let value = self.data[self.head];
            self.head = (self.head + 1) & (N - 1);
            self.count -= 1;
            Some(value)
        }
    }

    /// Peeks at the front value without removing it
    pub fn peek(&self) -> Option<T> {
        if self.count == 0 {
            None
        } else {
            Some(self.data[self.head])
        }
    }

    /// Returns the current count
    pub fn count(&self) -> usize {
        self.count
    }

    /// Returns true if empty
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Returns true if full
    pub fn is_full(&self) -> bool {
        self.count >= N
    }

    /// Clears the buffer
    pub fn clear(&mut self) {
        self.head = 0;
        self.tail = 0;
        self.count = 0;
    }
}

impl<T: Default + Copy, const N: usize> Default for RingBuffer<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

/// Fixed-size queue (simple FIFO)
///
/// Non-circular fixed-size queue for simpler use cases.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FixedQueue<T, const N: usize> {
    data: [T; N],
    front: usize,
    rear: usize,
    count: usize,
}

impl<T: Default + Copy, const N: usize> FixedQueue<T, N> {
    /// Creates a new empty queue
    pub fn new() -> Self {
        Self {
            data: [T::default(); N],
            front: 0,
            rear: 0,
            count: 0,
        }
    }

    /// Enqueues a value
    pub fn enqueue(&mut self, value: T) -> Result<(), &'static str> {
        if self.count >= N {
            return Err("Queue full");
        }
        self.data[self.rear] = value;
        self.rear = (self.rear + 1) % N;
        self.count += 1;
        Ok(())
    }

    /// Dequeues a value
    pub fn dequeue(&mut self) -> Option<T> {
        if self.count == 0 {
            None
        } else {
            let value = self.data[self.front];
            self.front = (self.front + 1) % N;
            self.count -= 1;
            Some(value)
        }
    }

    /// Peeks at the front value
    pub fn peek(&self) -> Option<T> {
        if self.count == 0 {
            None
        } else {
            Some(self.data[self.front])
        }
    }

    /// Returns the current count
    pub fn count(&self) -> usize {
        self.count
    }

    /// Returns true if empty
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Returns true if full
    pub fn is_full(&self) -> bool {
        self.count >= N
    }

    /// Clears the queue
    pub fn clear(&mut self) {
        self.front = 0;
        self.rear = 0;
        self.count = 0;
    }
}

impl<T: Default + Copy, const N: usize> Default for FixedQueue<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixed_array() {
        let mut arr: FixedArray<u32, 4> = FixedArray::new();
        assert!(arr.is_empty());
        assert!(!arr.is_full());

        arr.push(1).unwrap();
        arr.push(2).unwrap();
        arr.push(3).unwrap();
        arr.push(4).unwrap();

        assert!(arr.is_full());
        assert_eq!(arr.len(), 4);
        assert_eq!(arr.get(0), Some(1));
        assert_eq!(arr.get(3), Some(4));

        assert!(arr.push(5).is_err());
    }

    #[test]
    fn test_fixed_stack() {
        let mut stack: FixedStack<u32, 4> = FixedStack::new();
        assert!(stack.is_empty());

        stack.push(1).unwrap();
        stack.push(2).unwrap();
        stack.push(3).unwrap();

        assert_eq!(stack.depth(), 3);
        assert_eq!(stack.peek(), Some(3));

        assert_eq!(stack.pop(), Some(3));
        assert_eq!(stack.pop(), Some(2));
        assert_eq!(stack.depth(), 1);
    }

    #[test]
    fn test_ring_buffer() {
        let mut buf: RingBuffer<u32, 8> = RingBuffer::new();
        assert!(buf.is_empty());

        buf.enqueue(1).unwrap();
        buf.enqueue(2).unwrap();
        buf.enqueue(3).unwrap();

        assert_eq!(buf.count(), 3);
        assert_eq!(buf.peek(), Some(1));

        assert_eq!(buf.dequeue(), Some(1));
        assert_eq!(buf.dequeue(), Some(2));
        assert_eq!(buf.count(), 1);
    }

    #[test]
    fn test_fixed_queue() {
        let mut queue: FixedQueue<u32, 4> = FixedQueue::new();
        assert!(queue.is_empty());

        queue.enqueue(1).unwrap();
        queue.enqueue(2).unwrap();
        queue.enqueue(3).unwrap();

        assert_eq!(queue.count(), 3);
        assert_eq!(queue.peek(), Some(1));

        assert_eq!(queue.dequeue(), Some(1));
        assert_eq!(queue.dequeue(), Some(2));
        assert_eq!(queue.count(), 1);
    }
}
