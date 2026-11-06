//! Spacetime dimensions (x, y, z, t) for semantic topology and temporal ledger (`RM-18` re-export from `etalon-tensor`).

pub use etalon_tensor::spacetime::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scale_step_meters_tiers() {
        let expected_steps = [
            (0, 1e-9),  // sub-nanometer / nanometer
            (1, 1e-6),  // micrometer
            (2, 1e-3),  // millimeter
            (3, 1.0),   // meter
            (4, 1e3),   // kilometer
            (5, 1e6),   // megameter
            (6, 1e9),   // gigameter / astronomical
            (7, 1e12),  // parsec tier
        ];

        for (tier, expected) in expected_steps {
            let step = HierarchicalSpacetimeCoord::scale_step_meters(tier);
            assert!(
                (step - expected).abs() / expected < 1e-12,
                "Tier {tier} expected {expected}, got {step}"
            );
        }
    }

    #[test]
    fn test_from_and_to_meters_round_trip() {
        for s in 0..=7 {
            let step = HierarchicalSpacetimeCoord::scale_step_meters(s);
            let test_xyz = [10.5 * step, -5.25 * step, 0.125 * step];
            let coord = HierarchicalSpacetimeCoord::from_meters_f64(100, s, test_xyz, 0.0);
            assert_eq!(coord.scale_tier, s);
            let recovered = coord.to_meters_f64();
            for i in 0..3 {
                assert!(
                    (recovered[i] - test_xyz[i]).abs() / step < 1e-4,
                    "Failed at tier {s} axis {i}"
                );
            }
        }
    }

    #[test]
    fn test_camera_relative_offset() {
        let cam = HierarchicalSpacetimeCoord::from_meters_f64(0, 3, [100.0, 200.0, 300.0], 0.0);
        let obj = HierarchicalSpacetimeCoord::from_meters_f64(0, 3, [110.0, 200.0, 295.0], 0.0);
        let delta = obj.camera_relative_offset(&cam, 1000.0);
        assert!((delta[0] - 10.0).abs() < 1e-4);
        assert!(delta[1].abs() < 1e-4);
        assert!((delta[2] - (-5.0)).abs() < 1e-4);

        // Cross-sector offset
        let s_cam = pack_sector_xyz(0, 0, 0);
        let s_obj = pack_sector_xyz(1, 0, 0);
        let cam_sec = HierarchicalSpacetimeCoord::from_meters_f64(s_cam, 3, [0.0, 0.0, 0.0], 0.0);
        let obj_sec = HierarchicalSpacetimeCoord::from_meters_f64(s_obj, 3, [0.0, 0.0, 0.0], 0.0);
        let delta_sec = obj_sec.camera_relative_offset(&cam_sec, 5000.0);
        assert!((delta_sec[0] - 5000.0).abs() < 1e-3);
    }
}
