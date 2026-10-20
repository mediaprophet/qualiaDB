//! Zero-Heap Change-Detection Tick (`tick.rs`).
//!
//! Models wrapping 32-bit simulation and frame ticks (`Tick(pub u32)`) for SOA archetype tables,
//! supporting deterministic `is_newer_than(last_run, this_run)` comparisons without heap allocation.

use bytemuck::{Pod, Zeroable};

/// Monotonically advancing 32-bit change-detection tick.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Pod, Zeroable)]
pub struct Tick(pub u32);

impl Tick {
    /// Zero tick constant.
    pub const ZERO: Self = Self(0);

    /// Creates a new `Tick`.
    #[inline]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Raw `u32` tick value.
    #[inline]
    pub const fn get(self) -> u32 {
        self.0
    }

    /// Advances the tick by 1 using wrapping arithmetic.
    #[inline]
    pub fn advance(&mut self) -> Self {
        self.0 = self.0.wrapping_add(1);
        *self
    }

    /// Returns `true` if `self` occurred strictly after `last_run` relative to `this_run`.
    ///
    /// Uses wrapping modular subtraction (`this_run - self <= this_run - last_run` and `self != last_run`),
    /// matching Bevy's `Tick::is_newer_than` semantics.
    #[inline]
    pub const fn is_newer_than(self, last_run: Tick, this_run: Tick) -> bool {
        if self.0 == last_run.0 {
            return false;
        }
        let ticks_since_insert = this_run.0.wrapping_sub(self.0);
        let ticks_since_system = this_run.0.wrapping_sub(last_run.0);
        ticks_since_system > ticks_since_insert
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_is_newer_than_handles_monotonic_and_wrapping_cases() {
        let t1 = Tick::new(10);
        let t2 = Tick::new(15);
        let t3 = Tick::new(20);

        assert!(t2.is_newer_than(t1, t3));
        assert!(t3.is_newer_than(t1, t3));
        assert!(!t1.is_newer_than(t1, t3));
        assert!(!t1.is_newer_than(t2, t3));

        // Wrapping across u32::MAX
        let last = Tick::new(u32::MAX - 2);
        let changed = Tick::new(1);
        let now = Tick::new(5);
        assert!(changed.is_newer_than(last, now));
        assert!(!last.is_newer_than(changed, now));
    }
}
