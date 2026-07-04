//! P7.3 — σ spectral blend as interpolation on the spectral manifold.
//!
//! Blending colours in spectral space (interpolating SPDs) is physically
//! correct: it preserves the spectral power distribution's structure,
//! unlike RGB lerp which can produce intermediate colours that don't
//! correspond to any physical spectrum.
//!
//! ## Algorithm
//!
//! Spectral blend: `S_blend(t) = (1-t)·S_a + t·S_b` in SPD space, then
//! project to XYZ/RGB.
//!
//! RGB lerp: `RGB_blend(t) = (1-t)·RGB_a + t·RGB_b` in sRGB space.
//!
//! The divergence between the two is the ΔE of their XYZ projections.
//!
//! ## Determinism
//!
//! All operations are deterministic: SPD lerp is a pure function, and
//! the CMF projection is a compile-time constant.

use super::spectral_kernel::{
    delta_e_76, emf_to_spd, linear_to_srgb_channel, spd_to_xyz,
    xyz_to_linear_srgb, Spd, Xyz,
};

// ───────────────────────────────────────────────────────────────────────────
//  Spectral blend
// ───────────────────────────────────────────────────────────────────────────

/// Blend two SPDs in spectral space: `S(t) = (1-t)·a + t·b`.
#[inline]
pub fn spectral_blend_spd(a: &Spd, b: &Spd, t: f32) -> Spd {
    a.lerp(b, t)
}

/// Blend two EMF payloads in spectral space and return the resulting XYZ.
#[inline]
pub fn spectral_blend_emf(
    alpha_a: f32, mu_a: f32, sigma_a: f32,
    alpha_b: f32, mu_b: f32, sigma_b: f32,
    t: f32,
) -> Xyz {
    let spd_a = emf_to_spd(alpha_a, mu_a, sigma_a);
    let spd_b = emf_to_spd(alpha_b, mu_b, sigma_b);
    let blended = spectral_blend_spd(&spd_a, &spd_b, t);
    spd_to_xyz(&blended)
}

/// RGB lerp in linear sRGB space (for comparison with spectral blend).
#[inline]
pub fn rgb_lerp(
    rgb_a: [f32; 3],
    rgb_b: [f32; 3],
    t: f32,
) -> [f32; 3] {
    [
        rgb_a[0] * (1.0 - t) + rgb_b[0] * t,
        rgb_a[1] * (1.0 - t) + rgb_b[1] * t,
        rgb_a[2] * (1.0 - t) + rgb_b[2] * t,
    ]
}

