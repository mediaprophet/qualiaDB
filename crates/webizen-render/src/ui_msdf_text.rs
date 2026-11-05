//! GPU Multi-Channel Signed Distance Field (MSDF) Font Atlas & Zero-Heap UTF-8 Text Shaper (`ui_msdf_text.rs`)
//!
//! Re-exports the canonical zero-heap MSDF font atlas and text shaper from `etalon_render::ui_canvas::msdf_text`.

pub use etalon_render::ui_canvas::msdf_text::*;

#[cfg(test)]
mod tests {
    use super::*;
    use bytemuck::Zeroable;

    #[test]
    fn msdf_shaper_handles_utf8_10d_symbols_kerning_and_word_wrap() {
        let mut quads = [MsdfTextQuad::zeroed(); 128];
        let text = "AV α=0.95 μ=1 σ=2.4\n10D Holosonic HUD";
        let metrics = shape_text_zero_heap(
            text,
            10.0,
            20.0,
            16.0,
            1.25,
            120.0,
            [1.0, 1.0, 1.0, 1.0],
            &mut quads,
        );

        assert!(metrics.quad_count > 10);
        assert!(
            metrics.line_count >= 3,
            "Expected word-wrap + newline to produce >= 3 lines"
        );
        assert!(metrics.width_px <= 120.0 + 1e-3);
        assert!(MSDF_TEXT_WGSL.contains("max(min(r, g), min(max(r, g), b))"));
    }

    #[test]
    fn procedural_msdf_atlas_generates_valid_distance_fields() {
        let mut atlas = vec![0u8; MSDF_ATLAS_WIDTH * MSDF_ATLAS_HEIGHT * 4];
        let written = generate_procedural_msdf_atlas_into(&mut atlas);
        assert_eq!(written, MSDF_ATLAS_WIDTH * MSDF_ATLAS_HEIGHT * 4);

        // Verify 'A' (slot 33) has both inside (>128) and outside (<128) MSDF samples
        let slot_a = glyph_slot_for_char('A');
        let col = slot_a % MSDF_ATLAS_COLS;
        let row = slot_a / MSDF_ATLAS_COLS;
        let mut has_inside = false;
        let mut has_outside = false;
        for py in 0..MSDF_CELL_PX {
            for px in 0..MSDF_CELL_PX {
                let idx =
                    ((row * MSDF_CELL_PX + py) * MSDF_ATLAS_WIDTH + (col * MSDF_CELL_PX + px)) * 4;
                let r = atlas[idx] as f32 / 255.0;
                let g = atlas[idx + 1] as f32 / 255.0;
                let b = atlas[idx + 2] as f32 / 255.0;
                let med = msdf_median_of_three(r, g, b);
                if med > 0.55 {
                    has_inside = true;
                }
                if med < 0.45 {
                    has_outside = true;
                }
            }
        }
        assert!(
            has_inside && has_outside,
            "Glyph 'A' should contain both inside and outside SDF regions"
        );
    }
}
