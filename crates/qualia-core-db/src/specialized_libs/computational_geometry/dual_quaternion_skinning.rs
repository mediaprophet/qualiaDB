//! P21.7 — Dual-quaternion skinning (DQS) for skeletal articulation
//! (Kavan, Collins, Žára & O'Sullivan 2007).
//!
//! A unit dual quaternion `q̂ = q_r + ε q_d` encodes rigid rotation +
//! translation. Blending `q̂`s and transforming points avoids the
//! "candy-wrapper" volume collapse of linear blend skinning (LBS) under
//! twist — volume and bone length are preserved.
//!
//! All kernels are Tier-1 `HotZeroHeap`.

use super::math_geometry::Quaternion;
use super::primitives::Point3;

/// Unit dual quaternion: `re` is the rotation quaternion, `du` carries the
/// translation (`du = ½ · t · re`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DualQuaternion {
    pub re: Quaternion,
    pub du: Quaternion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DualQuaternionError {
    NonUnit,
    DimensionMismatch,
}

#[inline]
fn q_mul(a: Quaternion, b: Quaternion) -> Quaternion {
    Quaternion {
        w: a.w * b.w - a.x * b.x - a.y * b.y - a.z * b.z,
        x: a.w * b.x + a.x * b.w + a.y * b.z - a.z * b.y,
        y: a.w * b.y - a.x * b.z + a.y * b.w + a.z * b.x,
        z: a.w * b.z + a.x * b.y - a.y * b.x + a.z * b.w,
    }
}
#[inline]
fn q_add(a: Quaternion, b: Quaternion) -> Quaternion {
    Quaternion {
        w: a.w + b.w,
        x: a.x + b.x,
        y: a.y + b.y,
        z: a.z + b.z,
    }
}
#[inline]
fn q_scale(a: Quaternion, s: f64) -> Quaternion {
    Quaternion {
        w: a.w * s,
        x: a.x * s,
        y: a.y * s,
        z: a.z * s,
    }
}
#[inline]
fn q_conj(a: Quaternion) -> Quaternion {
    Quaternion {
        w: a.w,
        x: -a.x,
        y: -a.y,
        z: -a.z,
    }
}
#[inline]
fn q_dot(a: Quaternion, b: Quaternion) -> f64 {
    a.w * b.w + a.x * b.x + a.y * b.y + a.z * b.z
}
#[inline]
fn q_norm(a: Quaternion) -> f64 {
    q_dot(a, a).sqrt()
}

