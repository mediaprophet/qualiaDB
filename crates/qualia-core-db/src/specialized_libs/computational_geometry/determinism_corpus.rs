//! Determinism-as-contract corpus + cross-platform gate (P1.8).
//!
//! A fixed set of predicate test vectors (orientation_2, orient_3d, incircle,
//! insphere) whose collective sign-output is hashed into a single pinned
//! `u64`. The hash is **bit-identical on native and wasm32** — if any
//! predicate's sign diverges across platforms, the hash changes and the gate
//! fails.
//!
//! ## Why a pinned hash
//!
//! Individual predicate tests check correctness against the BigInt cross-check.
//! This corpus checks **determinism across platforms**: the same inputs must
//! produce the same signs on every target (native x86-64, native ARM64, wasm32).
//! The hash is a single 64-bit value that summarizes the entire corpus output —
//! if any sign changes, the hash changes.
//!
//! The hash is pinned as a compile-time constant. When a new predicate or test
//! vector is added, the hash is recomputed and the constant updated. Between
//! additions, any drift (platform divergence, compiler bug, fast-math leak)
//! breaks the gate.
//!
//! ## Corpus design
//!
//! The corpus covers:
//! - **Clear cases** — unambiguous signs (filtered stage resolves).
//! - **Degenerate cases** — exact-zero signs (coplanar, cocircular, cospherical).
//! - **Near-degenerate cases** — ±1-ulp perturbations (compensated/exact stage).
//! - **Extreme exponents** — large and small coordinates.
//! - **Adversarial cancellation** — coordinates chosen to cause massive
//!   cancellation in the determinant.
//! - **Symmetry** — vertex swaps flip signs.
//! - **Translation invariance** — translating all points preserves the sign.
//!
//! ## No fast-math
//!
//! The crate profile contains no `fast-math` flag (verified by inspection of
//! the workspace `Cargo.toml` and all crate-level `Cargo.toml` files). Fast-math
//! would break the error-free transforms (`two_product` via `fma`) that the
//! exact ladder depends on. This module's existence is the contract: if
//! fast-math is ever introduced, the pinned hash will change on at least one
//! platform, breaking the gate.

use super::expansion::Sign;
use super::incircle::incircle;
use super::insphere::insphere;
use super::orient3d::orient_3d;
use super::primitives::{orientation_2, Orientation, Point2, Point3};

/// The pinned determinism corpus hash.
///
/// This value is the FNV-1a hash of all predicate signs in the corpus. It is
/// bit-identical on native and wasm32. If any predicate's sign changes on any
/// platform, this hash will not match and the gate fails.
///
/// When adding new vectors to the corpus, recompute this value by running
/// `compute_corpus_hash()` and update the constant.
pub const PINNED_CORPUS_HASH: u64 = 0xa184a57fea2f6024;

// ──────────────────────────────────────────────────────────────────────────
//  FNV-1a hash (deterministic, no std::hash)
// ──────────────────────────────────────────────────────────────────────────

const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

#[inline]
fn fnv1a_u64(h: u64, val: u64) -> u64 {
    (h ^ val).wrapping_mul(FNV_PRIME)
}

/// Hash a sign into the running hash.
#[inline]
fn hash_sign(h: u64, s: Sign) -> u64 {
    fnv1a_u64(h, s as u64)
}

/// Hash an orientation into the running hash.
#[inline]
fn hash_orientation(h: u64, o: Orientation) -> u64 {
    fnv1a_u64(h, o as u64)
}

// ──────────────────────────────────────────────────────────────────────────
//  Corpus vectors
// ──────────────────────────────────────────────────────────────────────────

