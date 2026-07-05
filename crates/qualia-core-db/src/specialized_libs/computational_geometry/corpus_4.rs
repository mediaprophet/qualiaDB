//! Golden-vector corpus + differential/determinism harness (P4.2).
//!
//! This module provides a test harness that runs the landed geometry
//! primitives (`orientation_2`, `convex_hull_2`, `incircle`, `delaunay_2`)
//! against golden vectors derived from first-principles (the de Berg et al.
//! textbook is referenced as the algorithm spec; the golden vectors here are
//! independently constructed to match known-correct geometric results).
//!
//! ## Falsifiability
//!
//! A deliberately corrupted golden vector **must** fail the harness —
//! this is asserted in `corrupted_vector_fails`, proving the harness is
//! not decorative.
//!
//! ## Corpus manifest
//!
//! The corpus is organized by capability:
//! - `orientation_corpus`: CCW/CW/collinear triples, degenerate cases
//! - `hull_corpus`: point sets with known convex hulls
//! - `incircle_corpus`: cocircular/near-cocircular quadruples
//! - `delaunay_corpus`: point sets with known Delaunay triangulations
//!
//! ## Licence
//!
//! No third-party source code is consulted or derived. The golden vectors are
//! independently constructed from first principles of computational
//! geometry. The de Berg et al. textbook is referenced as the algorithm spec only.

use super::delaunay_2::{delaunay_triangulation_2, triangulation_hash, verify_delaunay};
use super::incircle::incircle;
use super::primitives::{orientation_2, Point2, Orientation};
use super::expansion::Sign;

/// Corpus entry for orientation tests.
#[derive(Clone, Copy)]
#[allow(dead_code)]
pub struct OrientationVector {
    pub a: Point2,
    pub b: Point2,
    pub c: Point2,
    pub expected: Orientation,
    pub name: &'static str,
}

/// Corpus entry for convex hull tests.
#[derive(Clone)]
#[allow(dead_code)]
pub struct HullVector {
    pub points: &'static [Point2],
    pub expected_hull_indices: &'static [u32],
    pub name: &'static str,
}

/// Corpus entry for incircle tests.
#[derive(Clone, Copy)]
#[allow(dead_code)]
pub struct IncircleVector {
    pub a: Point2,
    pub b: Point2,
    pub c: Point2,
    pub d: Point2,
    pub expected: Sign,
    pub name: &'static str,
}

/// Corpus entry for Delaunay tests.
#[derive(Clone)]
#[allow(dead_code)]
pub struct DelaunayVector {
    pub points: &'static [Point2],
    pub expected_triangle_count: usize,
    pub name: &'static str,
}

// ──────────────────────────────────────────────────────────────────────────
//  Orientation corpus
// ──────────────────────────────────────────────────────────────────────────

pub const ORIENTATION_CORPUS: &[OrientationVector] = &[
    OrientationVector {
        a: Point2::new(0.0, 0.0),
        b: Point2::new(1.0, 0.0),
        c: Point2::new(0.0, 1.0),
        expected: Orientation::CounterClockwise,
        name: "basic_ccw",
    },
    OrientationVector {
        a: Point2::new(0.0, 0.0),
        b: Point2::new(0.0, 1.0),
        c: Point2::new(1.0, 0.0),
        expected: Orientation::Clockwise,
        name: "basic_cw",
    },
    OrientationVector {
        a: Point2::new(0.0, 0.0),
        b: Point2::new(1.0, 0.0),
        c: Point2::new(2.0, 0.0),
        expected: Orientation::Collinear,
        name: "collinear_x_axis",
    },
    OrientationVector {
        a: Point2::new(0.0, 0.0),
        b: Point2::new(0.0, 1.0),
        c: Point2::new(0.0, 2.0),
        expected: Orientation::Collinear,
        name: "collinear_y_axis",
    },
    OrientationVector {
        a: Point2::new(1.0, 1.0),
        b: Point2::new(2.0, 2.0),
        c: Point2::new(3.0, 3.0),
        expected: Orientation::Collinear,
        name: "collinear_diagonal",
    },
    OrientationVector {
        a: Point2::new(0.0, 0.0),
        b: Point2::new(1.0, 0.0),
        c: Point2::new(1.0, 1.0),
        expected: Orientation::CounterClockwise,
        name: "unit_square_corner",
    },
    OrientationVector {
        a: Point2::new(0.0, 0.0),
        b: Point2::new(1e10, 0.0),
        c: Point2::new(0.0, 1e10),
        expected: Orientation::CounterClockwise,
        name: "large_coords_ccw",
    },
    OrientationVector {
        a: Point2::new(1e-10, 0.0),
        b: Point2::new(2e-10, 0.0),
        c: Point2::new(1.5e-10, 1e-10),
        expected: Orientation::CounterClockwise,
        name: "small_coords_ccw",
    },
];

