//! The `GeometryKernel` trait — the abstraction over predicate
//! implementations that lets the same algorithm run over a filtered `f64`
//! kernel (fast, the default) or an exact-arithmetic kernel (robust, the
//! degeneracy fallback). P1.2.
//!
//! ## Why a trait, not free functions
//!
//! `orientation_2` today is a free function in [`super::primitives`]. When the
//! only implementation is filtered `f64`, that's fine. But the execution plan
//! (P1.4–P1.7) calls for a **filtered → compensated → exact ladder**: the same
//! algorithm (hull, Delaunay, boolean) must run unchanged whether the predicate
//! is the fast filtered path or the slow exact path. The trait is the seam
//! where the kernel is swapped without touching the algorithm.
//!
//! ## Zero-heap contract
//!
//! The trait methods take `&self` and return a small enum — no `Vec`, `String`,
//! or `Box` in any predicate path (AGENTS.md §0). The exact kernel (P1.7) will
//! carry a caller-owned expansion-arithmetic workspace as a `&mut [u64]` borrow
//! inside its kernel struct; that workspace is stack/caller-allocated, not
//! heap. The filtered kernel ([`FilteredF64Kernel`]) is zero-sized and
//! `Copy`.
//!
//! ## P1.2 scope
//!
//! Only `orientation_2` is implemented today, so the trait has one method.
//! P1.4 adds `orient_3d`, P1.5 adds `incircle`, P1.6 adds `insphere`, P1.7
//! adds exact construction. Each lands as a new trait method with a default
//! that panics-by-contract (a kernel that doesn't implement a predicate cannot
//! be used by an algorithm that needs it — fail-closed, not silent).

use super::expansion::Sign;
use super::incircle::incircle as filtered_incircle;
use super::insphere::insphere as filtered_insphere;
use super::orient3d::orient_3d as filtered_orient_3d;
use super::primitives::{orientation_2 as filtered_orientation_2, Orientation, Point2, Point3};

/// The geometric-predicate kernel abstraction.
///
/// Implementors provide the sign of geometric predicates (orientation,
/// incircle, insphere) under a specific number model. The default
/// [`FilteredF64Kernel`] is the fast filtered-`f64` path; a future exact
/// kernel (P1.7) will provide the robust fallback for degenerate cases.
pub trait GeometryKernel {
    /// 2D orientation: the sign of the turn `a → b → c`.
    ///
    /// `CounterClockwise` / `Collinear` / `Clockwise`. This is the predicate
    /// `convex_hull_2` and (future) `delaunay_2` are built on.
    fn orientation_2(&self, a: Point2, b: Point2, c: Point2) -> Orientation;

    /// 3D orientation: the sign of `det(b−a, c−a, d−a)` (the signed volume of
    /// tetrahedron `a b c d`). [`Sign::Positive`] = `d` below the oriented
    /// plane `a → b → c`; [`Sign::Negative`] = above; [`Sign::Zero`] = coplanar.
    ///
    /// P1.4. The default fails closed — a kernel that does not implement 3D
    /// orientation cannot be used by an algorithm that needs it.
    fn orient_3d(&self, _a: Point3, _b: Point3, _c: Point3, _d: Point3) -> Sign {
        panic!("GeometryKernel::orient_3d not implemented for this kernel")
    }

    /// 2D in-circle: the side of `d` w.r.t. the oriented circle through
    /// `a, b, c`. [`Sign::Positive`] = inside (when `a, b, c` are CCW);
    /// [`Sign::Zero`] = on; [`Sign::Negative`] = outside.
    ///
    /// P1.5. The default fails closed.
    fn incircle(&self, _a: Point2, _b: Point2, _c: Point2, _d: Point2) -> Sign {
        panic!("GeometryKernel::incircle not implemented for this kernel")
    }

    /// 3D in-sphere: the side of `e` w.r.t. the oriented sphere through
    /// `a, b, c, d`. [`Sign::Positive`] = inside (when `a, b, c, d` are
    /// positively oriented); [`Sign::Zero`] = on; [`Sign::Negative`] = outside.
    ///
    /// P1.6. The default fails closed.
    fn insphere(&self, _a: Point3, _b: Point3, _c: Point3, _d: Point3, _e: Point3) -> Sign {
        panic!("GeometryKernel::insphere not implemented for this kernel")
    }
}

/// The default filtered-`f64` kernel — the fast path.
///
/// Uses [`super::primitives::orientation_2`] (filtered determinant + FMA
/// compensation near cancellation) and the P1.4–P1.6 predicate ladders
/// (filtered → compensated → exact, zero-heap). Zero-sized and `Copy`: pass it
/// by value or reference; there is no state. This is the kernel every existing
/// caller uses implicitly today; P1.2 makes that explicit.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FilteredF64Kernel;

impl GeometryKernel for FilteredF64Kernel {
    #[inline]
    fn orientation_2(&self, a: Point2, b: Point2, c: Point2) -> Orientation {
        filtered_orientation_2(a, b, c)
    }

    #[inline]
    fn orient_3d(&self, a: Point3, b: Point3, c: Point3, d: Point3) -> Sign {
        filtered_orient_3d(a, b, c, d)
    }

    #[inline]
    fn incircle(&self, a: Point2, b: Point2, c: Point2, d: Point2) -> Sign {
        filtered_incircle(a, b, c, d)
    }

    #[inline]
    fn insphere(&self, a: Point3, b: Point3, c: Point3, d: Point3, e: Point3) -> Sign {
        filtered_insphere(a, b, c, d, e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filtered_kernel_matches_free_function() {
        let k = FilteredF64Kernel::default();
        let a = Point2::new(0.0, 0.0);
        let b = Point2::new(1.0, 0.0);
        let c = Point2::new(1.0, 1.0);
        assert_eq!(k.orientation_2(a, b, c), filtered_orientation_2(a, b, c));
        assert_eq!(k.orientation_2(a, b, c), Orientation::CounterClockwise);
    }

    #[test]
    fn filtered_kernel_is_zero_sized() {
        assert_eq!(std::mem::size_of::<FilteredF64Kernel>(), 0);
    }

    #[test]
    fn filtered_kernel_classifies_all_three_turns() {
        let k = FilteredF64Kernel::default();
        let a = Point2::new(0.0, 0.0);
        let b = Point2::new(1.0, 0.0);
        assert_eq!(k.orientation_2(a, b, Point2::new(1.0, 1.0)), Orientation::CounterClockwise);
        assert_eq!(k.orientation_2(a, b, Point2::new(1.0, -1.0)), Orientation::Clockwise);
        assert_eq!(k.orientation_2(a, b, Point2::new(2.0, 0.0)), Orientation::Collinear);
    }
}
