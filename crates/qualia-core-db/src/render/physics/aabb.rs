//! Axis-aligned bounding box for an artefact, and the rigid+scale transform of its extent.
//!
//! Zero-alloc: every operation works over fixed `[f32; 3]` arrays and an 8-corner loop — no `Vec`,
//! no heap, suitable for the hot path. Rotation/translation come from the PGA motor oracle
//! (`render::pga`), the same map the GPU projector uses, so artefact physics and rendering agree.

use crate::render::pga::{sandwich_point, Motor};

/// An axis-aligned bounding box (model or world space).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aabb {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

impl Aabb {
    #[inline]
    pub fn new(min: [f32; 3], max: [f32; 3]) -> Self {
        Aabb { min, max }
    }

    /// AABB enclosing a point set; `None` if empty. (No allocation — single pass.)
    pub fn from_points(points: &[[f32; 3]]) -> Option<Aabb> {
        let first = *points.first()?;
        let mut min = first;
        let mut max = first;
        for p in &points[1..] {
            for k in 0..3 {
                if p[k] < min[k] {
                    min[k] = p[k];
                }
                if p[k] > max[k] {
                    max[k] = p[k];
                }
            }
        }
        Some(Aabb { min, max })
    }

    #[inline]
    pub fn center(&self) -> [f32; 3] {
        [
            0.5 * (self.min[0] + self.max[0]),
            0.5 * (self.min[1] + self.max[1]),
            0.5 * (self.min[2] + self.max[2]),
        ]
    }

    /// Per-axis extent (`max - min`).
    #[inline]
    pub fn extent(&self) -> [f32; 3] {
        [
            self.max[0] - self.min[0],
            self.max[1] - self.min[1],
            self.max[2] - self.min[2],
        ]
    }

    /// Enclosed volume (product of extents).
    #[inline]
    pub fn volume(&self) -> f32 {
        let e = self.extent();
        e[0] * e[1] * e[2]
    }

    #[inline]
    pub fn contains_point(&self, p: [f32; 3]) -> bool {
        (0..3).all(|k| p[k] >= self.min[k] && p[k] <= self.max[k])
    }

    /// Whether `self` fully encloses `other`.
    #[inline]
    pub fn contains(&self, other: &Aabb) -> bool {
        (0..3).all(|k| other.min[k] >= self.min[k] && other.max[k] <= self.max[k])
    }

    /// Transform the box by a per-axis `scale` about its own centre, then a rigid PGA `motor`
    /// (rotation + translation), and return the enclosing AABB of the result. Zero-alloc.
    pub fn transformed(&self, motor: Motor, scale: [f32; 3]) -> Aabb {
        let c = self.center();
        let mut min = [f32::INFINITY; 3];
        let mut max = [f32::NEG_INFINITY; 3];
        for ix in 0..8u8 {
            let corner = [
                if ix & 1 == 0 {
                    self.min[0]
                } else {
                    self.max[0]
                },
                if ix & 2 == 0 {
                    self.min[1]
                } else {
                    self.max[1]
                },
                if ix & 4 == 0 {
                    self.min[2]
                } else {
                    self.max[2]
                },
            ];
            let scaled = [
                c[0] + (corner[0] - c[0]) * scale[0],
                c[1] + (corner[1] - c[1]) * scale[1],
                c[2] + (corner[2] - c[2]) * scale[2],
            ];
            let w = sandwich_point(motor, scaled);
            for k in 0..3 {
                if w[k] < min[k] {
                    min[k] = w[k];
                }
                if w[k] > max[k] {
                    max[k] = w[k];
                }
            }
        }
        Aabb { min, max }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit() -> Aabb {
        Aabb::new([-1.0, -1.0, -1.0], [1.0, 1.0, 1.0])
    }

    #[test]
    fn extent_volume_center() {
        let b = unit();
        assert_eq!(b.extent(), [2.0, 2.0, 2.0]);
        assert_eq!(b.volume(), 8.0);
        assert_eq!(b.center(), [0.0, 0.0, 0.0]);
    }

    #[test]
    fn identity_transform_is_noop() {
        let b = unit();
        let out = b.transformed(Motor::identity(), [1.0, 1.0, 1.0]);
        for k in 0..3 {
            assert!((out.min[k] - b.min[k]).abs() < 1e-5);
            assert!((out.max[k] - b.max[k]).abs() < 1e-5);
        }
    }

    #[test]
    fn scale_shrinks_extent_about_centre() {
        let out = unit().transformed(Motor::identity(), [0.5, 0.5, 0.5]);
        assert!((out.extent()[0] - 1.0).abs() < 1e-5); // 2.0 * 0.5
        assert_eq!(out.center(), [0.0, 0.0, 0.0]); // centre preserved
    }

    #[test]
    fn contains_and_from_points() {
        let b = Aabb::from_points(&[[0.0, 0.0, 0.0], [1.0, 2.0, 3.0], [-1.0, 0.0, 0.0]]).unwrap();
        assert_eq!(b.min, [-1.0, 0.0, 0.0]);
        assert_eq!(b.max, [1.0, 2.0, 3.0]);
        assert!(b.contains_point([0.0, 1.0, 1.0]));
        assert!(!b.contains_point([5.0, 0.0, 0.0]));
        assert!(Aabb::from_points(&[]).is_none());
    }
}
