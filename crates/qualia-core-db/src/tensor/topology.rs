//! Topological classes (v) for geometric physics rules (`RM-18` re-export from `etalon-tensor`).

pub use etalon_tensor::topology::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_seven_topology_classes_round_trip() {
        let variants = [
            TopologyClass::Euclidean,
            TopologyClass::Cyclic,
            TopologyClass::Hyperbolic,
            TopologyClass::BoundaryCliques,
            TopologyClass::SphericalHealpix,
            TopologyClass::LorentzianSpacetime,
            TopologyClass::MultiplyConnectedPortal,
        ];

        for (expected_idx, variant) in variants.iter().enumerate() {
            assert_eq!(*variant as u32, expected_idx as u32);
            assert_eq!(variant.to_v_value(), expected_idx as f32);
            assert_eq!(
                TopologyClass::from_v_value(expected_idx as f32),
                *variant
            );
        }
    }

    #[test]
    fn test_spherical_healpix_distance() {
        let topo = TopologyClass::SphericalHealpix;
        // Two points on unit sphere separated by 90 degrees on equator: (1, 0, 0) and (0, 1, 0)
        let p1 = (1.0, 0.0, 0.0);
        let p2 = (0.0, 1.0, 0.0);
        let d = topo.calculate_distance(p1, p2);
        let expected = std::f32::consts::FRAC_PI_2; // pi / 2
        assert!((d - expected).abs() < 1e-4);

        // Near origin fallback
        let p_origin = (0.0, 0.0, 0.0);
        let p_near = (0.0, 0.0, 0.5);
        let d_origin = topo.calculate_distance(p_origin, p_near);
        assert!((d_origin - 0.5).abs() < 1e-4);
    }

    #[test]
    fn test_lorentzian_spacetime_interval() {
        let topo = TopologyClass::LorentzianSpacetime;
        // Spacelike separation: dx = 3, dt = 0 -> ds = +3
        let p1 = (0.0, 0.0, 0.0, 0.0);
        let p2 = (3.0, 0.0, 0.0, 0.0);
        assert_eq!(topo.calculate_spacetime_interval(p1, p2), 3.0);

        // Timelike separation: dx = 0, dt = 4 -> ds = -4
        let p3 = (0.0, 0.0, 0.0, 4.0);
        assert_eq!(topo.calculate_spacetime_interval(p1, p3), -4.0);

        // Lightlike: dx = 3, dy = 4 (spatial dist 5), dt = 5 -> ds = 0
        let p4 = (3.0, 4.0, 0.0, 5.0);
        assert_eq!(topo.calculate_spacetime_interval(p1, p4), 0.0);
    }

    #[test]
    fn test_multiply_connected_portal_wrap() {
        let topo = TopologyClass::MultiplyConnectedPortal;
        let p1 = (0.05, 0.1, 0.2);
        let p2 = (0.95, 0.1, 0.2);
        let d = topo.calculate_distance(p1, p2);
        // Wrapped dx across unit boundary = 0.1
        assert!((d - 0.1).abs() < 1e-5);
    }
}
