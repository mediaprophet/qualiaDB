//! Analytical SDF Collider & Tetrahedron Gradient Normal Evaluator (`sdf_collider.rs`).
//!
//! First-class analytical Signed Distance Field collision primitive unique to `QualiaDB`:
//! shares the exact mathematical SDF definitions of `HolographicNodeGpu` and `.10d` `PROC1`
//! (`evaluate_smooth_minimum_sdf`), evaluating signed distance $d = \text{sdf}(\mathbf{x})$ and
//! the surface gradient normal $\nabla\text{sdf}(\mathbf{x})$ in $O(1)$ with zero triangle-mesh
//! discretization error and zero heap allocation.

use qualia_core_db::container_10d::procedural_synthesis::evaluate_smooth_minimum_sdf;
use qualia_core_db::render::pga::Motor;

/// Analytical SDF primitive shapes evaluated in $O(1)$ local coordinates.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SdfPrimitive {
    /// Exact sphere of given radius centered at local origin.
    Sphere { radius: f32 },
    /// Exact rounded box with half-extents `[hx, hy, hz]` and corner fillet ` bevel_radius`.
    RoundedBox {
        half_extents: [f32; 3],
        bevel_radius: f32,
    },
    /// Exact vertical capsule along local Y-axis (`[-half_height, +half_height]`) with `radius`.
    Capsule { radius: f32, half_height: f32 },
    /// Exact torus in the local XZ plane with `major_radius` and `minor_radius`.
    Torus {
        major_radius: f32,
        minor_radius: f32,
    },
    /// `.10d` `PROC1` $C^1$ polynomial smooth-minimum union of two spheres along local X/Y/Z.
    SmoothUnionPair {
        center_a: [f32; 3],
        radius_a: f32,
        center_b: [f32; 3],
        radius_b: f32,
        blend_radius: f32,
    },
}

impl Default for SdfPrimitive {
    #[inline]
    fn default() -> Self {
        Self::Sphere { radius: 0.5 }
    }
}

impl SdfPrimitive {
    /// Evaluates the exact signed distance $d = \text{sdf}(\mathbf{p})$ in local space.
    ///
    /// Negative inside the volume, zero on the boundary, positive outside.
    #[inline]
    pub fn evaluate_local(&self, p: [f32; 3]) -> f32 {
        match *self {
            Self::Sphere { radius } => len3(p) - radius,
            Self::RoundedBox {
                half_extents,
                bevel_radius,
            } => {
                let qx = p[0].abs() - half_extents[0];
                let qy = p[1].abs() - half_extents[1];
                let qz = p[2].abs() - half_extents[2];
                let out_len = len3([qx.max(0.0), qy.max(0.0), qz.max(0.0)]);
                let in_dist = qx.max(qy.max(qz)).min(0.0);
                out_len + in_dist - bevel_radius
            }
            Self::Capsule {
                radius,
                half_height,
            } => {
                let clamped_y = p[1].clamp(-half_height, half_height);
                len3([p[0], p[1] - clamped_y, p[2]]) - radius
            }
            Self::Torus {
                major_radius,
                minor_radius,
            } => {
                let q_xz = (p[0] * p[0] + p[2] * p[2]).sqrt() - major_radius;
                (q_xz * q_xz + p[1] * p[1]).sqrt() - minor_radius
            }
            Self::SmoothUnionPair {
                center_a,
                radius_a,
                center_b,
                radius_b,
                blend_radius,
            } => {
                let d1 = len3(sub3(p, center_a)) - radius_a;
                let d2 = len3(sub3(p, center_b)) - radius_b;
                evaluate_smooth_minimum_sdf(d1, d2, blend_radius)
            }
        }
    }

    /// Evaluates the unit outward gradient normal $\mathbf{n} = \frac{\nabla\text{sdf}(\mathbf{p})}{\|\nabla\text{sdf}(\mathbf{p})\|}$
    /// in local space using the symmetric 4-sample tetrahedron finite-difference stencil
    /// (identical to `HOLOGRAPHIC_WGSL`).
    #[inline]
    pub fn gradient_normal_local(&self, p: [f32; 3]) -> [f32; 3] {
        const EPS: f32 = 1e-4;
        // Tetrahedron vertices: k0=(1,-1,-1), k1=(-1,-1,1), k2=(-1,1,-1), k3=(1,1,1)
        let d0 = self.evaluate_local([p[0] + EPS, p[1] - EPS, p[2] - EPS]);
        let d1 = self.evaluate_local([p[0] - EPS, p[1] - EPS, p[2] + EPS]);
        let d2 = self.evaluate_local([p[0] - EPS, p[1] + EPS, p[2] - EPS]);
        let d3 = self.evaluate_local([p[0] + EPS, p[1] + EPS, p[2] + EPS]);

        let gx = d0 - d1 - d2 + d3;
        let gy = -d0 - d1 + d2 + d3;
        let gz = -d0 + d1 - d2 + d3;
        normalize3_fallback([gx, gy, gz], [0.0, 1.0, 0.0])
    }

