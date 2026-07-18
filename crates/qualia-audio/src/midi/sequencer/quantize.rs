//! Snap an event tick to a musical grid, with optional strength.
//!
//! `quantize_tick(tick, grid_ticks, strength)` rounds `tick` to the nearest
//! multiple of `grid_ticks` (the grid), then moves the event only a fraction of
//! the way there when `strength < 1.0`:
//!
//! ```text
//! snapped = round(tick / grid) * grid
//! result  = round(tick + strength * (snapped - tick))
//! ```
//!
//! `strength` is clamped to `[0, 1]`: `1.0` snaps fully to the grid, `0.0`
//! leaves the tick untouched, and intermediate values "humanize" by partially
//! pulling toward the grid. Pure and allocation-free.

use crate::types::AudioError;

/// Snap `tick` toward the nearest multiple of `grid_ticks` by `strength`.
///
/// `grid_ticks` is the grid spacing in ticks (e.g. a 1/16 note = `ppq/4`).
/// `strength` in `[0, 1]` is clamped. Returns [`AudioError::InvalidParameter`]
/// if `grid_ticks == 0` or `strength` is not finite.
pub fn quantize_tick(tick: u64, grid_ticks: u32, strength: f64) -> Result<u64, AudioError> {
    if grid_ticks == 0 || !strength.is_finite() {
        return Err(AudioError::InvalidParameter);
    }
    let strength = strength.clamp(0.0, 1.0);
    let grid = grid_ticks as u64;

    // Nearest grid multiple via integer rounding (round half up).
    let snapped = ((tick + grid / 2) / grid) * grid;

    if strength >= 1.0 {
        return Ok(snapped);
    }
    let t = tick as f64;
    let moved = t + strength * (snapped as f64 - t);
    // Round to nearest whole tick, never negative.
    Ok(moved.round().max(0.0) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn golden_119_snaps_to_120_on_sixteenth_grid() {
        // 1/16 note at 480 PPQ = 120 ticks.
        let q = quantize_tick(119, 120, 1.0).unwrap();
        assert_eq!(q, 120);
    }

    #[test]
    fn rounds_down_below_half() {
        // 55 is closer to 0 than to 120 -> 0 at full strength.
        assert_eq!(quantize_tick(55, 120, 1.0).unwrap(), 0);
        // 65 is past the halfway point -> 120.
        assert_eq!(quantize_tick(65, 120, 1.0).unwrap(), 120);
    }

    #[test]
    fn partial_strength_pulls_halfway() {
        // tick 100, grid 120 -> snapped 120; at 0.5 strength -> 110.
        assert_eq!(quantize_tick(100, 120, 0.5).unwrap(), 110);
        // Zero strength leaves it untouched.
        assert_eq!(quantize_tick(100, 120, 0.0).unwrap(), 100);
    }

    #[test]
    fn rejects_zero_grid() {
        assert_eq!(quantize_tick(10, 0, 1.0), Err(AudioError::InvalidParameter));
    }
}