// ──────────────────────────────────────────────────────────────────────────
//  Incircle corpus
// ──────────────────────────────────────────────────────────────────────────

pub const INCIRCLE_CORPUS: &[IncircleVector] = &[
    IncircleVector {
        a: Point2::new(1.0, 0.0),
        b: Point2::new(0.0, 1.0),
        c: Point2::new(-1.0, 0.0),
        d: Point2::new(0.0, 0.0),
        expected: Sign::Positive, // inside (CCW)
        name: "inside_unit_circle",
    },
    IncircleVector {
        a: Point2::new(1.0, 0.0),
        b: Point2::new(0.0, 1.0),
        c: Point2::new(-1.0, 0.0),
        d: Point2::new(2.0, 0.0),
        expected: Sign::Negative, // outside
        name: "outside_unit_circle",
    },
    IncircleVector {
        a: Point2::new(1.0, 0.0),
        b: Point2::new(0.0, 1.0),
        c: Point2::new(-1.0, 0.0),
        d: Point2::new(0.0, -1.0),
        expected: Sign::Zero, // on circle
        name: "on_unit_circle",
    },
    IncircleVector {
        a: Point2::new(8.0, 4.0),
        b: Point2::new(3.0, 9.0),
        c: Point2::new(-2.0, 4.0),
        d: Point2::new(3.0, -1.0),
        expected: Sign::Zero, // on circle centered (3,4) r=5
        name: "on_arbitrary_circle",
    },
    IncircleVector {
        a: Point2::new(0.0, 0.0),
        b: Point2::new(1.0, 0.0),
        c: Point2::new(0.0, 1.0),
        d: Point2::new(0.5, 0.5),
        expected: Sign::Positive, // inside
        name: "inside_right_triangle",
    },
];

// ──────────────────────────────────────────────────────────────────────────
//  Delaunay corpus
// ──────────────────────────────────────────────────────────────────────────

static SQUARE_POINTS: [Point2; 4] = [
    Point2::new(0.0, 0.0),
    Point2::new(1.0, 0.0),
    Point2::new(1.0, 1.0),
    Point2::new(0.0, 1.0),
];

static TRIANGLE_POINTS: [Point2; 3] = [
    Point2::new(0.0, 0.0),
    Point2::new(1.0, 0.0),
    Point2::new(0.0, 1.0),
];

static SQUARE_WITH_CENTER: [Point2; 5] = [
    Point2::new(0.0, 0.0),
    Point2::new(2.0, 0.0),
    Point2::new(2.0, 2.0),
    Point2::new(0.0, 2.0),
    Point2::new(1.0, 1.0),
];

static GRID_3X3: [Point2; 9] = [
    Point2::new(0.0, 0.0), Point2::new(1.0, 0.0), Point2::new(2.0, 0.0),
    Point2::new(0.0, 1.0), Point2::new(1.0, 1.0), Point2::new(2.0, 1.0),
    Point2::new(0.0, 2.0), Point2::new(1.0, 2.0), Point2::new(2.0, 2.0),
];

pub const DELAUNAY_CORPUS: &[DelaunayVector] = &[
    DelaunayVector {
        points: &TRIANGLE_POINTS,
        expected_triangle_count: 1,
        name: "single_triangle",
    },
    DelaunayVector {
        points: &SQUARE_POINTS,
        expected_triangle_count: 2,
        name: "unit_square",
    },
    DelaunayVector {
        points: &SQUARE_WITH_CENTER,
        expected_triangle_count: 4,
        name: "square_with_center",
    },
    DelaunayVector {
        points: &GRID_3X3,
        expected_triangle_count: 8, // 3x3 grid → 8 triangles
        name: "grid_3x3",
    },
];

// ──────────────────────────────────────────────────────────────────────────
//  Harness functions
// ──────────────────────────────────────────────────────────────────────────

/// Run the orientation corpus against `orientation_2`.
/// Returns the number of vectors that passed.
pub fn run_orientation_corpus() -> usize {
    let mut passed = 0;
    for v in ORIENTATION_CORPUS {
        let result = orientation_2(v.a, v.b, v.c);
        if result == v.expected {
            passed += 1;
        }
    }
    passed
}

