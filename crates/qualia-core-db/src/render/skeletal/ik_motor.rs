//! Analytical Two-Bone, Constrained FABRIK, Coupled Swing-Twist Reach Cones,
//! Scapulo-Humeral Rhythm & Biarticular Tendon Coupling (re-exported from `etalon-render`).
pub use etalon_render::skeletal::ik_motor::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn swing_twist_and_couplings_reexported_zero_heap() {
        use crate::specialized_libs::computational_geometry::allocation_counter::assert_zero_alloc;

        let cone = SwingTwistConeConstraint::elliptical([1.0, 0.0, 0.0], 0.6, 0.4, -0.5, 0.5);
        let coupling = ScapuloHumeralCoupling::default();
        let biarticular = BiarticularLengthConstraint::new(
            &[
                BiarticularTerm {
                    joint_index: 0,
                    moment_arm_m: 0.03,
                    axis: [1.0, 0.0, 0.0],
                },
                BiarticularTerm {
                    joint_index: 1,
                    moment_arm_m: 0.04,
                    axis: [1.0, 0.0, 0.0],
                },
            ],
            0.04,
        )
        .unwrap();

        assert_zero_alloc("qualia_db_swing_twist_reexport", || {
            let (s, t) = decompose_swing_twist([1.0, 0.0, 0.0, 0.0], [1.0, 0.0, 0.0]);
            assert!((s[0] - 1.0).abs() < 1e-6 && (t[0] - 1.0).abs() < 1e-6);

            let proj = project_swing_twist_cone(
                [0.70710677, 0.0, 0.70710677, 0.0],
                [1.0, 0.0, 0.0],
                0.6,
                0.4,
                -0.5,
                0.5,
                1.0,
            );
            assert!(cone.is_quaternion_within(proj, 1.0, 1e-4));

            let (gh, st) = coupling.partition_elevation_rad(std::f32::consts::FRAC_PI_2);
            assert!((gh + st - std::f32::consts::FRAC_PI_2).abs() < 1e-5);

            let mut angles = [0.9_f32, 0.8_f32];
            let viol = biarticular
                .project_angles_in_place(&mut angles, None, 1.0)
                .unwrap();
            assert!(viol > 0.0);
            assert!(biarticular.is_satisfied(&angles, 1.0).unwrap());
        });
    }
}
