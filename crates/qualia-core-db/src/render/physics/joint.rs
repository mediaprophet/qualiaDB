//! Kinematic joints as PGA motors animated over time `t` (Phase 2).
//!
//! "Joints as kinematic multivectors": a joint's pose at time `t` is a pure function returning a
//! `render::pga::Motor` (the even-subalgebra multivector: rotation + translation). Identity at
//! `t = 0`; deterministic (same `t` → same motor). Composing joints is motor multiplication, so a
//! chain (arm → forearm → hand) is `motor_a ⊗ motor_b ⊗ …` with no heap.

use crate::render::pga::{motor_mul, motor_translate, rotor_from_axis_angle, Motor};

/// The kind of single-DOF joint.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum JointKind {
    /// Rotation about a unit `axis` through the origin (`rate` in radians per unit time).
    Revolute { axis: [f32; 3] },
    /// Translation along a unit `axis` (`rate` in distance per unit time).
    Prismatic { axis: [f32; 3] },
}

/// A single-DOF kinematic joint driven by a constant `rate`.
#[derive(Clone, Copy, Debug)]
pub struct Joint {
    pub kind: JointKind,
    pub rate: f32,
}

impl Joint {
    #[inline]
    pub fn revolute(axis: [f32; 3], rate: f32) -> Self {
        Joint {
            kind: JointKind::Revolute { axis },
            rate,
        }
    }

    #[inline]
    pub fn prismatic(axis: [f32; 3], rate: f32) -> Self {
        Joint {
            kind: JointKind::Prismatic { axis },
            rate,
        }
    }

    /// The joint's motor at time `t`. Pure function of `t` (deterministic); identity at `t = 0`.
    #[inline]
    pub fn motor_at(&self, t: f32) -> Motor {
        let q = self.rate * t;
        match self.kind {
            JointKind::Revolute { axis } => Motor::from_rotor(rotor_from_axis_angle(axis, q)),
            JointKind::Prismatic { axis } => {
                motor_translate([axis[0] * q, axis[1] * q, axis[2] * q])
            }
        }
    }
}

/// Compose a chain of joint motors at time `t` (root-first): `m0 ⊗ m1 ⊗ … ⊗ mn`. Zero-alloc.
pub fn chain_motor_at(joints: &[Joint], t: f32) -> Motor {
    let mut m = Motor::identity();
    for j in joints {
        m = motor_mul(m, j.motor_at(t));
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::pga::sandwich_point;

    fn approx(a: [f32; 3], b: [f32; 3]) -> bool {
        (0..3).all(|k| (a[k] - b[k]).abs() < 1e-5)
    }

    #[test]
    fn identity_at_t_zero() {
        let rev = Joint::revolute([0.0, 1.0, 0.0], 1.5);
        let p = [0.7, -0.2, 0.4];
        assert!(approx(sandwich_point(rev.motor_at(0.0), p), p));
    }

    #[test]
    fn revolute_rotates_over_t() {
        // 90° about Y at t·rate = π/2: (1,0,0) → (0,0,-1) in this right-handed convention.
        let rev = Joint::revolute([0.0, 1.0, 0.0], std::f32::consts::FRAC_PI_2);
        let out = sandwich_point(rev.motor_at(1.0), [1.0, 0.0, 0.0]);
        assert!(out[0].abs() < 1e-5, "x≈0, got {out:?}");
        assert!(out[1].abs() < 1e-5, "y≈0");
        assert!(out[2].abs() > 0.9, "|z|≈1");
    }

    #[test]
    fn prismatic_translates_over_t() {
        let pri = Joint::prismatic([1.0, 0.0, 0.0], 2.0);
        let out = sandwich_point(pri.motor_at(0.5), [0.0, 0.0, 0.0]);
        assert!(approx(out, [1.0, 0.0, 0.0])); // 2.0 * 0.5
    }

    #[test]
    fn motor_at_is_deterministic() {
        let j = Joint::revolute([0.0, 0.0, 1.0], 0.9);
        let a = sandwich_point(j.motor_at(0.33), [0.5, 0.1, 0.0]);
        let b = sandwich_point(j.motor_at(0.33), [0.5, 0.1, 0.0]);
        assert_eq!(a, b);
    }

    #[test]
    fn chain_composes_two_joints() {
        // base prismatic +x by 1, then revolute — the chain motor moves the origin to +1 x at least.
        let chain = [
            Joint::prismatic([1.0, 0.0, 0.0], 1.0),
            Joint::revolute([0.0, 1.0, 0.0], 0.5),
        ];
        let out = sandwich_point(chain_motor_at(&chain, 1.0), [0.0, 0.0, 0.0]);
        assert!(out[0] > 0.5);
    }
}
