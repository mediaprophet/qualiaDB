//! A fixed-capacity track of timed MIDI events plus a zero-alloc "due window"
//! iterator.
//!
//! [`TimedEvent`] is a small `Copy` POD — `(tick, status, data1, data2)` — the
//! generic event currency this lane uses so it never depends on another lane's
//! uncommitted types. A [`Track`] stores up to `N` of them on the stack, kept
//! sorted by tick, and [`Track::due_window`] yields exactly the events whose
//! tick falls in a half-open `[start, end)` window, in order. The iterator
//! borrows the track and allocates nothing, so it is safe to call every audio
//! block on the real-time thread.

use crate::types::AudioError;

/// A single timed MIDI event: a status byte + up to two data bytes, at an
/// absolute tick position. All-integer and `Copy`, so it is torn-read-safe to
/// move through the [`super::event_ring::EventRing`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimedEvent {
    /// Absolute position on the sequencer timeline, in PPQ ticks.
    pub tick: u64,
    /// MIDI status byte (e.g. `0x90` note-on, channel in low nibble).
    pub status: u8,
    /// First data byte (e.g. note number).
    pub data1: u8,
    /// Second data byte (e.g. velocity).
    pub data2: u8,
}

impl TimedEvent {
    /// The all-zero event, usable as a fixed-array filler.
    pub const ZERO: TimedEvent = TimedEvent {
        tick: 0,
        status: 0,
        data1: 0,
        data2: 0,
    };

    /// Construct a timed event.
    #[inline]
    pub const fn new(tick: u64, status: u8, data1: u8, data2: u8) -> Self {
        Self {
            tick,
            status,
            data1,
            data2,
        }
    }
}

/// A fixed-capacity, tick-sorted track of `N` events.
#[derive(Debug, Clone)]
pub struct Track<const N: usize> {
    events: [TimedEvent; N],
    len: usize,
}

impl<const N: usize> Default for Track<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> Track<N> {
    /// An empty track.
    pub const fn new() -> Self {
        Self {
            events: [TimedEvent::ZERO; N],
            len: 0,
        }
    }

    /// Number of events currently stored.
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether the track holds no events.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Insert an event, keeping the track sorted by tick (stable for equal
    /// ticks). This is a cold, loading-time operation — not for the audio
    /// thread. Returns [`AudioError::OutputBufferTooSmall`] if the track is full.
    pub fn insert(&mut self, event: TimedEvent) -> Result<(), AudioError> {
        if self.len >= N {
            return Err(AudioError::OutputBufferTooSmall);
        }
        // Find the insertion point (first index with a strictly greater tick),
        // then shift the tail up by one. Stable: equal ticks keep insert order.
        let mut i = self.len;
        while i > 0 && self.events[i - 1].tick > event.tick {
            self.events[i] = self.events[i - 1];
            i -= 1;
        }
        self.events[i] = event;
        self.len += 1;
        Ok(())
    }

    /// Remove every event from the track.
    pub fn clear(&mut self) {
        self.len = 0;
    }

    /// All stored events, in tick order.
    #[inline]
    pub fn events(&self) -> &[TimedEvent] {
        &self.events[..self.len]
    }

    /// Yield the events due in the half-open window `[start, end)`, in tick
    /// order. Borrows the track; allocates nothing.
    #[inline]
    pub fn due_window(&self, start: u64, end: u64) -> DueWindow<'_> {
        // Binary-search the first index with tick >= start; the tick-sorted
        // slice lets the iterator stop as soon as it passes `end`.
        let slice = &self.events[..self.len];
        let mut lo = 0usize;
        let mut hi = self.len;
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if slice[mid].tick < start {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        DueWindow {
            slice,
            idx: lo,
            end,
        }
    }
}

/// Zero-alloc iterator over the events of a [`Track`] due in `[start, end)`.
pub struct DueWindow<'a> {
    slice: &'a [TimedEvent],
    idx: usize,
    end: u64,
}

impl<'a> Iterator for DueWindow<'a> {
    type Item = &'a TimedEvent;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.idx >= self.slice.len() {
            return None;
        }
        let ev = &self.slice[self.idx];
        if ev.tick >= self.end {
            return None;
        }
        self.idx += 1;
        Some(ev)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_keeps_sorted() {
        let mut t: Track<8> = Track::new();
        t.insert(TimedEvent::new(300, 0x90, 60, 100)).unwrap();
        t.insert(TimedEvent::new(100, 0x90, 62, 100)).unwrap();
        t.insert(TimedEvent::new(200, 0x90, 64, 100)).unwrap();
        let ticks: [u64; 3] = [t.events()[0].tick, t.events()[1].tick, t.events()[2].tick];
        assert_eq!(ticks, [100, 200, 300]);
    }

    #[test]
    fn due_window_in_order_none_outside() {
        let mut t: Track<16> = Track::new();
        for &tick in &[0u64, 50, 100, 150, 200, 250, 300] {
            t.insert(TimedEvent::new(tick, 0x90, 60, 64)).unwrap();
        }
        // Window [100, 250): expect ticks 100, 150, 200 in order (250 excluded).
        let got: heapless_vec::V =
            t.due_window(100, 250)
                .fold(heapless_vec::V::new(), |mut v, e| {
                    v.push(e.tick);
                    v
                });
        assert_eq!(got.as_slice(), &[100, 150, 200]);
    }

    #[test]
    fn full_track_rejects_insert() {
        let mut t: Track<2> = Track::new();
        t.insert(TimedEvent::ZERO).unwrap();
        t.insert(TimedEvent::ZERO).unwrap();
        assert_eq!(
            t.insert(TimedEvent::ZERO),
            Err(AudioError::OutputBufferTooSmall)
        );
    }

    /// Tiny fixed-capacity collector so the test asserts order without `Vec`.
    mod heapless_vec {
        pub struct V {
            buf: [u64; 16],
            len: usize,
        }
        impl V {
            pub fn new() -> Self {
                Self {
                    buf: [0; 16],
                    len: 0,
                }
            }
            pub fn push(&mut self, v: u64) {
                self.buf[self.len] = v;
                self.len += 1;
            }
            pub fn as_slice(&self) -> &[u64] {
                &self.buf[..self.len]
            }
        }
    }
}