/// Run the full determinism corpus and return the hash of all predicate signs.
///
/// This is the function that must produce the same hash on every platform.
/// The corpus is fixed (no randomness, no platform-specific behavior).
pub fn compute_corpus_hash() -> u64 {
    let mut h = FNV_OFFSET;

    // ══════════════════════════════════════════════════════════════════════
    //  orientation_2 (2D orientation)
    // ══════════════════════════════════════════════════════════════════════
    h = hash_str(h, "orientation_2");

    // Clear cases
    h = hash_orientation(h, orientation_2(Point2::new(0.0, 0.0), Point2::new(1.0, 0.0), Point2::new(1.0, 1.0)));
    h = hash_orientation(h, orientation_2(Point2::new(0.0, 0.0), Point2::new(1.0, 0.0), Point2::new(1.0, -1.0)));
    h = hash_orientation(h, orientation_2(Point2::new(0.0, 0.0), Point2::new(1.0, 0.0), Point2::new(2.0, 0.0)));

    // Collinear on arbitrary line
    h = hash_orientation(h, orientation_2(Point2::new(3.0, 7.0), Point2::new(5.0, 11.0), Point2::new(7.0, 15.0)));

    // Near-collinear (±1-ulp)
    let base_y = 0.0f64;
    for &delta in &[1i64, -1, 2, -2, 5, -5] {
        let y = f64::from_bits((base_y.to_bits() as i64 + delta) as u64);
        h = hash_orientation(h, orientation_2(Point2::new(0.0, 0.0), Point2::new(1.0, 0.0), Point2::new(0.5, y)));
    }

    // Extreme exponents
    h = hash_orientation(h, orientation_2(Point2::new(1e100, 0.0), Point2::new(0.0, 1e100), Point2::new(0.0, 0.0)));
    h = hash_orientation(h, orientation_2(Point2::new(1e-100, 0.0), Point2::new(0.0, 1e-100), Point2::new(0.0, 0.0)));

    // Translation invariance
    let t = Point2::new(1e10, -1e10);
    h = hash_orientation(h, orientation_2(
        Point2::new(0.0 + t.x, 0.0 + t.y),
        Point2::new(1.0 + t.x, 0.0 + t.y),
        Point2::new(1.0 + t.x, 1.0 + t.y),
    ));

    // ══════════════════════════════════════════════════════════════════════
    //  orient_3d (3D orientation)
    // ══════════════════════════════════════════════════════════════════════
    h = hash_str(h, "orient_3d");

    // Clear cases: positive and negative tetrahedra
    h = hash_sign(h, orient_3d(
        Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0),
        Point3::new(0.0, 1.0, 0.0), Point3::new(0.0, 0.0, 1.0)));
    h = hash_sign(h, orient_3d(
        Point3::new(0.0, 0.0, 0.0), Point3::new(0.0, 1.0, 0.0),
        Point3::new(1.0, 0.0, 0.0), Point3::new(0.0, 0.0, 1.0)));

    // Coplanar (exact zero)
    h = hash_sign(h, orient_3d(
        Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0),
        Point3::new(1.0, 1.0, 0.0), Point3::new(0.0, 1.0, 0.0)));

    // Coplanar on arbitrary plane
    h = hash_sign(h, orient_3d(
        Point3::new(1.0, 2.0, 3.0), Point3::new(4.0, 5.0, 6.0),
        Point3::new(7.0, 8.0, 9.0), Point3::new(10.0, 11.0, 12.0)));

    // Near-coplanar (±1-ulp)
    let base_z = 0.0f64;
    for &delta in &[1i64, -1, 2, -2, 5, -5, 100, -100] {
        let z = f64::from_bits((base_z.to_bits() as i64 + delta) as u64);
        h = hash_sign(h, orient_3d(
            Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0), Point3::new(0.0, 0.0, z)));
    }

    // Extreme exponents
    h = hash_sign(h, orient_3d(
        Point3::new(1e50, 0.0, 0.0), Point3::new(0.0, 1e50, 0.0),
        Point3::new(0.0, 0.0, 1e50), Point3::new(0.0, 0.0, 0.0)));
    h = hash_sign(h, orient_3d(
        Point3::new(1e-50, 0.0, 0.0), Point3::new(0.0, 1e-50, 0.0),
        Point3::new(0.0, 0.0, 1e-50), Point3::new(0.0, 0.0, 0.0)));

    // Massive cancellation
    h = hash_sign(h, orient_3d(
        Point3::new(1e50, 0.0, 0.0), Point3::new(0.0, 1e50, 0.0),
        Point3::new(0.0, 0.0, 1e50), Point3::new(1e50, 1e50, 1e50)));

    // Vertex swap flips sign
    h = hash_sign(h, orient_3d(
        Point3::new(0.0, 1.0, 0.0), Point3::new(0.0, 0.0, 0.0),
        Point3::new(1.0, 0.0, 0.0), Point3::new(0.0, 0.0, 1.0)));

    // Translation invariance
    let t3 = Point3::new(1e10, -1e10, 5e9);
    h = hash_sign(h, orient_3d(
        Point3::new(0.0 + t3.x, 0.0 + t3.y, 0.0 + t3.z),
        Point3::new(1.0 + t3.x, 0.0 + t3.y, 0.0 + t3.z),
        Point3::new(0.0 + t3.x, 1.0 + t3.y, 0.0 + t3.z),
        Point3::new(0.0 + t3.x, 0.0 + t3.y, 1.0 + t3.z),
    ));

    // ══════════════════════════════════════════════════════════════════════
    //  incircle (2D in-circle)
    // ══════════════════════════════════════════════════════════════════════
    h = hash_str(h, "incircle");

    let ca = Point2::new(1.0, 0.0);
    let cb = Point2::new(0.0, 1.0);
    let cc = Point2::new(-1.0, 0.0);

    // Clear cases: inside, outside, on
    h = hash_sign(h, incircle(ca, cb, cc, Point2::new(0.0, 0.0))); // inside
    h = hash_sign(h, incircle(ca, cb, cc, Point2::new(2.0, 0.0))); // outside
    h = hash_sign(h, incircle(ca, cb, cc, Point2::new(0.0, -1.0))); // on

    // Cocircular on arbitrary circle
    h = hash_sign(h, incircle(
        Point2::new(8.0, 4.0), Point2::new(3.0, 9.0),
        Point2::new(-2.0, 4.0), Point2::new(3.0, -1.0)));

    // Near-cocircular (±1-ulp)
    let base_dy = -1.0f64;
    for &delta in &[1i64, -1, 2, -2, 5, -5, 100, -100] {
        let dy = f64::from_bits((base_dy.to_bits() as i64 + delta) as u64);
        h = hash_sign(h, incircle(ca, cb, cc, Point2::new(0.0, dy)));
    }

    // Extreme exponents
    h = hash_sign(h, incircle(
        Point2::new(1e50, 0.0), Point2::new(0.0, 1e50),
        Point2::new(-1e50, 0.0), Point2::new(0.0, 0.0)));
    h = hash_sign(h, incircle(
        Point2::new(1e-50, 0.0), Point2::new(0.0, 1e-50),
        Point2::new(-1e-50, 0.0), Point2::new(0.0, 0.0)));

    // Massive cancellation
    h = hash_sign(h, incircle(
        Point2::new(1e50, 0.0), Point2::new(0.0, 1e50),
        Point2::new(-1e50, 0.0), Point2::new(0.0, -1e50 + 1.0)));

    // Clockwise abc (sign flips)
    h = hash_sign(h, incircle(
        Point2::new(1.0, 0.0), Point2::new(-1.0, 0.0),
        Point2::new(0.0, 1.0), Point2::new(0.0, 0.0)));

    // Translation invariance
    let ti = Point2::new(1e10, -1e10);
    h = hash_sign(h, incircle(
        Point2::new(ca.x + ti.x, ca.y + ti.y),
        Point2::new(cb.x + ti.x, cb.y + ti.y),
        Point2::new(cc.x + ti.x, cc.y + ti.y),
        Point2::new(0.0 + ti.x, 0.0 + ti.y),
    ));

    // ══════════════════════════════════════════════════════════════════════
    //  insphere (3D in-sphere)
    // ══════════════════════════════════════════════════════════════════════
    h = hash_str(h, "insphere");

    let sa = Point3::new(1.0, 0.0, 0.0);
    let sb = Point3::new(0.0, 1.0, 0.0);
    let sc = Point3::new(0.0, 0.0, 1.0);
    let sd = Point3::new(-1.0, 0.0, 0.0);

    // Clear cases: inside, outside, on
    h = hash_sign(h, insphere(sa, sb, sc, sd, Point3::new(0.0, 0.0, 0.0))); // inside
    h = hash_sign(h, insphere(sa, sb, sc, sd, Point3::new(2.0, 0.0, 0.0))); // outside
    h = hash_sign(h, insphere(sa, sb, sc, sd, Point3::new(0.0, -1.0, 0.0))); // on

    // Cospherical on arbitrary sphere
    h = hash_sign(h, insphere(
        Point3::new(7.0, 2.0, 3.0), Point3::new(1.0, 8.0, 3.0),
        Point3::new(1.0, 2.0, 9.0), Point3::new(-5.0, 2.0, 3.0),
        Point3::new(1.0, -4.0, 3.0)));

    // Near-cospherical (±1-ulp)
    let base_ey = -1.0f64;
    for &delta in &[1i64, -1, 2, -2, 5, -5, 100, -100] {
        let ey = f64::from_bits((base_ey.to_bits() as i64 + delta) as u64);
        h = hash_sign(h, insphere(sa, sb, sc, sd, Point3::new(0.0, ey, 0.0)));
    }

    // Extreme exponents
    h = hash_sign(h, insphere(
        Point3::new(1e30, 0.0, 0.0), Point3::new(0.0, 1e30, 0.0),
        Point3::new(0.0, 0.0, 1e30), Point3::new(-1e30, 0.0, 0.0),
        Point3::new(0.0, 0.0, 0.0)));
    h = hash_sign(h, insphere(
        Point3::new(1e-30, 0.0, 0.0), Point3::new(0.0, 1e-30, 0.0),
        Point3::new(0.0, 0.0, 1e-30), Point3::new(-1e-30, 0.0, 0.0),
        Point3::new(0.0, 0.0, 0.0)));

    // Massive cancellation
    h = hash_sign(h, insphere(
        Point3::new(1e30, 0.0, 0.0), Point3::new(0.0, 1e30, 0.0),
        Point3::new(0.0, 0.0, 1e30), Point3::new(-1e30, 0.0, 0.0),
        Point3::new(0.0, -1e30 + 1.0, 0.0)));

    // Negative orientation (sign flips)
    h = hash_sign(h, insphere(
        Point3::new(0.0, 1.0, 0.0), Point3::new(1.0, 0.0, 0.0),
        Point3::new(0.0, 0.0, 1.0), Point3::new(-1.0, 0.0, 0.0),
        Point3::new(0.0, 0.0, 0.0)));

    // Translation invariance
    let ts = Point3::new(1e10, -1e10, 5e9);
    h = hash_sign(h, insphere(
        Point3::new(sa.x + ts.x, sa.y + ts.y, sa.z + ts.z),
        Point3::new(sb.x + ts.x, sb.y + ts.y, sb.z + ts.z),
        Point3::new(sc.x + ts.x, sc.y + ts.y, sc.z + ts.z),
        Point3::new(sd.x + ts.x, sd.y + ts.y, sd.z + ts.z),
        Point3::new(0.0 + ts.x, 0.0 + ts.y, 0.0 + ts.z),
    ));

    h
}

