//! Warp scale velocity curve (OCS §12.2).
//!
//! TOS: v = w³·c
//! TNG: v = w^(10/3)·c for 1 ≤ w ≤ 9
//! TNG soft saturation: w^(10/3)·c·[1 + α·tan(π(w-9)/(2(10-9+ε)))] for 9 < w < 10
//!
//! Reference: OCS Specification v2.2.0 §12.2.

const C: f64 = 299_792_458.0; // Speed of light (m/s)

/// Warp scale variant (OCS §12.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WarpScale {
    /// TOS: v = w³·c
    Tos,
    /// TNG: v = w^(10/3)·c with soft saturation near warp 10
    Tng,
}

/// Compute velocity (m/s) for a given warp factor (OCS §12.2).
///
/// - TOS scale: v = w³ · c
/// - TNG scale (w ≤ 9): v = w^(10/3) · c
/// - TNG scale (9 < w < 10): soft saturation with tangent barrier
/// - w ≥ 10: returns infinity (warp 10 is asymptotic)
pub fn warp_velocity(w: f64, scale: WarpScale) -> f64 {
    if w < 0.0 {
        return 0.0;
    }
    match scale {
        WarpScale::Tos => w * w * w * C,
        WarpScale::Tng => {
            if w >= 10.0 {
                return f64::INFINITY;
            }
            if w <= 9.0 {
                return f64::powf(w, 10.0 / 3.0) * C;
            }
            // Soft saturation: 9 < w < 10
            // v = w^(10/3) · c · [1 + α · tan(π(w-9) / (2(10-9+ε)))]
            let epsilon = 1e-6;
            let alpha = 0.1; // Saturation coefficient
            let tan_arg = std::f64::consts::PI * (w - 9.0) / (2.0 * (1.0 + epsilon));
            let base = f64::powf(w, 10.0 / 3.0) * C;
            let saturation = 1.0 + alpha * tan_arg.tan();
            base * saturation
        }
    }
}

/// Warp factor as multiple of c (dimensionless).
pub fn warp_factor_c(w: f64, scale: WarpScale) -> f64 {
    warp_velocity(w, scale) / C
}

/// Cochrane unit — field distortion metric (OCS §12.3).
/// C = 1.0 at threshold c (warp 1).
pub fn cochrane_units(w: f64, scale: WarpScale) -> f64 {
    warp_factor_c(w, scale)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tos_warp1_is_c() {
        let v = warp_velocity(1.0, WarpScale::Tos);
        assert!((v - C).abs() < 1.0, "warp 1 should be c");
    }

    #[test]
    fn tos_warp2_is_8c() {
        let v = warp_velocity(2.0, WarpScale::Tos);
        assert!((v - 8.0 * C).abs() < 1.0, "warp 2 should be 8c");
    }

    #[test]
    fn tng_warp1_is_c() {
        let v = warp_velocity(1.0, WarpScale::Tng);
        assert!((v - C).abs() < 1.0, "warp 1 should be c");
    }

    #[test]
    fn tng_warp9() {
        // v = 9^(10/3) * c ≈ 1516.38 * c
        let v = warp_velocity(9.0, WarpScale::Tng);
        let expected = f64::powf(9.0, 10.0 / 3.0) * C;
        assert!((v - expected).abs() < 1.0);
    }

    #[test]
    fn tng_warp10_is_infinite() {
        let v = warp_velocity(10.0, WarpScale::Tng);
        assert!(v.is_infinite(), "warp 10 should be infinite");
    }

    #[test]
    fn tng_warp_9_99_finite() {
        // OCS-T13: Warp 9.99 evaluates without NaN
        let v = warp_velocity(9.99, WarpScale::Tng);
        assert!(v.is_finite(), "warp 9.99 should be finite");
        assert!(v > 0.0);
    }

    #[test]
    fn tng_warp_9_99_no_nan() {
        // OCS-T13: no numerical overflow/NaN
        let v = warp_velocity(9.999, WarpScale::Tng);
        assert!(!v.is_nan(), "warp 9.999 must not be NaN");
        assert!(v.is_finite(), "warp 9.999 should be finite");
    }

    #[test]
    fn warp0_is_zero() {
        assert_eq!(warp_velocity(0.0, WarpScale::Tos), 0.0);
        assert_eq!(warp_velocity(0.0, WarpScale::Tng), 0.0);
    }

    #[test]
    fn negative_warp_is_zero() {
        assert_eq!(warp_velocity(-1.0, WarpScale::Tos), 0.0);
        assert_eq!(warp_velocity(-1.0, WarpScale::Tng), 0.0);
    }

    #[test]
    fn cochrane_at_warp1() {
        let c = cochrane_units(1.0, WarpScale::Tng);
        assert!((c - 1.0).abs() < 1e-10, "C should be 1.0 at warp 1");
    }

    #[test]
    fn tng_soft_saturation_increases() {
        // Velocity at 9.5 should be greater than at 9.0
        let v9 = warp_velocity(9.0, WarpScale::Tng);
        let v95 = warp_velocity(9.5, WarpScale::Tng);
        assert!(v95 > v9, "warp 9.5 should be faster than warp 9");
    }
}
