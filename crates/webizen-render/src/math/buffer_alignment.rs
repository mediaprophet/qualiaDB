//! Buffer alignment utilities for GPU transfer.
//!
//! Re-exports the canonical 64-byte aligned GPU buffer types from
//! `etalon_render::webizen_core::buffer_alignment`.

pub use etalon_render::webizen_core::buffer_alignment::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aligned_buffer_size() {
        let buffer = AlignedBufferF32::new([0.0; 16]);
        assert_eq!(std::mem::size_of_val(&buffer), 64);
    }

    #[test]
    fn test_render_quin_size() {
        assert_eq!(std::mem::size_of::<RenderQuin>(), 64);
    }

    #[test]
    fn test_render_quin_bytes() {
        let quin = RenderQuin::new(
            [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0],
            123,
            0.5,
            0.9,
            1000.0,
        );
        let bytes = bytemuck::bytes_of(&quin);
        assert_eq!(bytes.len(), 64);
    }

    #[test]
    fn test_render_quin_default() {
        let quin = RenderQuin::default();
        assert_eq!(quin.motor[0], 1.0); // Identity motor scalar
        assert_eq!(quin.semantic_id, 0);
    }
}