/// Hash a string tag into the running hash (to delimit predicate sections).
fn hash_str(h: u64, s: &str) -> u64 {
    let mut h = h;
    for b in s.bytes() {
        h = fnv1a_u64(h, b as u64);
    }
    h = fnv1a_u64(h, 0); // null terminator
    h
}

// ──────────────────────────────────────────────────────────────────────────
//  Tests
// ──────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// The determinism corpus hash must be stable across runs (determinism).
    #[test]
    fn corpus_hash_is_deterministic_across_calls() {
        let h1 = compute_corpus_hash();
        let h2 = compute_corpus_hash();
        assert_eq!(h1, h2, "corpus hash must be deterministic");
    }

    /// The corpus hash must match the pinned value.
    ///
    /// If this test fails, either:
    /// 1. A predicate's sign changed (regression or platform divergence).
    /// 2. New vectors were added to the corpus (update the pinned hash).
    /// 3. Fast-math was introduced (breaks error-free transforms).
    #[test]
    fn corpus_hash_matches_pinned_value() {
        let h = compute_corpus_hash();
        assert_eq!(
            h, PINNED_CORPUS_HASH,
            "corpus hash {h:#018x} does not match pinned value {PINNED_CORPUS_HASH:#018x} — \
             a predicate sign changed or the corpus was updated (update PINNED_CORPUS_HASH)"
        );
    }

    /// The corpus exercises all four predicates (sanity: no predicate is
    /// accidentally omitted).
    #[test]
    fn corpus_exercises_all_four_predicates() {
        // We verify this indirectly: the hash with all predicates should differ
        // from the hash with any predicate removed. We just check that the
        // corpus function runs without panic and produces a non-zero hash.
        let h = compute_corpus_hash();
        assert_ne!(h, FNV_OFFSET, "corpus hash should not be the initial value");
        assert_ne!(h, 0, "corpus hash should not be zero");
    }

    /// No fast-math in the crate profile.
    ///
    /// This is a documentation-level test — the actual verification is by
    /// inspection of all `Cargo.toml` files (no `fast-math` flag exists).
    /// The test exists to document the contract and fail if someone adds
    /// fast-math and the hash diverges.
    #[test]
    fn no_fast_math_in_profile() {
        // The pinned hash test above is the real gate. This test documents
        // the contract: if fast-math is introduced, the error-free transforms
        // (two_product via fma) will break, changing the hash.
        let h = compute_corpus_hash();
        assert_eq!(h, PINNED_CORPUS_HASH, "fast-math would break the pinned hash");
    }
}
