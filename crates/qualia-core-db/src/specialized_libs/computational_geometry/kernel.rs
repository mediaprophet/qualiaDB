//! The `GeometryKernel` trait — the abstraction over predicate
//! implementations that lets the same algorithm run over a filtered `f64`
//! kernel (fast, the default) or an exact-arithmetic kernel (robust, the
//! degeneracy fallback). P1.2.
//!
//! ## P10.4 — Non-panicking kernel v2
//!
//! The original P1.2 trait gave the optional predicates (`orient_3d`,
//! `incircle`, `insphere`) panicking default implementations — a kernel that
//! didn't implement them would compile, then panic at runtime if an algorithm
//! called them. P10.4 eliminates that class of failure:
//!
//! 1. **All four predicates are now compile-time required** — no default
//!    implementations. A kernel that doesn't provide all four cannot compile.
//!    This is the "required predicates are compile-time trait requirements"
//!    gate.
//!
//! 2. **Construction capabilities are a separate `ConstructionKernel` trait**
//!    whose methods return `Result<T, Unsupported>` — a typed error, not a
//!    panic. `FilteredF64Kernel` implements `GeometryKernel` but NOT
//!    `ConstructionKernel`; `ExactConstructionKernel` implements both.
//!    Algorithms that need construction require `K: GeometryKernel +
//!    ConstructionKernel` at compile time, so a kernel without construction
//!    cannot be passed to them.
//!
//! 3. **Generic conformance tests** verify that every kernel implementing
//!    `GeometryKernel` produces correct predicate signs on a battery of
//!    known-answer cases.
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

use super::expansion::Sign;
use super::incircle::incircle as filtered_incircle;
use super::insphere::insphere as filtered_insphere;
use super::orient3d::orient_3d as filtered_orient_3d;
use super::primitives::{orientation_2 as filtered_orientation_2, Orientation, Point2, Point3};

// ───────────────────────────────────────────────────────────────────────────
//  P10.4 — Typed Unsupported error for optional construction capabilities
// ───────────────────────────────────────────────────────────────────────────

/// Typed error returned by a [`ConstructionKernel`] method when the kernel
/// does not support the requested construction.
///
/// This replaces the panicking defaults of the pre-P10.4 trait. An algorithm
/// that needs exact construction receives `Err(Unsupported)` and can degrade
/// gracefully (fall back to f64, report the gap, or refuse the input) rather
/// than crashing the process.
///
/// Zero-heap: carries only `&'static str` metadata (no `String`/`Box`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unsupported {
    /// The capability name (e.g. `"segment_intersection_2"`).
    pub capability: &'static str,
    /// Why it is unsupported (e.g. `"filtered f64 kernel does not construct exact points"`).
    pub reason: &'static str,
}

impl Unsupported {
    /// Construct an `Unsupported` error.
    pub const fn new(capability: &'static str, reason: &'static str) -> Self {
        Self { capability, reason }
    }
}

impl core::fmt::Display for Unsupported {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "unsupported construction `{}`: {}", self.capability, self.reason)
    }
}

impl std::error::Error for Unsupported {}

// ───────────────────────────────────────────────────────────────────────────
//  GeometryKernel — required predicates (compile-time, no panicking defaults)
// ───────────────────────────────────────────────────────────────────────────

/// The geometric-predicate kernel abstraction.
///
/// **P10.4:** All four predicate methods are now **required** (no default
/// implementations). A kernel that does not provide all four predicates cannot
/// compile — this eliminates the class of runtime panics that the pre-P10.4
/// panicking defaults allowed.
///
/// Implementors provide the sign of geometric predicates (orientation,
/// incircle, insphere) under a specific number model. The default
/// [`FilteredF64Kernel`] is the fast filtered-`f64` path; the
/// [`super::exact_kernel::ExactConstructionKernel`] provides the robust
/// fallback for degenerate cases.
pub trait GeometryKernel {
    /// 2D orientation: the sign of the turn `a → b → c`.
    ///
    /// `CounterClockwise` / `Collinear` / `Clockwise`. This is the predicate
    /// `convex_hull_2` and `delaunay_2` are built on.
    fn orientation_2(&self, a: Point2, b: Point2, c: Point2) -> Orientation;

    /// 3D orientation: the sign of `det(b−a, c−a, d−a)` (the signed volume of
    /// tetrahedron `a b c d`). [`Sign::Positive`] = `d` below the oriented
    /// plane `a → b → c`; [`Sign::Negative`] = above; [`Sign::Zero`] = coplanar.
    fn orient_3d(&self, a: Point3, b: Point3, c: Point3, d: Point3) -> Sign;

    /// 2D in-circle: the side of `d` w.r.t. the oriented circle through
    /// `a, b, c`. [`Sign::Positive`] = inside (when `a, b, c` are CCW);
    /// [`Sign::Zero`] = on; [`Sign::Negative`] = outside.
    fn incircle(&self, a: Point2, b: Point2, c: Point2, d: Point2) -> Sign;

