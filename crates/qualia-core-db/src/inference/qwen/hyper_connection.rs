//! Compatibility exports for the canonical Etalon numerical implementation.
pub use qualia_inference_kernel::qwen::hyper_connection::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn group_rms_norm_applies_stored_weight() {
        // The converter folds (1 + w) into the stored gamma: it multiplies as-is.
        let streams = [3.0f32, 4.0];
        let weight = [1.0f32, 2.0];
        let mut out = [0.0f32; 2];
        group_rms_norm_into(&streams, 1, 2, &weight, 1.0e-6, &mut out).unwrap();
        let inv_rms = 1.0 / (25.0f32 / 2.0 + 1.0e-6).sqrt();
        assert!((out[0] - 3.0 * inv_rms * 1.0).abs() < 1.0e-5);
        assert!((out[1] - 4.0 * inv_rms * 2.0).abs() < 1.0e-5);
    }

    #[test]
    fn mix_and_inject_preserve_all_streams() {
        let normalized = [1.0f32, 2.0, 3.0, 4.0];
        let gates = [1.0f32; 4];
        let mut mixed = [0.0f32; 2];
        mix_streams_into(&normalized, &gates, 2, 2, &mut mixed).unwrap();
        assert_eq!(mixed, [2.0, 3.0]);
        let mut streams = normalized;
        inject_stream_update(&mut streams, &[1.0, 2.0], &gates, 2, 2).unwrap();
        assert_eq!(streams, [3.0, 6.0, 5.0, 8.0]);
    }
}
