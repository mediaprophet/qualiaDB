//! A bounded, wait-free single-producer / single-consumer (SPSC) ring buffer
//! for scheduled [`TimedEvent`]s.
//!
//! One thread (e.g. the sequencer / loader) pushes events; one thread (the
//! audio thread) pops them. Producer and consumer touch disjoint atomics — the
//! producer owns `tail`, the consumer owns `head` — so neither ever blocks or
//! allocates. One slot of the `N`-element buffer is kept empty to distinguish
//! "full" from "empty", giving a usable capacity of `N - 1`.
//!
//! The events are all-integer `Copy` PODs, so the plain data writes carry no
//! interior pointers; the `Acquire`/`Release` ordering on the indices is what
//! publishes a written slot to the consumer. This mirrors the `rtrb`-style
//! discipline the Qualia hot path uses without pulling in a dependency.

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicUsize, Ordering};

use super::track::TimedEvent;

/// A fixed-capacity SPSC ring of `N` slots (usable capacity `N - 1`).
pub struct EventRing<const N: usize> {
    buf: [UnsafeCell<TimedEvent>; N],
    /// Next slot the consumer will read.
    head: AtomicUsize,
    /// Next slot the producer will write.
    tail: AtomicUsize,
}

// SAFETY: access is disciplined SPSC — the producer only writes the slot at
// `tail` (which the consumer will not read until `tail` is published), and the
// consumer only reads the slot at `head`. `TimedEvent` is `Copy` POD, so there
// are no interior pointers or drop concerns.
unsafe impl<const N: usize> Sync for EventRing<N> {}

impl<const N: usize> Default for EventRing<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> EventRing<N> {
    /// Create an empty ring. `N` must be at least 2 (one slot is a sentinel).
    pub fn new() -> Self {
        assert!(N >= 2, "EventRing needs at least 2 slots");
        Self {
            buf: core::array::from_fn(|_| UnsafeCell::new(TimedEvent::ZERO)),
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
        }
    }

    /// Usable capacity (one slot is reserved as the full/empty sentinel).
    #[inline]
    pub const fn capacity(&self) -> usize {
        N - 1
    }

    /// Producer side: enqueue an event. Returns the event back as `Err` if the
    /// ring is full. Wait-free and allocation-free.
    #[inline]
    pub fn push(&self, event: TimedEvent) -> Result<(), TimedEvent> {
        let tail = self.tail.load(Ordering::Relaxed);
        let next = if tail + 1 == N { 0 } else { tail + 1 };
        if next == self.head.load(Ordering::Acquire) {
            return Err(event); // full
        }
        // SAFETY: SPSC — this slot is not being read by the consumer until we
        // publish `tail` below.
        unsafe {
            *self.buf[tail].get() = event;
        }
        self.tail.store(next, Ordering::Release);
        Ok(())
    }

    /// Consumer side: dequeue the oldest event, or `None` if empty. Wait-free.
    #[inline]
    pub fn pop(&self) -> Option<TimedEvent> {
        let head = self.head.load(Ordering::Relaxed);
        if head == self.tail.load(Ordering::Acquire) {
            return None; // empty
        }
        // SAFETY: SPSC — the producer has published this slot and will not
        // overwrite it until `head` advances past it below.
        let event = unsafe { *self.buf[head].get() };
        let next = if head + 1 == N { 0 } else { head + 1 };
        self.head.store(next, Ordering::Release);
        Some(event)
    }

    /// Whether the ring currently holds no events (consumer-side view).
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.head.load(Ordering::Relaxed) == self.tail.load(Ordering::Acquire)
    }

    /// Number of events currently queued.
    #[inline]
    pub fn len(&self) -> usize {
        let tail = self.tail.load(Ordering::Acquire);
        let head = self.head.load(Ordering::Acquire);
        if tail >= head {
            tail - head
        } else {
            N - head + tail
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fifo_order() {
        let ring: EventRing<8> = EventRing::new();
        for i in 0..5u64 {
            ring.push(TimedEvent::new(i, 0x90, i as u8, 64)).unwrap();
        }
        for i in 0..5u64 {
            assert_eq!(ring.pop().unwrap().tick, i);
        }
        assert!(ring.pop().is_none());
    }

    #[test]
    fn reports_full_at_capacity() {
        let ring: EventRing<4> = EventRing::new(); // capacity 3
        assert_eq!(ring.capacity(), 3);
        ring.push(TimedEvent::ZERO).unwrap();
        ring.push(TimedEvent::ZERO).unwrap();
        ring.push(TimedEvent::ZERO).unwrap();
        assert!(ring.push(TimedEvent::new(99, 0, 0, 0)).is_err());
        assert_eq!(ring.len(), 3);
    }

    #[test]
    fn wraps_around() {
        let ring: EventRing<4> = EventRing::new();
        for round in 0..10u64 {
            ring.push(TimedEvent::new(round, 0x90, 1, 1)).unwrap();
            assert_eq!(ring.pop().unwrap().tick, round);
            assert!(ring.is_empty());
        }
    }
}