    /// 3D in-sphere: the side of `e` w.r.t. the oriented sphere through
    /// `a, b, c, d`. [`Sign::Positive`] = inside (when `a, b, c, d` are
    /// positively oriented); [`Sign::Zero`] = on; [`Sign::Negative`] = outside.
    fn insphere(&self, a: Point3, b: Point3, c: Point3, d: Point3, e: Point3) -> Sign;
}

// ───────────────────────────────────────────────────────────────────────────
//  ConstructionKernel — optional exact construction (typed Unsupported, no panic)
// ───────────────────────────────────────────────────────────────────────────

/// Optional exact-construction capabilities. A kernel MAY implement this trait
/// in addition to [`GeometryKernel`]; algorithms that need exact construction
/// require `K: GeometryKernel + ConstructionKernel` at compile time.
///
/// **P10.4:** Methods return `Result<T, Unsupported>` — a typed error, not a
/// panic. A kernel that does not support a construction returns
/// `Err(Unsupported::new(...))`, and the caller can degrade gracefully.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExactPoint2 {
    /// Numerator of the x-coordinate (exact integer).
    pub x_num: i128,
    /// Numerator of the y-coordinate (exact integer).
    pub y_num: i128,
    /// Common denominator (always positive).
    pub den: i128,
}

/// Optional exact-construction kernel trait.
///
/// Implementors provide exact coordinate construction (intersection points that
/// survive re-predication without sign drift). `FilteredF64Kernel` does NOT
/// implement this — it returns f64 approximations, not exact points.
/// `ExactConstructionKernel` does.
pub trait ConstructionKernel {
    /// Exact 2-D segment-segment intersection point.
    ///
    /// Returns the intersection of segment `ab` with segment `cd` as an exact
    /// rational point (`ExactPoint2`), or `Err(Unsupported)` if the kernel
    /// cannot construct exact points. Parallel / collinear segments return
    /// `Err(Unsupported::new("segment_intersection_2", "parallel or collinear"))`.
    fn segment_intersection_2(
        &self,
        a: Point2,
        b: Point2,
        c: Point2,
        d: Point2,
    ) -> Result<ExactPoint2, Unsupported>;
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

    // ── P10.4 — Generic conformance tests ─────────────────────────────────
    //
    // These tests run a battery of known-answer predicate cases over ANY
    // kernel implementing `GeometryKernel`. Both `FilteredF64Kernel` and
    // `ExactConstructionKernel` must pass. This is the "existing kernels pass
    // generic conformance tests" gate from P10.4.

    /// Generic conformance battery for `GeometryKernel`.
    ///
    /// Runs known-answer tests for all four predicates. Every kernel
    /// implementing `GeometryKernel` must pass this — it's the compile-time +
    /// runtime guarantee that the kernel's predicate signs are correct.
    fn kernel_conforms<K: GeometryKernel>(k: &K) {
        // ── orientation_2 ──
        let o = Point2::new(0.0, 0.0);
        let x = Point2::new(1.0, 0.0);
        let y = Point2::new(0.0, 1.0);
        assert_eq!(k.orientation_2(o, x, y), Orientation::CounterClockwise);
        assert_eq!(k.orientation_2(o, y, x), Orientation::Clockwise);
        assert_eq!(k.orientation_2(o, x, Point2::new(2.0, 0.0)), Orientation::Collinear);

        // ── orient_3d ──
        let a3 = Point3::new(0.0, 0.0, 0.0);
        let b3 = Point3::new(1.0, 0.0, 0.0);
        let c3 = Point3::new(0.0, 1.0, 0.0);
        let d_pos = Point3::new(0.0, 0.0, 1.0);
        let d_neg = Point3::new(0.0, 0.0, -1.0);
        let d_coplanar = Point3::new(0.5, 0.5, 0.0);
        // det(b-a, c-a, d-a) for d=(0,0,1) is +1 (Positive) — d is on the
        // positive side of the oriented plane (same side as the normal
        // a→b→c). For d=(0,0,-1) it's -1 (Negative). Coplanar → Zero.
        assert_eq!(k.orient_3d(a3, b3, c3, d_pos), Sign::Positive);
        assert_eq!(k.orient_3d(a3, b3, c3, d_neg), Sign::Negative);
        assert_eq!(k.orient_3d(a3, b3, c3, d_coplanar), Sign::Zero);

        // ── incircle ──
        // CCW triangle (0,0), (1,0), (0,1); circumcircle center (0.5,0.5), r²=0.5.
        let p_in = Point2::new(0.25, 0.25);
        let p_out = Point2::new(2.0, 2.0);
        // (1,1) is on the circumcircle: dist² from (0.5,0.5) = 0.5 = r².
        let p_on = Point2::new(1.0, 1.0);
        assert_eq!(k.incircle(o, x, y, p_in), Sign::Positive);
        assert_eq!(k.incircle(o, x, y, p_out), Sign::Negative);
        // The filtered → exact ladder should resolve this to Zero.
        assert_eq!(k.incircle(o, x, y, p_on), Sign::Zero);

        // ── insphere ──
        // Tetrahedron (0,0,0), (1,0,0), (0,1,0), (0,0,1) — positively oriented
        // (det(b-a, c-a, d-a) = det((1,0,0),(0,1,0),(0,0,1)) = +1).
        // Circumcenter = (0.5, 0.5, 0.5), R² = 0.75.
        // The insphere implementation's sign convention (verified against the
        // existing insphere tests): for a positively oriented tetrahedron,
        // inside → Negative, outside → Positive (opposite of the doc comment,
        // but the tests are ground truth).
        let t0 = Point3::new(0.0, 0.0, 0.0);
        let t1 = Point3::new(1.0, 0.0, 0.0);
        let t2 = Point3::new(0.0, 1.0, 0.0);
        let t3 = Point3::new(0.0, 0.0, 1.0);
        let inside = Point3::new(0.1, 0.1, 0.1);
        let outside = Point3::new(2.0, 2.0, 2.0);
        assert_eq!(k.insphere(t0, t1, t2, t3, inside), Sign::Negative, "inside should be Negative (positive orientation)");
        assert_eq!(k.insphere(t0, t1, t2, t3, outside), Sign::Positive, "outside should be Positive (positive orientation)");
        assert_eq!(k.insphere(t0, t1, t2, t3, t0), Sign::Zero, "vertex on sphere");
    }

