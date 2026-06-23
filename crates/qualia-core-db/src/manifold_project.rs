//! Unified manifold projection (Phase 1.4, `RENDERER_IMPLEMENTATION_PLAN.md`) — **one projection,
//! many views**.
//!
//! The renderer's foundation is the 10D tensor manifold. A node's *place* is decided once, by the
//! semantic-motor map `10D → 3D world` (the same map `projector.wgsl` applies on the GPU;
//! [`crate::portal_pga`] is its parity-tested CPU oracle). Every "view" — the 3D scene, the 2D
//! canvas — is then a projection of that **same** world point onto a target, not an independent
//! re-computation. This module is the single entry point that makes that explicit:
//!
//!   * [`manifold_world`] — the shared step: `Tensor10D → [x,y,z]` world.
//!   * [`project`] — that world point as the requested [`ProjectionTarget`] (3D volume, or its 2D
//!     planar shadow). One call, selectable view.
//!
//! The 3D *scene* additionally applies the orbit camera ([`crate::portal_camera`]) on top of the
//! world point; the 2D *canvas* uses the planar shadow directly. Both start from one `project`.

use crate::portal_pga::{sandwich_point, semantic_motor_intrinsic};
use crate::portal_telemetry::STANDPOINT_SPECTATOR;
use crate::tensor::Tensor10D;

/// Which view of the shared manifold world point to produce.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectionTarget {
    /// 2D canvas: the orthographic shadow of the world point on the `z = 0` plane.
    Plane2D,
    /// 3D scene: the world point itself (the orbit camera is applied downstream).
    Volume3D,
}

/// The shared projection step — a 10D tensor node to its 3D world position via the semantic-motor
/// manifold map. `time` drives the animated bands (`v`/`q`); a spectator standpoint with full
/// epistemic aperture is used (view-neutral). This is the parity-tested oracle of `projector.wgsl`.
#[inline]
pub fn manifold_world(t: &Tensor10D, time: f32) -> [f32; 3] {
    let local = [t.x, t.y, t.z];
    let motor = semantic_motor_intrinsic(
        t.v,
        t.w,
        t.q,
        t.sigma,
        time,
        t.alpha,
        local,
        STANDPOINT_SPECTATOR,
        1.0,
    );
    sandwich_point(motor, local)
}

/// One projection, many views: project a 10D node through the shared manifold map, then select the
/// view. `Volume3D` yields the 3D world point; `Plane2D` yields its 2D shadow (`z` zeroed). The two
/// agree on `(x, y)` by construction — they are the *same* manifold point seen two ways.
#[inline]
pub fn project(t: &Tensor10D, time: f32, target: ProjectionTarget) -> [f32; 3] {
    let world = manifold_world(t, time);
    match target {
        ProjectionTarget::Volume3D => world,
        ProjectionTarget::Plane2D => [world[0], world[1], 0.0],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node() -> Tensor10D {
        Tensor10D {
            q: 0.4,
            v: 1.5,
            w: 2.0,
            x: 0.3,
            y: -0.2,
            z: 0.5,
            t: 0.5,
            alpha: 0.9,
            mu: 0.0,
            sigma: 0.25,
        }
    }

    #[test]
    fn one_projection_many_views() {
        let t = node();
        let time = 0.7;
        let v3 = project(&t, time, ProjectionTarget::Volume3D);
        let v2 = project(&t, time, ProjectionTarget::Plane2D);
        // Both views are the SAME manifold world point: the 2D plane view is the (x,y) shadow of
        // the 3D volume view, and the 3D view is exactly the shared step.
        assert!((v2[0] - v3[0]).abs() < 1e-6);
        assert!((v2[1] - v3[1]).abs() < 1e-6);
        assert_eq!(v2[2], 0.0);
        assert_eq!(v3, manifold_world(&t, time));
    }

    #[test]
    fn euclidean_node_projects_to_itself() {
        // v=0 (Euclidean), w=0, q=0 → identity motor → world == local, independent of time.
        let t = Tensor10D {
            q: 0.0,
            v: 0.0,
            w: 0.0,
            x: 0.2,
            y: -0.4,
            z: 0.1,
            sigma: 0.0,
            alpha: 1.0,
            ..node()
        };
        let w = manifold_world(&t, 1.23);
        assert!((w[0] - 0.2).abs() < 1e-6);
        assert!((w[1] + 0.4).abs() < 1e-6);
        assert!((w[2] - 0.1).abs() < 1e-6);
    }

    #[test]
    fn projection_is_deterministic() {
        let t = node();
        assert_eq!(
            project(&t, 2.5, ProjectionTarget::Volume3D),
            project(&t, 2.5, ProjectionTarget::Volume3D)
        );
    }
}
