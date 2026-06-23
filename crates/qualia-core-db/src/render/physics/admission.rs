//! Deterministic admission of a proposed transform on an artefact (Phase 2).
//!
//! The rail (RENDERER_IMPLEMENTATION_PLAN.md): **deterministic prevention, no probabilistic guess**.
//! Given an artefact's bounding box and a proposed `(motor, scale)` transform, [`Admission::admit`]
//! returns the same verdict for the same inputs, every time — refusing a transform that would
//! *contract* the artefact below its material floor, or move it outside permitted world bounds.
//! "PGA geometry that refuses to contract on a bounding-box violation."

use super::aabb::Aabb;
use crate::render::pga::Motor;

/// Why a transform was refused. Carries the offending measurement so the caller can report it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Refusal {
    /// A scale would contract an axis extent below the material floor.
    Contraction { axis: usize, resulting: f32, floor: f32 },
    /// The transformed artefact would leave the permitted world bounds.
    OutOfBounds,
}

/// An admission policy for an artefact: how far it may be compressed, and where it may exist.
#[derive(Clone, Copy, Debug)]
pub struct Admission {
    /// Minimum permitted per-axis extent (material incompressibility floor). A scale that takes any
    /// axis below this is refused. `0.0` disables the contraction check.
    pub min_extent: f32,
    /// Optional world bound the transformed artefact must stay within. `None` = unbounded.
    pub world: Option<Aabb>,
}

impl Admission {
    #[inline]
    pub fn new(min_extent: f32, world: Option<Aabb>) -> Self {
        Admission { min_extent, world }
    }

    /// Deterministically admit (returning the resulting AABB) or refuse the proposed transform.
    ///
    /// Contraction is judged on the **scale against the artefact's own extent** (not the post-
    /// rotation AABB), so a pure rotation — which enlarges the axis-aligned box but does not
    /// compress the artefact — is never mistaken for contraction. Out-of-bounds is judged on the
    /// actual transformed enclosure.
    pub fn admit(&self, artefact: &Aabb, motor: Motor, scale: [f32; 3]) -> Result<Aabb, Refusal> {
        if self.min_extent > 0.0 {
            let e = artefact.extent();
            for axis in 0..3 {
                let resulting = e[axis] * scale[axis].abs();
                if resulting < self.min_extent {
                    return Err(Refusal::Contraction { axis, resulting, floor: self.min_extent });
                }
            }
        }
        let moved = artefact.transformed(motor, scale);
        if let Some(world) = self.world {
            if !world.contains(&moved) {
                return Err(Refusal::OutOfBounds);
            }
        }
        Ok(moved)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::pga::{motor_translate, Motor};

    fn artefact() -> Aabb {
        Aabb::new([-1.0, -1.0, -1.0], [1.0, 1.0, 1.0]) // extent 2 per axis
    }

    #[test]
    fn admits_a_valid_rigid_move() {
        let policy = Admission::new(0.5, None);
        let out = policy.admit(&artefact(), motor_translate([3.0, 0.0, 0.0]), [1.0, 1.0, 1.0]);
        assert!(out.is_ok());
    }

    #[test]
    fn refuses_contraction_below_floor() {
        let policy = Admission::new(0.5, None);
        // scale 0.1 → extent 0.2 < floor 0.5 → refused, deterministically, on axis 0.
        let verdict = policy.admit(&artefact(), Motor::identity(), [0.1, 1.0, 1.0]);
        assert_eq!(
            verdict,
            Err(Refusal::Contraction { axis: 0, resulting: 0.2, floor: 0.5 })
        );
    }

    #[test]
    fn rotation_is_not_contraction() {
        use crate::render::pga::rotor_from_axis_angle;
        let policy = Admission::new(0.5, None);
        let spin = Motor::from_rotor(rotor_from_axis_angle([0.0, 1.0, 0.0], 0.7));
        // pure rotation, scale 1 — must be admitted even though the AABB grows.
        assert!(policy.admit(&artefact(), spin, [1.0, 1.0, 1.0]).is_ok());
    }

    #[test]
    fn refuses_out_of_world_bounds() {
        let world = Aabb::new([-2.0, -2.0, -2.0], [2.0, 2.0, 2.0]);
        let policy = Admission::new(0.0, Some(world));
        // translate +5 on x pushes the box outside the world → refused.
        let verdict = policy.admit(&artefact(), motor_translate([5.0, 0.0, 0.0]), [1.0, 1.0, 1.0]);
        assert_eq!(verdict, Err(Refusal::OutOfBounds));
        // a small move stays inside → admitted.
        assert!(policy
            .admit(&artefact(), motor_translate([0.5, 0.0, 0.0]), [1.0, 1.0, 1.0])
            .is_ok());
    }

    #[test]
    fn verdict_is_deterministic() {
        let policy = Admission::new(0.5, None);
        let a = policy.admit(&artefact(), Motor::identity(), [0.1, 1.0, 1.0]);
        let b = policy.admit(&artefact(), Motor::identity(), [0.1, 1.0, 1.0]);
        assert_eq!(a, b);
    }
}
