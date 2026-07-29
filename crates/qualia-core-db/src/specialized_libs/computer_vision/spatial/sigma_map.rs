//! D2 — class / confidence → spectral σ for Tensor10D node hints.
//!
//! Documented mapping for portal paint/scrub. Not ground truth; epistemic proposals
//! only. Values are in `[0, 1]` for spectral index slots.

use crate::specialized_libs::computer_vision::types::Detection;

/// Map a class hash into a base σ band.
///
/// Reserved bands (programme D2 table):
/// - `0` unknown / empty → 0.05
/// - low ordinal classes (1..=8) when used as ids → spaced 0.15..0.85
/// - full FNV hashes → fold into mid band so distinct classes separate in paint
pub fn class_hash_to_sigma_base(class_hash: u64) -> f32 {
    if class_hash == 0 {
        return 0.05;
    }
    // Prefer stable bands for small integer class ids still in use.
    if class_hash <= 8 {
        return match class_hash as u32 {
            1 => 0.20,
            2 => 0.30,
            3 => 0.40,
            4 => 0.50,
            5 => 0.60,
            6 => 0.70,
            7 => 0.80,
            8 => 0.85,
            _ => 0.05,
        };
    }
    let h = (class_hash.wrapping_mul(0x9E37_79B9_7F4A_7C15)) % 1000;
    0.15 + (h as f32 / 1000.0) * 0.70
}

/// Combine class base σ with detection confidence (score).
///
/// `sigma = base * (0.35 + 0.65 * score)` so low-confidence proposals stay dim.
pub fn class_score_to_sigma(class_hash: u64, score: f32) -> f32 {
    let s = score.clamp(0.0, 1.0);
    let base = class_hash_to_sigma_base(class_hash);
    (base * (0.35 + 0.65 * s)).clamp(0.0, 1.0)
}

/// σ from a [`Detection`] (uses `class_hash` + `score_f32`).
pub fn detection_to_sigma(d: &Detection) -> f32 {
    class_score_to_sigma(d.class_hash, d.score_f32())
}

// Back-compat alias used in public docs / D2 table naming.
pub use class_hash_to_sigma_base as class_id_to_sigma_base;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specialized_libs::computer_vision::types::Detection;

    #[test]
    fn unknown_class_low() {
        assert!((class_hash_to_sigma_base(0) - 0.05).abs() < 1e-5);
    }

    #[test]
    fn score_scales_down() {
        let full = class_score_to_sigma(4, 1.0);
        let half = class_score_to_sigma(4, 0.0);
        assert!(full > half);
        assert!(half > 0.0);
    }

    #[test]
    fn detection_path() {
        let mut d = Detection::empty();
        d.class_hash = 2;
        d.score_u16 = 65535;
        let s = detection_to_sigma(&d);
        assert!((s - class_score_to_sigma(2, 1.0)).abs() < 1e-5);
    }

    #[test]
    fn distinct_hashes_differ() {
        let a = class_hash_to_sigma_base(0xABCD_1234_5678_9EF0);
        let b = class_hash_to_sigma_base(0x1111_2222_3333_4444);
        assert!((a - b).abs() > 1e-4);
    }
}