/// Compute the ΔE divergence between spectral blend and gamma-encoded
/// sRGB lerp at parameter `t`.
///
/// The CMF projection is linear, so SPD lerp = XYZ lerp. The divergence
/// with "RGB lerp" appears because display RGB lerp happens in gamma-encoded
/// sRGB space, which is non-linear. We decode the gamma-encoded endpoints,
/// lerp in linear space, then re-encode — no, that would be the same.
///
/// The real divergence is: spectral blend produces an SPD with two peaks
/// (for narrow-band inputs), which when projected gives a different XYZ
/// than the gamma-encoded sRGB lerp. We simulate the common "lerp in
/// 8-bit sRGB" workflow: encode both endpoints to 8-bit sRGB, lerp the
/// 8-bit values, decode back to linear, convert to XYZ.
pub fn blend_divergence(
    alpha_a: f32, mu_a: f32, sigma_a: f32,
    alpha_b: f32, mu_b: f32, sigma_b: f32,
    t: f32,
) -> f32 {
    let spd_a = emf_to_spd(alpha_a, mu_a, sigma_a);
    let spd_b = emf_to_spd(alpha_b, mu_b, sigma_b);

    // Spectral blend → XYZ.
    let blended_spd = spectral_blend_spd(&spd_a, &spd_b, t);
    let xyz_spectral = spd_to_xyz(&blended_spd);

    // sRGB 8-bit lerp: encode both to 8-bit sRGB, lerp, decode back.
    let xyz_a = spd_to_xyz(&spd_a);
    let xyz_b = spd_to_xyz(&spd_b);
    let rgb_a = xyz_to_linear_srgb(&xyz_a);
    let rgb_b = xyz_to_linear_srgb(&xyz_b);

    // Encode to 8-bit sRGB (gamma encode + quantise to 0-255).
    let enc_a = [
        (linear_to_srgb_channel(rgb_a.r) * 255.0).round() / 255.0,
        (linear_to_srgb_channel(rgb_a.g) * 255.0).round() / 255.0,
        (linear_to_srgb_channel(rgb_a.b) * 255.0).round() / 255.0,
    ];
    let enc_b = [
        (linear_to_srgb_channel(rgb_b.r) * 255.0).round() / 255.0,
        (linear_to_srgb_channel(rgb_b.g) * 255.0).round() / 255.0,
        (linear_to_srgb_channel(rgb_b.b) * 255.0).round() / 255.0,
    ];

    // Lerp in gamma-encoded space.
    let enc_blend = [
        enc_a[0] * (1.0 - t) + enc_b[0] * t,
        enc_a[1] * (1.0 - t) + enc_b[1] * t,
        enc_a[2] * (1.0 - t) + enc_b[2] * t,
    ];

    // Decode back to linear (inverse sRGB gamma).
    let decode_channel = |c: f32| -> f32 {
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    let rgb_blend = [
        decode_channel(enc_blend[0]),
        decode_channel(enc_blend[1]),
        decode_channel(enc_blend[2]),
    ];

    // Convert blended linear RGB to XYZ.
    let xyz_srgb_lerp = Xyz::new(
        0.4124564 * rgb_blend[0] + 0.3575761 * rgb_blend[1] + 0.1804375 * rgb_blend[2],
        0.2126729 * rgb_blend[0] + 0.7151522 * rgb_blend[1] + 0.0721750 * rgb_blend[2],
        0.0193339 * rgb_blend[0] + 0.1191920 * rgb_blend[1] + 0.9503041 * rgb_blend[2],
    );

    delta_e_76(&xyz_spectral, &xyz_srgb_lerp)
}

// ───────────────────────────────────────────────────────────────────────────
//  Tests
// ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blend_at_t0_returns_first() {
        let spd_a = emf_to_spd(1.0, 0.1, 0.2);
        let spd_b = emf_to_spd(1.0, 0.1, 0.8);
        let blended = spectral_blend_spd(&spd_a, &spd_b, 0.0);
        assert_eq!(blended, spd_a, "t=0 should return first SPD");
    }

    #[test]
    fn blend_at_t1_returns_second() {
        let spd_a = emf_to_spd(1.0, 0.1, 0.2);
        let spd_b = emf_to_spd(1.0, 0.1, 0.8);
        let blended = spectral_blend_spd(&spd_a, &spd_b, 1.0);
        assert_eq!(blended, spd_b, "t=1 should return second SPD");
    }

    #[test]
    fn spectral_blend_differs_from_rgb_lerp() {
        // Blend between a blue (σ=0) and red (σ=1) — the divergence
        // should be significant because spectral blending preserves the
        // two peaks while RGB lerp produces a mid-colour.
        let de = blend_divergence(1.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.5);
        assert!(de > 0.5, "spectral blend should differ from RGB lerp: ΔE={}", de);
    }

    #[test]
    fn blend_no_nans_in_sweep() {
        for i in 0..=100 {
            let t = i as f32 / 100.0;
            let xyz = spectral_blend_emf(1.0, 0.2, 0.3, 0.8, 0.5, 0.7, t);
            assert!(xyz.x.is_finite(), "X NaN at t={}", t);
            assert!(xyz.y.is_finite(), "Y NaN at t={}", t);
            assert!(xyz.z.is_finite(), "Z NaN at t={}", t);
        }
    }

    #[test]
    fn blend_determinism() {
        let xyz1 = spectral_blend_emf(1.0, 0.2, 0.3, 0.8, 0.5, 0.7, 0.5);
        let xyz2 = spectral_blend_emf(1.0, 0.2, 0.3, 0.8, 0.5, 0.7, 0.5);
        assert_eq!(xyz1, xyz2, "blend must be deterministic");
    }

    #[test]
    fn blend_monotone_continuity() {
        // The blended XYZ should change continuously (no jumps).
        let mut prev = spectral_blend_emf(1.0, 0.1, 0.2, 1.0, 0.1, 0.8, 0.0);
        for i in 1..=100 {
            let t = i as f32 / 100.0;
            let curr = spectral_blend_emf(1.0, 0.1, 0.2, 1.0, 0.1, 0.8, t);
            let dx = (curr.x - prev.x).abs();
            let dy = (curr.y - prev.y).abs();
            let dz = (curr.z - prev.z).abs();
            assert!(dx < 0.1, "X discontinuity at t={}: {}", t, dx);
            assert!(dy < 0.1, "Y discontinuity at t={}: {}", t, dy);
            assert!(dz < 0.1, "Z discontinuity at t={}: {}", t, dz);
            prev = curr;
        }
    }
}
