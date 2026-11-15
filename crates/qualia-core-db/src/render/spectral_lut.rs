//! Deterministic Zero-Heap Spectral Inversion LUT & Levenberg-Marquardt Solver in Pure Rust.
pub use etalon_render::spectral::lut::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mid_gray_inversion_roundtrip() {
        let target = [0.5, 0.5, 0.5];
        let coeffs = invert_srgb_to_coeffs(target, [0.0, 0.0, 0.0]);
        let (recovered, _) = project_coeffs_to_srgb(coeffs);

        assert!((recovered[0] - 0.5).abs() < 0.02);
        assert!(
            (recovered[1] - 0.5).abs() < 0.02,
            "recovered={recovered:?}, coeffs={coeffs:?}"
        );
        assert!((recovered[2] - 0.5).abs() < 0.02);
    }
}
