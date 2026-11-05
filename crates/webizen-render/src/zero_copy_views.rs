//! Zero-copy WASM buffer views and std140-aligned uniform layouts.
//!
//! Re-exports the canonical zero-copy views and std140 layout primitives from
//! `etalon_render::webizen_core::zero_copy_views`.

pub use etalon_render::webizen_core::zero_copy_views::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_uniform_is_128_bytes() {
        assert_eq!(std::mem::size_of::<CameraUniform>(), 128);
    }

    #[test]
    fn model_uniform_is_80_bytes() {
        assert_eq!(std::mem::size_of::<ModelUniform>(), 80);
    }

    #[test]
    fn time_uniform_is_16_bytes() {
        assert_eq!(std::mem::size_of::<TimeUniform>(), 16);
    }

    #[test]
    fn camera_uniform_alignment_is_16() {
        assert_eq!(std::mem::align_of::<CameraUniform>(), 16);
    }

    #[test]
    fn std140_layout_camera_struct() {
        let (layout, total) =
            std140_layout(&[("view", Std140Type::Mat4), ("proj", Std140Type::Mat4)]);
        assert_eq!(layout.len(), 2);
        assert_eq!(layout[0].name, "view");
        assert_eq!(layout[0].offset, 0);
        assert_eq!(layout[0].size, 64);
        assert_eq!(layout[1].name, "proj");
        assert_eq!(layout[1].offset, 64);
        assert_eq!(layout[1].size, 64);
        assert_eq!(total, 128);
    }

    #[test]
    fn std140_layout_vec3_then_scalar() {
        // vec3 at offset 0 (size 12, align 16), then f32 at offset 12
        // (fits in the vec4 padding, no extra alignment needed since 12 is
        // 4-byte aligned for a scalar).
        let (layout, total) = std140_layout(&[("a", Std140Type::Vec3), ("b", Std140Type::Scalar)]);
        assert_eq!(layout[0].offset, 0);
        assert_eq!(layout[0].size, 12);
        assert_eq!(layout[0].align, 16);
        assert_eq!(layout[1].offset, 12);
        assert_eq!(layout[1].size, 4);
        // Total: 16 (rounded up from 16 to 16)
        assert_eq!(total, 16);
    }

    #[test]
    fn std140_layout_array_of_floats() {
        // array<f32, 4>: stride 16 (std140 rounds up to vec4), total 64
        let (layout, total) = std140_layout(&[(
            "data",
            Std140Type::Array {
                elem: Std140TypeSimple::Scalar,
                count: 4,
            },
        )]);
        assert_eq!(layout[0].offset, 0);
        assert_eq!(layout[0].size, 64); // 4 × 16
        assert_eq!(layout[0].align, 16); // stride
        assert_eq!(total, 64);
    }

    #[test]
    fn std140_layout_mixed_struct() {
        // mat4 (64B) + vec3 (12B, aligned 16) + f32 (4B)
        let (layout, total) = std140_layout(&[
            ("model", Std140Type::Mat4),
            ("color", Std140Type::Vec3),
            ("intensity", Std140Type::Scalar),
        ]);
        assert_eq!(layout[0].offset, 0);
        assert_eq!(layout[0].size, 64);
        assert_eq!(layout[1].offset, 64); // 64 is 16-aligned
        assert_eq!(layout[1].size, 12);
        assert_eq!(layout[2].offset, 76); // 64+12=76, 76 is 4-aligned
        assert_eq!(layout[2].size, 4);
        // Total: 80 (76+4=80, already 16-aligned)
        assert_eq!(total, 80);
    }

    #[test]
    fn camera_uniform_bytes_roundtrip() {
        let cam = CameraUniform::new([[1.0; 4]; 4], [[2.0; 4]; 4]);
        let bytes = cam.as_bytes();
        assert_eq!(bytes.len(), 128);
        // First 4 bytes should be 1.0f32 (view matrix [0][0])
        let first_f32 = f32::from_le_bytes(bytes[0..4].try_into().unwrap());
        assert_eq!(first_f32, 1.0);
        // Byte 64 should be 2.0f32 (proj matrix [0][0])
        let proj_f32 = f32::from_le_bytes(bytes[64..68].try_into().unwrap());
        assert_eq!(proj_f32, 2.0);
    }

    #[test]
    fn time_uniform_bytes_roundtrip() {
        let t = TimeUniform::new(1.5, 800.0, 600.0);
        let bytes = t.as_bytes();
        assert_eq!(bytes.len(), 16);
        let time = f32::from_le_bytes(bytes[0..4].try_into().unwrap());
        assert_eq!(time, 1.5);
        let width = f32::from_le_bytes(bytes[4..8].try_into().unwrap());
        assert_eq!(width, 800.0);
    }
}
