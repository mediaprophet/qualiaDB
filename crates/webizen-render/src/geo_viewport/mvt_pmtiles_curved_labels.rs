// SPDX-License-Identifier: MIT OR Apache-2.0
//! Re-exports MVT/PMTiles vector tile style evaluator, curve-following centerline glyph placement,
//! and R-Tree screen-space label collision solver from `etalon_render::geo_viewport::curved_labels`.

pub use etalon_render::geo_viewport::curved_labels::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_curved_glyph_quad_size() {
        assert_eq!(core::mem::size_of::<CurvedGlyphQuad>(), 32);
        assert_eq!(core::mem::align_of::<CurvedGlyphQuad>(), 4);
    }

    #[test]
    fn test_zoom_stop_interpolation() {
        let stops = [
            StyleZoomStop {
                zoom: 10.0,
                value: 2.0,
            },
            StyleZoomStop {
                zoom: 12.0,
                value: 8.0,
            },
            StyleZoomStop {
                zoom: 14.0,
                value: 20.0,
            },
        ];
        assert_eq!(evaluate_style_zoom_interpolated(&stops, 8.0, 1.0), 2.0);
        assert_eq!(evaluate_style_zoom_interpolated(&stops, 15.0, 1.0), 20.0);

        let linear_val = evaluate_style_zoom_interpolated(&stops, 11.0, 1.0);
        assert!((linear_val - 5.0).abs() < 1e-4);

        let exp_val = evaluate_style_zoom_interpolated(&stops, 11.0, 2.0);
        assert!((exp_val - 4.0).abs() < 1e-4);
    }

    #[test]
    fn test_curve_following_glyph_placement_and_hairpin_rejection() {
        let gentle_river = [
            [0.0, 0.0],
            [50.0, 5.0],
            [100.0, 15.0],
            [150.0, 30.0],
            [200.0, 50.0],
        ];
        let max_turn = core::f32::consts::PI / 4.0;
        let res_gentle = layout_glyphs_along_centerline(
            &gentle_river,
            "RIVER",
            16.0,
            12.0,
            max_turn,
            0xFFFFFFFF,
            65536,
        );
        assert!(res_gentle.is_ok());
        let quads = res_gentle.unwrap();
        assert_eq!(quads.len(), 5);

        let hairpin_bend = [[0.0, 0.0], [100.0, 0.0], [10.0, 30.0]];
        let res_hairpin = layout_glyphs_along_centerline(
            &hairpin_bend,
            "RIVER",
            20.0,
            12.0,
            max_turn,
            0xFFFFFFFF,
            65536,
        );
        match res_hairpin {
            Err(GeoLabelError::MaxCurvatureExceeded { turn_angle_rad, .. }) => {
                assert!(turn_angle_rad > 2.0);
            }
            other => panic!("expected MaxCurvatureExceeded, got {other:?}"),
        }
    }

    #[test]
    fn test_screen_space_label_collision_and_imhof_offsets() {
        let candidates = vec![
            LabelCandidate {
                id: 1,
                text: "Alpha".to_string(),
                anchor_px: [200.0, 200.0],
                bbox_half_px: [20.0, 10.0],
                priority: 10.0,
                allow_8_way_offset: true,
            },
            LabelCandidate {
                id: 2,
                text: "Beta".to_string(),
                anchor_px: [200.0, 200.0],
                bbox_half_px: [20.0, 10.0],
                priority: 9.0,
                allow_8_way_offset: true,
            },
        ];

        let placed =
            solve_screen_space_label_collisions(&candidates, [800.0, 600.0], 4.0, 65536).unwrap();
        assert_eq!(placed.len(), 2);
        assert_eq!(placed[0].id, 1);
        assert_eq!(placed[0].offset_position_code, OFFSET_TR);
        assert_eq!(placed[1].id, 2);
        assert_eq!(placed[1].offset_position_code, OFFSET_TL);

        let budget_err = solve_screen_space_label_collisions(&candidates, [800.0, 600.0], 4.0, 10);
        match budget_err {
            Err(GeoLabelError::ByteBudgetExceeded { .. }) => {}
            other => panic!("expected ByteBudgetExceeded, got {other:?}"),
        }
    }
}
