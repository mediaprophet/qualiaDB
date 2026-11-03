//! Compatibility exports for the canonical Etalon numerical implementation.
pub use qualia_inference_kernel::qwen::numerics::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rms_norm_is_scale_aware() {
        let mut out = [0.0f32; 2];
        rms_norm_into(&[3.0, 4.0], &[1.0, 2.0], 1e-6, &mut out).unwrap();
        assert!((out[0] - 0.8485).abs() < 0.001);
        assert!((out[1] - 2.2627).abs() < 0.001);
    }
}
