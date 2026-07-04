use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

use crate::tensor::Tensor10D;

/// POD 2D point used by CPU, WASM, and serialized tool boundaries.
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct Point2 {
    pub x: f64,
    pub y: f64,
}

impl Point2 {
    #[inline]
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// POD 3D point used by mesh and spatial-index ports.
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct Point3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Point3 {
    #[inline]
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }
}

#[repr(i8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Orientation {
    Clockwise = -1,
    Collinear = 0,
    CounterClockwise = 1,
}

/// Filtered 2D orientation predicate.
///
/// The common path is one determinant. Near cancellation, `mul_add` recovers
/// each product residual and forms a compensated determinant. For Qualia 10D
/// coordinates (stored as `f32`) conversion to `f64` makes the products exact,
/// so [`orientation_2_tensor_xy`] is robust for every finite tensor coordinate.
#[inline]
pub fn orientation_2(a: Point2, b: Point2, c: Point2) -> Orientation {
    let acx = a.x - c.x;
    let bcx = b.x - c.x;
    let acy = a.y - c.y;
    let bcy = b.y - c.y;
    let left = acx * bcy;
    let right = acy * bcx;
    let det = left - right;

    let scale = left.abs() + right.abs();
    let error_bound = scale * (8.0 * f64::EPSILON);
    let resolved = if det.abs() > error_bound {
        det
    } else {
        // Exact residuals of the rounded products on targets with IEEE-754 FMA.
        let left_error = acx.mul_add(bcy, -left);
        let right_error = acy.mul_add(bcx, -right);
        det + (left_error - right_error)
    };

    if resolved > 0.0 {
        Orientation::CounterClockwise
    } else if resolved < 0.0 {
        Orientation::Clockwise
    } else {
        Orientation::Collinear
    }
}

/// Orientation over the spatial `(x,y)` plane of three 10D manifold nodes.
#[inline]
pub fn orientation_2_tensor_xy(a: &Tensor10D, b: &Tensor10D, c: &Tensor10D) -> Orientation {
    orientation_2(
        Point2::new(a.x as f64, a.y as f64),
        Point2::new(b.x as f64, b.y as f64),
        Point2::new(c.x as f64, c.y as f64),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orientation_classifies_turns() {
        let a = Point2::new(0.0, 0.0);
        let b = Point2::new(1.0, 0.0);
        assert_eq!(
            orientation_2(a, b, Point2::new(1.0, 1.0)),
            Orientation::CounterClockwise
        );
        assert_eq!(
            orientation_2(a, b, Point2::new(1.0, -1.0)),
            Orientation::Clockwise
        );
        assert_eq!(
            orientation_2(a, b, Point2::new(2.0, 0.0)),
            Orientation::Collinear
        );
    }

    #[test]
    fn tensor_predicate_uses_spatial_plane() {
        let mut a = Tensor10D::default();
        let mut b = Tensor10D::default();
        let mut c = Tensor10D::default();
        b.x = 1.0;
        c.x = 1.0;
        c.y = 1.0;
        a.q = 4.0;
        b.v = 2.0;
        c.sigma = 9.0;
        assert_eq!(
            orientation_2_tensor_xy(&a, &b, &c),
            Orientation::CounterClockwise
        );
    }
}
