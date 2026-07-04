//! P7.2 — Gamut / object-colour solid as a convex polytope + closest-point
//! gamut mapping.
//!
//! The sRGB gamut is the convex hull of the primary colours (red, green,
//! blue) and their combinations in XYZ space. An out-of-gamut colour is
//! mapped to the closest point on the gamut boundary.
//!
//! ## Algorithm
//!
//! 1. The sRGB gamut is the set of all `(R,G,B)` with `0 ≤ R,G,B ≤ 1`,
//!    mapped through the sRGB→XYZ matrix.
//! 2. In-gamut check: convert XYZ to linear sRGB; if all channels are in
//!    `[0,1]`, the colour is in-gamut.
//! 3. Out-of-gamut mapping: clamp each sRGB channel to `[0,1]` and convert
//!    back to XYZ. This is a simple (non-optimal) closest-point mapping.
//!
//! ## Determinism
//!
//! All operations are deterministic: the XYZ→sRGB matrix is a constant,
//! and clamping is a pure function.

use super::spectral_kernel::{xyz_to_linear_srgb, LinearRgb, Xyz};

// ───────────────────────────────────────────────────────────────────────────
//  Errors
// ───────────────────────────────────────────────────────────────────────────

/// Gamut mapping error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GamutError {
    /// Non-finite input.
    NonFinite,
}

impl core::fmt::Display for GamutError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NonFinite => write!(f, "gamut: non-finite input"),
        }
    }
}

impl std::error::Error for GamutError {}

// ───────────────────────────────────────────────────────────────────────────
//  Gamut operations
// ───────────────────────────────────────────────────────────────────────────

/// Check if a colour is in the sRGB gamut (all linear sRGB channels in [0,1]).
#[inline]
pub fn is_in_gamut(xyz: &Xyz) -> bool {
    let rgb = xyz_to_linear_srgb(xyz);
    rgb.r >= 0.0 && rgb.r <= 1.0
        && rgb.g >= 0.0 && rgb.g <= 1.0
        && rgb.b >= 0.0 && rgb.b <= 1.0
}

/// Map an out-of-gamut colour to the closest in-gamut colour.
///
/// This uses the simple clamping approach: convert to linear sRGB, clamp
/// each channel to [0,1], and convert back to XYZ.
#[inline]
pub fn gamut_map_clamp(xyz: &Xyz) -> Xyz {
    let rgb = xyz_to_linear_srgb(xyz);
    let clamped = LinearRgb::new(
        rgb.r.clamp(0.0, 1.0),
        rgb.g.clamp(0.0, 1.0),
        rgb.b.clamp(0.0, 1.0),
    );
    linear_srgb_to_xyz(&clamped)
}

/// Convert linear sRGB to CIE XYZ (inverse of `xyz_to_linear_srgb`).
#[inline]
pub fn linear_srgb_to_xyz(rgb: &LinearRgb) -> Xyz {
    let x = 0.4124564 * rgb.r + 0.3575761 * rgb.g + 0.1804375 * rgb.b;
    let y = 0.2126729 * rgb.r + 0.7151522 * rgb.g + 0.0721750 * rgb.b;
    let z = 0.0193339 * rgb.r + 0.1191920 * rgb.g + 0.9503041 * rgb.b;
    Xyz::new(x, y, z)
}

/// Check if a linear sRGB colour is in gamut.
#[inline]
pub fn linear_rgb_is_in_gamut(rgb: &LinearRgb) -> bool {
    rgb.r >= 0.0 && rgb.r <= 1.0
        && rgb.g >= 0.0 && rgb.g <= 1.0
        && rgb.b >= 0.0 && rgb.b <= 1.0
}

/// Interior idempotence: an in-gamut colour maps to itself.
#[inline]
pub fn gamut_map_idempotent(xyz: &Xyz) -> bool {
    if !is_in_gamut(xyz) {
        return false;
    }
    let mapped = gamut_map_clamp(xyz);
    let diff = ((mapped.x - xyz.x).abs() + (mapped.y - xyz.y).abs() + (mapped.z - xyz.z).abs()) / 3.0;
    diff < 1e-6
}

// ───────────────────────────────────────────────────────────────────────────
//  Tests
// ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_gamut_colour_is_in_gamut() {
        // Use exact D65 white point values.
        let white = Xyz::new(0.95047, 1.0, 1.08883);
        assert!(is_in_gamut(&white), "D65 white should be in gamut");
    }

    #[test]
    fn out_of_gamut_detected() {
        // A very saturated "super-red" outside the sRGB triangle.
        let super_red = Xyz::new(2.0, 0.5, 0.0);
        assert!(!is_in_gamut(&super_red), "super-red should be out of gamut");
    }

    #[test]
    fn gamut_map_brings_out_of_gamut_inside() {
        let super_red = Xyz::new(2.0, 0.5, 0.0);
        let mapped = gamut_map_clamp(&super_red);
        assert!(is_in_gamut(&mapped), "mapped colour should be in gamut");
    }

    #[test]
    fn interior_idempotence() {
        let in_gamut = Xyz::new(0.4, 0.5, 0.6);
        if is_in_gamut(&in_gamut) {
            assert!(gamut_map_idempotent(&in_gamut),
                "in-gamut colour should map to itself");
        }
    }

    #[test]
    fn gamut_map_determinism() {
        let out = Xyz::new(1.5, 0.3, 0.1);
        let m1 = gamut_map_clamp(&out);
        let m2 = gamut_map_clamp(&out);
        assert_eq!(m1, m2, "gamut mapping must be deterministic");
    }

    #[test]
    fn linear_srgb_round_trip() {
        let rgb = LinearRgb::new(0.5, 0.3, 0.8);
        let xyz = linear_srgb_to_xyz(&rgb);
        let rgb2 = xyz_to_linear_srgb(&xyz);
        assert!((rgb.r - rgb2.r).abs() < 1e-4, "R round-trip");
        assert!((rgb.g - rgb2.g).abs() < 1e-4, "G round-trip");
        assert!((rgb.b - rgb2.b).abs() < 1e-4, "B round-trip");
    }

    #[test]
    fn primary_red_is_in_gamut() {
        let red = LinearRgb::new(1.0, 0.0, 0.0);
        let xyz = linear_srgb_to_xyz(&red);
        assert!(is_in_gamut(&xyz), "primary red should be in gamut");
    }

    #[test]
    fn primary_green_is_in_gamut() {
        let green = LinearRgb::new(0.0, 1.0, 0.0);
        let xyz = linear_srgb_to_xyz(&green);
        assert!(is_in_gamut(&xyz), "primary green should be in gamut");
    }

    #[test]
    fn primary_blue_is_in_gamut() {
        let blue = LinearRgb::new(0.0, 0.0, 1.0);
        let xyz = linear_srgb_to_xyz(&blue);
        assert!(is_in_gamut(&xyz), "primary blue should be in gamut");
    }

    #[test]
    fn black_is_in_gamut() {
        let black = Xyz::new(0.0, 0.0, 0.0);
        assert!(is_in_gamut(&black), "black should be in gamut");
    }
}
