//! Transient AO-only packed-depth contract AODEPTH_LOG1P_BA16 v1.
//! Main camera/deferred depth formats remain owned by their existing contracts.
use crate::render::camera::CAMERA_FAR_PLANE;

/// Stored in the previously reserved camera_forward.w AO-uniform lane. Both
/// producers and consumers must use this kind; old transient targets are rebuilt.
pub(super) const AODEPTH_LOG1P_BA16: u32 = 1;
const INTERVALS: f32 = 65_534.0;

/// log2(1 + world-space view depth) per valid BA16 interval. Code zero remains
/// background; codes 1..=65535 cover zero through the native camera far plane.
pub(super) fn log2_depth_step() -> f32 {
    (1.0 + CAMERA_FAR_PLANE).log2() / INTERVALS
}

/// CPU readback oracle for the actual internal GPU packer, not a public codec.
#[cfg(test)]
pub(in crate::render::gpu) fn encode_depth(depth: f32) -> Option<u16> {
    if !depth.is_finite() || !(0.0..=CAMERA_FAR_PLANE).contains(&depth) {
        return None;
    }
    Some(
        1 + ((1.0 + depth).log2() / log2_depth_step())
            .clamp(0.0, INTERVALS)
            .round() as u16,
    )
}

#[cfg(test)]
pub(in crate::render::gpu) fn decode_depth(code: u16) -> Option<f32> {
    if code == 0 {
        return None;
    }
    Some(((code - 1) as f32 * log2_depth_step()).exp2() - 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::camera::CAMERA_NEAR_PLANE;

    #[test]
    fn ao_log_depth_keeps_background_and_native_camera_endpoints_distinct() {
        assert_eq!(decode_depth(0), None);
        assert_eq!(encode_depth(0.0), Some(1));
        assert_eq!(encode_depth(CAMERA_FAR_PLANE), Some(u16::MAX));
        assert!((decode_depth(u16::MAX).unwrap() - CAMERA_FAR_PLANE).abs() < 0.02);
        for bad in [-1.0, f32::NAN, f32::INFINITY, CAMERA_FAR_PLANE + 1.0] {
            assert_eq!(encode_depth(bad), None);
        }
    }

    #[test]
    fn ao_log_depth_resolves_near_object_detail_below_one_millimetre() {
        for depth in [CAMERA_NEAR_PLANE, 0.1, 0.5, 1.0, 3.5, 4.0, 8.0, 10.0] {
            let decoded = decode_depth(encode_depth(depth).unwrap()).unwrap();
            assert!(
                (decoded - depth).abs() < 0.001,
                "depth={depth} decoded={decoded}"
            );
        }
        // A 5 cm feature at four metres spans many codes rather than disappearing
        // into the previous 22.9 cm linear-depth quantum.
        assert!(encode_depth(4.05).unwrap() - encode_depth(4.0).unwrap() > 32);
    }

    #[test]
    fn ao_log_depth_roundtrip_has_bounded_relative_error_through_fifteen_kilometres() {
        let mut previous = 0;
        for index in 0..=4096 {
            let depth = (1.0 + CAMERA_FAR_PLANE).powf(index as f32 / 4096.0) - 1.0;
            let depth = depth.min(CAMERA_FAR_PLANE);
            let code = encode_depth(depth).unwrap();
            assert!(code > previous);
            previous = code;
            let decoded = decode_depth(code).unwrap();
            assert!(
                (decoded - depth).abs() / (1.0 + depth) < 0.000075,
                "depth={depth} code={code} decoded={decoded}"
            );
        }
    }

    #[test]
    fn ao_log_depth_preserves_subradius_bias_separation_at_reference_distance() {
        let center = 4.0;
        for separation in [0.005, 0.01, 0.025, 0.05] {
            let delta = decode_depth(encode_depth(center + separation).unwrap()).unwrap()
                - decode_depth(encode_depth(center).unwrap()).unwrap();
            assert!(delta > 0.0 && (delta - separation).abs() < 0.001);
        }
        let rear = encode_depth(3.875).unwrap();
        let front = encode_depth(3.15).unwrap();
        assert!(rear > front + 100, "unchanged AO mask separation contract");
    }
}
