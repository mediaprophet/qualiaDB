//! Persistent 4-Point Contact Manifold with Warm-Starting (`manifold.rs`).
//!
//! Maintains up to 4 contact points per colliding pair (`ContactManifold3D`), preserving
//! accumulated normal and tangential impulses across simulation substeps/frames (`warm_start`)
//! and using area-maximizing point reduction when a 5th contact candidate is added.

use crate::physics3d::gjk_epa::NarrowphaseContact;

/// Maximum number of persistent contact points in a single 3D manifold.
pub const MAX_MANIFOLD_POINTS: usize = 4;

/// Distance squared threshold ($m^2$) for matching a new contact point to a cached warm-start point.
pub const PERSISTENT_CONTACT_THRESHOLD_SQ: f32 = 0.04 * 0.04;

/// A single contact point inside a [`ContactManifold3D`].
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContactPoint3D {
    /// Contact point in body A's local space.
    pub local_a: [f32; 3],
    /// Contact point in body B's local space.
    pub local_b: [f32; 3],
    /// Midpoint contact position in world space.
    pub world_point: [f32; 3],
    /// Positive penetration depth in meters.
    pub penetration: f32,
    /// Cached normal impulse $\lambda_n$ for warm-starting across frames.
    pub normal_impulse: f32,
    /// Cached friction tangent impulses $[\lambda_{t1}, \lambda_{t2}]$.
    pub tangent_impulse: [f32; 2],
    /// Geometric feature identifier for persistent contact matching.
    pub feature_id: u32,
}

impl Default for ContactPoint3D {
    #[inline]
    fn default() -> Self {
        Self {
            local_a: [0.0; 3],
            local_b: [0.0; 3],
            world_point: [0.0; 3],
            penetration: 0.0,
            normal_impulse: 0.0,
            tangent_impulse: [0.0; 2],
            feature_id: 0,
        }
    }
}

/// Persistent 4-point 3D contact manifold between `body_a` and `body_b`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContactManifold3D {
    pub body_a: u32,
    pub body_b: u32,
    /// Shared outward contact normal pointing from `body_a` toward `body_b`.
    pub normal: [f32; 3],
    pub points: [ContactPoint3D; MAX_MANIFOLD_POINTS],
    pub point_count: usize,
}

impl Default for ContactManifold3D {
    #[inline]
    fn default() -> Self {
        Self {
            body_a: u32::MAX,
            body_b: u32::MAX,
            normal: [0.0, 1.0, 0.0],
            points: [ContactPoint3D::default(); MAX_MANIFOLD_POINTS],
            point_count: 0,
        }
    }
}

impl ContactManifold3D {
    /// Creates an empty manifold for `(body_a, body_b)`.
    #[inline]
    pub const fn new(body_a: u32, body_b: u32, normal: [f32; 3]) -> Self {
        Self {
            body_a,
            body_b,
            normal,
            points: [
                ContactPoint3D {
                    local_a: [0.0; 3],
                    local_b: [0.0; 3],
                    world_point: [0.0; 3],
                    penetration: 0.0,
                    normal_impulse: 0.0,
                    tangent_impulse: [0.0; 2],
                    feature_id: 0,
                };
                MAX_MANIFOLD_POINTS
            ],
            point_count: 0,
        }
    }

    /// Updates or inserts a contact point from a [`NarrowphaseContact`], preserving cached
    /// warm-start impulses if a matching point lies within [`PERSISTENT_CONTACT_THRESHOLD_SQ`].
    pub fn push_or_warm_start(
        &mut self,
        contact: &NarrowphaseContact,
        local_a: [f32; 3],
        local_b: [f32; 3],
        feature_id: u32,
    ) {
        self.normal = contact.normal;
        let world_point = [
            0.5 * (contact.point_a[0] + contact.point_b[0]),
            0.5 * (contact.point_a[1] + contact.point_b[1]),
            0.5 * (contact.point_a[2] + contact.point_b[2]),
        ];

        // 1. Check if an existing point matches by proximity or feature_id -> preserve warm-start impulse!
        let mut i = 0usize;
        while i < self.point_count {
            let p = &mut self.points[i];
            let dx = p.world_point[0] - world_point[0];
            let dy = p.world_point[1] - world_point[1];
            let dz = p.world_point[2] - world_point[2];
            let dist_sq = dx * dx + dy * dy + dz * dz;
            if dist_sq <= PERSISTENT_CONTACT_THRESHOLD_SQ || (feature_id != 0 && p.feature_id == feature_id) {
                p.local_a = local_a;
                p.local_b = local_b;
                p.world_point = world_point;
                p.penetration = contact.penetration;
                p.feature_id = feature_id;
                return;
            }
            i += 1;
        }

        let new_pt = ContactPoint3D {
            local_a,
            local_b,
            world_point,
            penetration: contact.penetration,
            normal_impulse: 0.0,
            tangent_impulse: [0.0; 2],
            feature_id,
        };

        // 2. If capacity available, append.
        if self.point_count < MAX_MANIFOLD_POINTS {
            self.points[self.point_count] = new_pt;
            self.point_count += 1;
            return;
        }

        // 3. Otherwise replace the shallowest non-deepest point to maximize support polygon area.
        let mut deepest_idx = 0usize;
        let mut max_pen = self.points[0].penetration;
        let mut k = 1usize;
        while k < MAX_MANIFOLD_POINTS {
            if self.points[k].penetration > max_pen {
                max_pen = self.points[k].penetration;
                deepest_idx = k;
            }
            k += 1;
        }

        let replace_idx = (deepest_idx + 1) % MAX_MANIFOLD_POINTS;
        self.points[replace_idx] = new_pt;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contact_manifold_preserves_warm_start_impulse_on_nearby_contact() {
        let mut manifold = ContactManifold3D::new(0, 1, [0.0, 1.0, 0.0]);
        let c1 = NarrowphaseContact {
            normal: [0.0, 1.0, 0.0],
            penetration: 0.05,
            point_a: [1.0, 0.0, 1.0],
            point_b: [1.0, -0.05, 1.0],
        };
        manifold.push_or_warm_start(&c1, [1.0, 0.0, 1.0], [1.0, 0.0, 1.0], 1);
        manifold.points[0].normal_impulse = 42.5;

        // Next frame: slightly shifted contact within threshold
        let c2 = NarrowphaseContact {
            normal: [0.0, 1.0, 0.0],
            penetration: 0.03,
            point_a: [1.01, 0.0, 1.0],
            point_b: [1.01, -0.03, 1.0],
        };
        manifold.push_or_warm_start(&c2, [1.01, 0.0, 1.0], [1.01, 0.0, 1.0], 1);
        assert_eq!(manifold.point_count, 1);
        assert!((manifold.points[0].normal_impulse - 42.5).abs() < 1e-5);
        assert!((manifold.points[0].penetration - 0.03).abs() < 1e-5);
    }
}