    /// Evaluates signed distance and world-space outward normal for a world query point `p_world`
    /// given the SDF collider's world PGA [`Motor`].
    #[inline]
    pub fn evaluate_world(&self, motor_world: &Motor, p_world: [f32; 3]) -> (f32, [f32; 3]) {
        let inv_motor = motor_world.reverse();
        let p_local = inv_motor.transform_point(p_world);
        let dist = self.evaluate_local(p_local);
        let n_local = self.gradient_normal_local(p_local);
        let n_world = motor_world.transform_vector(n_local);
        (dist, normalize3_fallback(n_world, [0.0, 1.0, 0.0]))
    }

    /// Returns conservative local-space half-extents `[hx, hy, hz]` enclosing the SDF zero-isosurface.
    #[inline]
    pub fn local_half_extents(&self) -> [f32; 3] {
        match *self {
            Self::Sphere { radius } => [radius, radius, radius],
            Self::RoundedBox {
                half_extents,
                bevel_radius,
            } => [
                half_extents[0] + bevel_radius,
                half_extents[1] + bevel_radius,
                half_extents[2] + bevel_radius,
            ],
            Self::Capsule {
                radius,
                half_height,
            } => [radius, half_height + radius, radius],
            Self::Torus {
                major_radius,
                minor_radius,
            } => [
                major_radius + minor_radius,
                minor_radius,
                major_radius + minor_radius,
            ],
            Self::SmoothUnionPair {
                center_a,
                radius_a,
                center_b,
                radius_b,
                blend_radius,
            } => {
                let pad = blend_radius * 0.5;
                let hx = (center_a[0].abs() + radius_a).max(center_b[0].abs() + radius_b) + pad;
                let hy = (center_a[1].abs() + radius_a).max(center_b[1].abs() + radius_b) + pad;
                let hz = (center_a[2].abs() + radius_a).max(center_b[2].abs() + radius_b) + pad;
                [hx, hy, hz]
            }
        }
    }
}

#[inline]
fn sub3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

#[inline]
fn len3(v: [f32; 3]) -> f32 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

#[inline]
fn normalize3_fallback(v: [f32; 3], fallback: [f32; 3]) -> [f32; 3] {
    let l = len3(v);
    if l < 1e-8 {
        fallback
    } else {
        [v[0] / l, v[1] / l, v[2] / l]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qualia_core_db::render::pga::motor_translate;

    #[test]
    fn sdf_primitives_distance_and_tetrahedron_gradient_are_exact() {
        let sphere = SdfPrimitive::Sphere { radius: 2.0 };
        assert!((sphere.evaluate_local([3.0, 0.0, 0.0]) - 1.0).abs() < 1e-5);
        assert!((sphere.evaluate_local([1.0, 0.0, 0.0]) + 1.0).abs() < 1e-5);
        let n = sphere.gradient_normal_local([3.0, 0.0, 0.0]);
        assert!((n[0] - 1.0).abs() < 1e-4);
        assert!(n[1].abs() < 1e-4);
        assert!(n[2].abs() < 1e-4);

        // Smooth union pair with PROC1 evaluate_smooth_minimum_sdf
        let union = SdfPrimitive::SmoothUnionPair {
            center_a: [-0.5, 0.0, 0.0],
            radius_a: 0.6,
            center_b: [0.5, 0.0, 0.0],
            radius_b: 0.6,
            blend_radius: 0.4,
        };
        // Origin is inside the smooth bridge between the two spheres
        let d_mid = union.evaluate_local([0.0, 0.0, 0.0]);
        assert!(d_mid < 0.0, "origin should be inside smooth union, got {d_mid}");

        let motor = motor_translate([10.0, 2.0, -3.0]);
        let (d_world, n_world) = sphere.evaluate_world(&motor, [10.0, 5.0, -3.0]);
        assert!((d_world - 1.0).abs() < 1e-4);
        assert!((n_world[1] - 1.0).abs() < 1e-4);
    }
}
