// SPDX-License-Identifier: MIT OR Apache-2.0
//! Re-exports geospatial polyline/polygon tessellation, CDLOD quadtree, and terrain skirt stitcher
//! from `etalon_render::geo_viewport::tessellation`.

pub use etalon_render::geo_viewport::tessellation::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_geo_render_vertex_size() {
        assert_eq!(core::mem::size_of::<GeoRenderVertex>(), 32);
    }

    #[test]
    fn test_capped_miter_polyline_clamping_sharp_turn() {
        let p0 = [0.0, 0.0, 0.0];
        let p1 = [10.0, 0.0, 0.0];
        let rad = 5.0f32.to_radians();
        let p2 = [10.0 - 10.0 * rad.cos(), 10.0 * rad.sin(), 0.0];
        let (half_width, miter_limit) = (1.0f32, 2.0f32);
        let style = StrokeJoinCapStyle::MiterCapped { miter_limit };
        let (verts, indices) =
            tessellate_polyline_stroke_capped(&[p0, p1, p2], half_width, style, 100_000).unwrap();

        assert!(!verts.is_empty() && !indices.is_empty());
        let max_allowed = miter_limit * half_width + 0.01;
        for v in &verts {
            let (dx, dy) = (v.pos[0] - p1[0], v.pos[1] - p1[1]);
            let dist = (dx * dx + dy * dy).sqrt();
            if dist < 8.0 {
                assert!(
                    dist <= max_allowed,
                    "vertex at {:?} dist {} > max {}",
                    v.pos,
                    dist,
                    max_allowed
                );
            }
        }
    }

    #[test]
    fn test_concave_l_shape_polygon_earcut() {
        let exterior = [
            [0.0, 0.0],
            [2.0, 0.0],
            [2.0, 1.0],
            [1.0, 1.0],
            [1.0, 2.0],
            [0.0, 2.0],
        ];
        let (verts, indices) = tessellate_polygon_earcut(&exterior, 10.0, 100_000).unwrap();
        assert_eq!(verts.len(), 6);
        assert_eq!(indices.len(), 12);

        let mut total_area = 0.0f32;
        for i in (0..indices.len()).step_by(3) {
            let (p0, p1, p2) = (
                verts[indices[i] as usize].pos,
                verts[indices[i + 1] as usize].pos,
                verts[indices[i + 2] as usize].pos,
            );
            let a = (p1[0] - p0[0]) * (p2[1] - p0[1]) - (p1[1] - p0[1]) * (p2[0] - p0[0]);
            total_area += a.abs() * 0.5;
        }
        assert!((total_area - 3.0).abs() < 1e-4);
    }

    #[test]
    fn test_cdlod_quadtree_subdivision_and_morph() {
        let (root_min, root_size, max_lod, camera) = ([0.0, 0.0], 1000.0, 3, [10.0, 10.0, 5.0]);
        let nodes = select_cdlod_quadtree_nodes(
            root_min, root_size, max_lod, camera, 1080.0, 1.0, 15.0, 100_000,
        )
        .unwrap();
        assert!(!nodes.is_empty());
        assert!(nodes.iter().any(|n| n.lod_level == max_lod));
        assert!(nodes.iter().any(|n| n.lod_level < max_lod));
        for n in &nodes {
            assert!(n.morph_factor >= 0.0 && n.morph_factor <= 1.0);
        }
    }

    #[test]
    fn test_terrain_patch_with_skirts_and_normals() {
        let dem = [0.0, 1.0, 2.0, 1.0, 2.0, 3.0, 2.0, 3.0, 4.0];
        let (verts, indices) =
            build_terrain_patch_with_skirts(&dem, 3, 3, 10.0, 5.0, 100_000).unwrap();
        assert_eq!(verts.len(), 21);
        assert!(verts.iter().any(|v| (v.pos[2] - (-5.0)).abs() < 1e-4));
        assert!(verts.iter().any(|v| v.normal == [0.0, -1.0, 0.0]));
        assert!(verts.iter().any(|v| v.normal == [0.0, 1.0, 0.0]));
        assert!(verts.iter().any(|v| v.normal == [-1.0, 0.0, 0.0]));
        assert!(verts.iter().any(|v| v.normal == [1.0, 0.0, 0.0]));
        assert!(!indices.is_empty());
    }

    #[test]
    fn test_byte_budget_exceeded_fail_closed() {
        let pts = [[0.0, 0.0, 0.0], [10.0, 0.0, 0.0]];
        let err =
            tessellate_polyline_stroke_capped(&pts, 1.0, StrokeJoinCapStyle::Bevel, 10).unwrap_err();
        assert!(matches!(err, GeoTessError::ByteBudgetExceeded { .. }));

        let ext = [[0.0, 0.0], [1.0, 0.0], [0.5, 1.0]];
        let err = tessellate_polygon_earcut(&ext, 0.0, 10).unwrap_err();
        assert!(matches!(err, GeoTessError::ByteBudgetExceeded { .. }));

        let dem = [0.0; 9];
        let err = build_terrain_patch_with_skirts(&dem, 3, 3, 1.0, 1.0, 10).unwrap_err();
        assert!(matches!(err, GeoTessError::ByteBudgetExceeded { .. }));
    }
}