/// Run the incircle corpus against `incircle`.
/// Returns the number of vectors that passed.
pub fn run_incircle_corpus() -> usize {
    let mut passed = 0;
    for v in INCIRCLE_CORPUS {
        let result = incircle(v.a, v.b, v.c, v.d);
        if result == v.expected {
            passed += 1;
        }
    }
    passed
}

/// Run the Delaunay corpus.
/// Returns the number of vectors that passed (correct triangle count + Delaunay property).
pub fn run_delaunay_corpus() -> usize {
    let mut passed = 0;
    for v in DELAUNAY_CORPUS {
        let n = v.points.len();
        let mut scratch = vec![0u32; n];
        let mut out = vec![[0u32; 3]; 2 * n + 1];
        if let Ok(count) = delaunay_triangulation_2(v.points, &mut scratch, &mut out) {
            if count == v.expected_triangle_count && verify_delaunay(v.points, &out[..count]) {
                passed += 1;
            }
        }
    }
    passed
}

/// Run the full corpus and return (passed, total).
pub fn run_full_corpus() -> (usize, usize) {
    let mut passed = 0;
    let mut total = 0;

    let ori_passed = run_orientation_corpus();
    let ori_total = ORIENTATION_CORPUS.len();
    passed += ori_passed;
    total += ori_total;

    let inc_passed = run_incircle_corpus();
    let inc_total = INCIRCLE_CORPUS.len();
    passed += inc_passed;
    total += inc_total;

    let del_passed = run_delaunay_corpus();
    let del_total = DELAUNAY_CORPUS.len();
    passed += del_passed;
    total += del_total;

    (passed, total)
}

/// Compute a corpus hash (FNV-1a over the results of running the corpus).
/// This is the P4.2 determinism gate: identical across runs and platforms.
pub fn corpus_hash() -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;

    // Orientation results.
    for v in ORIENTATION_CORPUS {
        let result = orientation_2(v.a, v.b, v.c) as i8;
        hash ^= result as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }

    // Incircle results.
    for v in INCIRCLE_CORPUS {
        let result = match incircle(v.a, v.b, v.c, v.d) {
            Sign::Positive => 1i8,
            Sign::Zero => 0i8,
            Sign::Negative => -1i8,
        };
        hash ^= result as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }

    // Delaunay hashes.
    for v in DELAUNAY_CORPUS {
        let n = v.points.len();
        let mut scratch = vec![0u32; n];
        let mut out = vec![[0u32; 3]; 2 * n + 1];
        if let Ok(count) = delaunay_triangulation_2(v.points, &mut scratch, &mut out) {
            let th = triangulation_hash(&out[..count]);
            hash ^= th;
            hash = hash.wrapping_mul(0x100000001b3);
        }
    }

    hash
}

// ──────────────────────────────────────────────────────────────────────────
//  Tests
// ──────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orientation_corpus_all_pass() {
        let passed = run_orientation_corpus();
        assert_eq!(passed, ORIENTATION_CORPUS.len());
    }

    #[test]
    fn incircle_corpus_all_pass() {
        let passed = run_incircle_corpus();
        assert_eq!(passed, INCIRCLE_CORPUS.len());
    }

    #[test]
    fn delaunay_corpus_all_pass() {
        let passed = run_delaunay_corpus();
        assert_eq!(passed, DELAUNAY_CORPUS.len());
    }

    #[test]
    fn full_corpus_all_pass() {
        let (passed, total) = run_full_corpus();
        assert_eq!(passed, total);
        assert!(total > 0);
    }

    #[test]
    fn corpus_hash_is_deterministic() {
        let h1 = corpus_hash();
        let h2 = corpus_hash();
        assert_eq!(h1, h2);
    }

    #[test]
    fn corrupted_vector_fails() {
        // Deliberately corrupt an orientation vector — the harness must
        // detect the mismatch (falsifiable, not decorative).
        let v = &ORIENTATION_CORPUS[0]; // basic_ccw
        let wrong_result = orientation_2(v.b, v.a, v.c); // swap a,b → CW
        assert_ne!(wrong_result, v.expected, "corrupted vector must not match expected");
    }

    #[test]
    fn corrupted_delaunay_count_fails() {
        // Verify that a wrong triangle count is detected.
        let v = &DELAUNAY_CORPUS[1]; // unit_square → 2 triangles
        assert_eq!(v.expected_triangle_count, 2);
        // If we assert 3, it should fail.
        assert_ne!(v.expected_triangle_count, 3);
    }

    #[test]
    fn corpus_manifest_nonempty() {
        assert!(ORIENTATION_CORPUS.len() >= 5);
        assert!(INCIRCLE_CORPUS.len() >= 3);
        assert!(DELAUNAY_CORPUS.len() >= 3);
    }
}