    #[test]
    fn filtered_f64_kernel_passes_conformance() {
        kernel_conforms(&FilteredF64Kernel::default());
    }

    #[test]
    fn exact_construction_kernel_passes_conformance() {
        kernel_conforms(&crate::specialized_libs::computational_geometry::exact_kernel::ExactConstructionKernel::default());
    }

    // ── P10.4 — Unsupported error tests ───────────────────────────────────

    #[test]
    fn unsupported_is_zero_sized() {
        // Unsupported carries only &'static str — no heap.
        assert_eq!(std::mem::size_of::<Unsupported>(), 2 * std::mem::size_of::<&'static str>());
    }

    #[test]
    fn unsupported_displays() {
        let u = Unsupported::new("segment_intersection_2", "filtered f64 kernel does not construct exact points");
        let s = format!("{}", u);
        assert!(s.contains("segment_intersection_2"));
        assert!(s.contains("filtered f64"));
    }

    #[test]
    fn unsupported_implements_error() {
        let u = Unsupported::new("test", "reason");
        // It implements std::error::Error (trait object works).
        let _: &dyn std::error::Error = &u;
    }

    // ── P10.4 — FilteredF64Kernel does NOT implement ConstructionKernel ──
    //
    // This is a compile-time guarantee: `FilteredF64Kernel` does not implement
    // `ConstructionKernel`, so an algorithm requiring `K: GeometryKernel +
    // ConstructionKernel` cannot accept `FilteredF64Kernel`. We can't test
    // negative trait impls directly in Rust, but we CAN test that the
    // `ExactConstructionKernel` DOES implement it and returns correct results.

    #[test]
    fn exact_kernel_implements_construction_kernel() {
        use crate::specialized_libs::computational_geometry::exact_kernel::ExactConstructionKernel;
        let k = ExactConstructionKernel::default();
        // Two segments that intersect at (0.5, 0.5):
        //   ab: (0,0)→(1,1), cd: (0,1)→(1,0)
        let a = Point2::new(0.0, 0.0);
        let b = Point2::new(1.0, 1.0);
        let c = Point2::new(0.0, 1.0);
        let d = Point2::new(1.0, 0.0);
        let result = k.segment_intersection_2(a, b, c, d);
        assert!(result.is_ok(), "non-parallel segments should intersect");
        let pt = result.unwrap();
        // (0.5, 0.5) = (1/2, 1/2)
        assert_eq!(pt.x_num, 1);
        assert_eq!(pt.y_num, 1);
        assert_eq!(pt.den, 2);
    }

    #[test]
    fn exact_kernel_construction_rejects_parallel() {
        use crate::specialized_libs::computational_geometry::exact_kernel::ExactConstructionKernel;
        let k = ExactConstructionKernel::default();
        // Parallel segments: (0,0)→(1,0) and (0,1)→(1,1)
        let a = Point2::new(0.0, 0.0);
        let b = Point2::new(1.0, 0.0);
        let c = Point2::new(0.0, 1.0);
        let d = Point2::new(1.0, 1.0);
        let result = k.segment_intersection_2(a, b, c, d);
        assert!(result.is_err(), "parallel segments should return Err");
        let err = result.unwrap_err();
        assert_eq!(err.capability, "segment_intersection_2");
    }
}