impl DualQuaternion {
    /// Identity rigid transform.
    pub const IDENTITY: Self = Self {
        re: Quaternion {
            w: 1.0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        du: Quaternion {
            w: 0.0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
    };

    /// Build a unit dual quaternion from rotation `q` and translation `t`.
    pub fn from_rotation_translation(q: Quaternion, t: Point3) -> Self {
        let qt = Quaternion {
            w: 0.0,
            x: t.x,
            y: t.y,
            z: t.z,
        };
        Self {
            re: q,
            du: q_scale(q_mul(qt, q), 0.5),
        }
    }

    /// `q̂₁ ⊗ q̂₂` — rigid-transform composition.
    pub fn mul(self, o: DualQuaternion) -> Self {
        Self {
            re: q_mul(self.re, o.re),
            du: q_add(q_mul(self.re, o.du), q_mul(self.du, o.re)),
        }
    }

    /// Normalize to a unit dual quaternion.
    pub fn normalized(self) -> Result<Self, DualQuaternionError> {
        let n = q_norm(self.re);
        if !(n > 1e-12) {
            return Err(DualQuaternionError::NonUnit);
        }
        Ok(Self {
            re: q_scale(self.re, 1.0 / n),
            du: q_scale(self.du, 1.0 / n),
        })
    }

    /// ScLERP — dual-quaternion spherical blend with antipodal fix.
    pub fn sclerp(a: Self, b: Self, t: f64) -> Self {
        let sign = if q_dot(a.re, b.re) < 0.0 { -1.0 } else { 1.0 };
        let bre = q_scale(b.re, sign);
        let bdu = q_scale(b.du, sign);
        Self {
            re: q_add(a.re, q_scale(q_add(bre, q_scale(a.re, -1.0)), t)),
            du: q_add(a.du, q_scale(q_add(bdu, q_scale(a.du, -1.0)), t)),
        }
        .normalized()
        .unwrap_or(Self::IDENTITY)
    }

    /// Transform point `p` by this unit dual quaternion.
    pub fn transform_point(self, p: Point3) -> Point3 {
        let dq = self.normalized().unwrap_or(self);
        // rotation: p' = re · p · conj(re); translation from 2·du·conj(re).
        let qp = Quaternion {
            w: 0.0,
            x: p.x,
            y: p.y,
            z: p.z,
        };
        let pr = q_mul(q_mul(dq.re, qp), q_conj(dq.re));
        let tq = q_mul(dq.du, q_conj(dq.re));
        Point3::new(pr.x + 2.0 * tq.x, pr.y + 2.0 * tq.y, pr.z + 2.0 * tq.z)
    }
}

/// Linear blend + normalize of dual quaternions (the standard DQS blend):
/// `q̂ = Σ w_i q̂_i / |Σ w_i q̂_i|` with antipodal sign alignment to `q̂_0`.
///
/// Tier-1 `HotZeroHeap`.
pub fn dqs_blend(
    bones: &[DualQuaternion],
    weights: &[f64],
) -> Result<DualQuaternion, DualQuaternionError> {
    if bones.is_empty() || bones.len() != weights.len() {
        return Err(DualQuaternionError::DimensionMismatch);
    }
    let mut acc = DualQuaternion {
        re: Quaternion {
            w: 0.0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        du: Quaternion {
            w: 0.0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
    };
    let anchor = bones[0].re;
    for (dq, &w) in bones.iter().zip(weights) {
        let s = if q_dot(anchor, dq.re) < 0.0 { -w } else { w };
        acc.re = q_add(acc.re, q_scale(dq.re, s));
        acc.du = q_add(acc.du, q_scale(dq.du, s));
    }
    acc.normalized()
}

/// Skin `point` with weighted `bones` — returns the deformed position.
///
/// Tier-1 `HotZeroHeap`.
pub fn dqs_skin_vertex(
    point: Point3,
    bones: &[DualQuaternion],
    weights: &[f64],
) -> Result<Point3, DualQuaternionError> {
    Ok(dqs_blend(bones, weights)?.transform_point(point))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specialized_libs::computational_geometry::math_geometry::quaternion_normalize;

    fn rot_x(deg: f64) -> Quaternion {
        let h = deg.to_radians() * 0.5;
        quaternion_normalize(Quaternion {
            w: h.cos(),
            x: h.sin(),
            y: 0.0,
            z: 0.0,
        })
    }

    #[test]
    fn rigid_transform_preserves_distances() {
        let bone =
            DualQuaternion::from_rotation_translation(rot_x(90.0), Point3::new(1.0, 2.0, 3.0));
        let a = Point3::new(0.0, 0.0, 0.0);
        let b = Point3::new(0.0, 1.0, 0.0);
        let pa = bone.transform_point(a);
        let pb = bone.transform_point(b);
        let d = ((pa.x - pb.x).powi(2) + (pa.y - pb.y).powi(2) + (pa.z - pb.z).powi(2)).sqrt();
        assert!(
            (d - 1.0).abs() < 1e-9,
            "rigid transform broke distance: {d}"
        );
        assert!((pa.x - 1.0).abs() < 1e-9 && (pa.y - 2.0).abs() < 1e-9);
    }

    #[test]
    fn dqs_preserves_volume_under_twist() {
        // Classic candy-wrapper: 50/50 blend of identity and 180° twist.
        let identity = DualQuaternion::IDENTITY;
        let twist = DualQuaternion::from_rotation_translation(rot_x(180.0), Point3::default());
        let p = Point3::new(0.0, 1.0, 0.0);
        let out = dqs_skin_vertex(p, &[identity, twist], &[0.5, 0.5]).unwrap();
        // DQS rotates by ~90° — point keeps unit length; LBS would collapse
        // to ~0.
        let len = (out.x * out.x + out.y * out.y + out.z * out.z).sqrt();
        assert!((len - 1.0).abs() < 1e-6, "DQS lost volume: |p'|={len}");
    }
}
