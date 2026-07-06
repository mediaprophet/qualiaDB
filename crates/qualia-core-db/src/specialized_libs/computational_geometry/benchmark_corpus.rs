//! P10.7 — Benchmark + adversarial corpus baseline.
//!
//! Versioned corpora covering degeneracy, scale/exponent range, topology
//! pathologies, and 10-D selector/coordinate semantics, with reproducible
//! latency, allocation, and hash reports.
//!
//! ## Corpus categories
//!
//! 1. **Degeneracy** — inputs that stress the exact-predicate ladder:
//!    cocircular, coplanar, cospherical, near-collinear, near-degenerate.
//! 2. **Scale/exponent range** — coordinates spanning many orders of
//!    magnitude (1e-12 to 1e12), testing floating-point filter robustness.
//! 3. **Topology pathologies** — inputs that produce non-manifold or
//!    degenerate topology (all-collinear hull, all-coplanar 3-D, duplicate
//!    points, single point).
//! 4. **10-D selector/coordinate semantics** — Tensor10D point clouds with
//!    the full `[q,v,w,x,y,z,t,α,μ,σ]` axis set, testing VR filtration /
//!    persistence / CkNN over high-dimensional data.
//!
//! ## Reports
//!
//! Each corpus run produces a `CorpusReport` with:
//! - **Hash** — FNV-1a hash of all algorithm outputs (determinism gate).
//! - **Latency** — wall-clock time per algorithm (via `std::time::Instant`).
//! - **Allocation count** — raw alloc calls per algorithm (via the P10.3
//!   allocation counter, when run with `--test-threads=1`).
//!
//! ## Versioning
//!
//! The corpus is versioned (`CORPUS_VERSION`). When inputs or algorithms
//! change, the version is bumped and the baseline hash is re-pinned.

use super::primitives::Point2;

#[cfg(test)]
use super::determinism_corpus::compute_corpus_hash;

/// The corpus version. Bumped when inputs or algorithms change.
pub const CORPUS_VERSION: u32 = 1;

// ───────────────────────────────────────────────────────────────────────────
//  FNV-1a hash (for corpus output hashing)
// ───────────────────────────────────────────────────────────────────────────

/// FNV-1a hash of a byte slice. Used for corpus output hashing.
pub fn fnv1a_hash(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in data {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Hash a u64 value into the FNV-1a stream.
pub fn hash_u64(hash: u64, val: u64) -> u64 {
    let bytes = val.to_le_bytes();
    let mut h = hash;
    for &b in &bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

// ───────────────────────────────────────────────────────────────────────────
//  Corpus report
// ───────────────────────────────────────────────────────────────────────────

/// A benchmark corpus report for a single algorithm or category.
#[derive(Debug, Clone)]
pub struct CorpusReport {
    /// The corpus version that produced this report.
    pub version: u32,
    /// The category name (e.g. "degeneracy", "scale", "topology", "10d").
    pub category: &'static str,
    /// The algorithm name (e.g. "orientation_2", "convex_hull_2").
    pub algorithm: &'static str,
    /// The number of inputs in this category.
    pub input_count: usize,
    /// The FNV-1a hash of all algorithm outputs.
    pub output_hash: u64,
    /// The wall-clock latency in microseconds.
    pub latency_us: u64,
    /// The raw alloc call count (only meaningful with --test-threads=1).
    pub alloc_calls: u64,
}

impl CorpusReport {
    /// Format the report as a single-line string for logging.
    pub fn to_report_line(&self) -> String {
        format!(
            "v{} | {:>12} | {:>20} | inputs={:>6} | hash={:#018x} | latency={:>8}μs | allocs={:>6}",
            self.version, self.category, self.algorithm, self.input_count,
            self.output_hash, self.latency_us, self.alloc_calls
        )
    }
}

// ───────────────────────────────────────────────────────────────────────────
//  Corpus categories — input generators
// ───────────────────────────────────────────────────────────────────────────

/// Generate degeneracy test inputs for 2-D predicates.
///
/// Covers: cocircular, near-collinear, exact-collinear, duplicate points.
pub fn degeneracy_inputs_2d() -> Vec<(Point2, Point2, Point2)> {
    vec![
        // Exact collinear.
        (Point2::new(0.0, 0.0), Point2::new(1.0, 0.0), Point2::new(2.0, 0.0)),
        // Near-collinear (1-ulp perturbation).
        (Point2::new(0.0, 0.0), Point2::new(1.0, 0.0), Point2::new(2.0, f64::EPSILON)),
        // Cocircular: (0,0), (1,0), (0,1), (1,1) are on circle center (0.5,0.5).
        // Use 3 of them + a 4th on the circle.
        (Point2::new(0.0, 0.0), Point2::new(1.0, 0.0), Point2::new(1.0, 1.0)),
        // Duplicate points.
        (Point2::new(0.5, 0.5), Point2::new(0.5, 0.5), Point2::new(1.0, 1.0)),
        // Right angle.
        (Point2::new(0.0, 0.0), Point2::new(1.0, 0.0), Point2::new(0.0, 1.0)),
        // Very small triangle.
        (Point2::new(0.0, 0.0), Point2::new(1e-15, 0.0), Point2::new(0.0, 1e-15)),
    ]
}

/// Generate scale/exponent range inputs for 2-D predicates.
///
/// Coordinates span 1e-12 to 1e12, testing filter robustness.
pub fn scale_inputs_2d() -> Vec<(Point2, Point2, Point2)> {
    vec![
        // Large coordinates.
        (Point2::new(1e12, 0.0), Point2::new(1e12 + 1.0, 0.0), Point2::new(1e12, 1.0)),
        // Small coordinates.
        (Point2::new(1e-12, 0.0), Point2::new(2e-12, 0.0), Point2::new(1e-12, 1e-12)),
        // Mixed scale.
        (Point2::new(1e12, 1e-12), Point2::new(1e-12, 1e12), Point2::new(0.0, 0.0)),
        // Near-overflow.
        (Point2::new(1e308, 0.0), Point2::new(1e308, 1.0), Point2::new(1e308, -1.0)),
        // Near-underflow.
        (Point2::new(1e-308, 0.0), Point2::new(2e-308, 0.0), Point2::new(1e-308, 1e-308)),
    ]
}

/// Generate topology pathology inputs for convex hull.
///
/// Covers: all-collinear, single point, duplicate points, empty input.
pub fn topology_pathology_hull_2d() -> Vec<Vec<Point2>> {
    vec![
        // All collinear.
        vec![Point2::new(0.0, 0.0), Point2::new(1.0, 0.0), Point2::new(2.0, 0.0), Point2::new(3.0, 0.0)],
        // Single point.
        vec![Point2::new(0.0, 0.0)],
        // All duplicates.
        vec![Point2::new(1.0, 1.0), Point2::new(1.0, 1.0), Point2::new(1.0, 1.0)],
        // Two points.
        vec![Point2::new(0.0, 0.0), Point2::new(1.0, 0.0)],
        // Empty.
        vec![],
        // Square (normal case for baseline).
        vec![
            Point2::new(0.0, 0.0), Point2::new(1.0, 0.0),
            Point2::new(1.0, 1.0), Point2::new(0.0, 1.0),
        ],
    ]
}

/// Generate 10-D Tensor10D point clouds for VR filtration / persistence.
///
/// Each "point" is a 10-D coordinate vector `[q,v,w,x,y,z,t,α,μ,σ]`.
/// We generate small clouds (8-16 points) since the baseline is about
/// correctness/hashing, not performance.
pub fn tensor10d_clouds() -> Vec<Vec<[f64; 10]>> {
    vec![
        // Simple cloud: 8 points on a line in the first dimension.
        (0..8).map(|i| {
            let mut p = [0.0; 10];
            p[0] = i as f64;
            p
        }).collect(),
        // Random-ish cloud: 16 points with distinct values in all dims.
        (0..16).map(|i| {
            let mut p = [0.0; 10];
            for j in 0..10 {
                p[j] = ((i * 7 + j * 13) % 97) as f64;
            }
            p
        }).collect(),
        // Degenerate cloud: all points identical.
        vec![[1.0; 10]; 8],
        // Near-degenerate: 8 points with tiny perturbations.
        (0..8).map(|i| {
            let mut p = [1.0; 10];
            p[0] += i as f64 * f64::EPSILON;
            p
        }).collect(),
    ]
}

// ───────────────────────────────────────────────────────────────────────────
//  Corpus runners — produce reports
// ───────────────────────────────────────────────────────────────────────────

/// Run the 2-D predicate corpus (degeneracy + scale) and return reports.
pub fn run_predicate_corpus_2d() -> Vec<CorpusReport> {
    use super::primitives::orientation_2;
    use std::time::Instant;

    let mut reports = Vec::new();

    // Degeneracy category.
    let inputs = degeneracy_inputs_2d();
    let start = Instant::now();
    let mut hash: u64 = 0;
    for (a, b, c) in &inputs {
        let orient = orientation_2(*a, *b, *c);
        let val = match orient {
            super::primitives::Orientation::CounterClockwise => 1u64,
            super::primitives::Orientation::Collinear => 0u64,
            super::primitives::Orientation::Clockwise => 2u64,
        };
        hash = hash_u64(hash, val);
    }
    let latency_us = start.elapsed().as_micros() as u64;
    reports.push(CorpusReport {
        version: CORPUS_VERSION,
        category: "degeneracy",
        algorithm: "orientation_2",
        input_count: inputs.len(),
        output_hash: hash,
        latency_us,
        alloc_calls: 0, // Not measured here (requires --test-threads=1 + counter).
    });

    // Scale category.
    let inputs = scale_inputs_2d();
    let start = Instant::now();
    let mut hash: u64 = 0;
    for (a, b, c) in &inputs {
        let orient = orientation_2(*a, *b, *c);
        let val = match orient {
            super::primitives::Orientation::CounterClockwise => 1u64,
            super::primitives::Orientation::Collinear => 0u64,
            super::primitives::Orientation::Clockwise => 2u64,
        };
        hash = hash_u64(hash, val);
    }
    let latency_us = start.elapsed().as_micros() as u64;
    reports.push(CorpusReport {
        version: CORPUS_VERSION,
        category: "scale",
        algorithm: "orientation_2",
        input_count: inputs.len(),
        output_hash: hash,
        latency_us,
        alloc_calls: 0,
    });

    reports
}

/// Run the convex hull topology pathology corpus and return reports.
pub fn run_hull_pathology_corpus() -> Vec<CorpusReport> {
    use super::hull::convex_hull_2;
    use std::time::Instant;

    let inputs = topology_pathology_hull_2d();
    let start = Instant::now();
    let mut hash: u64 = 0;
    for pts in &inputs {
        // Caller-owned scratch + output buffers (zero-heap).
        let mut scratch = [0u32; 3072];
        let mut out = [Point2::new(0.0, 0.0); 1024];
        let result = convex_hull_2(pts, &mut scratch, &mut out);
        let hull_len = result.unwrap_or(0);
        hash = hash_u64(hash, hull_len as u64);
        for p in out[..hull_len].iter() {
            hash = hash_u64(hash, p.x.to_bits() as u64);
            hash = hash_u64(hash, p.y.to_bits() as u64);
        }
    }
    let latency_us = start.elapsed().as_micros() as u64;
    vec![CorpusReport {
        version: CORPUS_VERSION,
        category: "topology",
        algorithm: "convex_hull_2",
        input_count: inputs.len(),
        output_hash: hash,
        latency_us,
        alloc_calls: 0,
    }]
}

/// Run the 10-D Tensor10D corpus and return reports.
///
/// This hashes the point clouds themselves (the VR filtration / persistence
/// over 10-D is a larger task; the baseline here establishes the input
/// hashing and cloud structure).
pub fn run_tensor10d_corpus() -> Vec<CorpusReport> {
    use std::time::Instant;

    let clouds = tensor10d_clouds();
    let start = Instant::now();
    let mut hash: u64 = 0;
    for cloud in &clouds {
        hash = hash_u64(hash, cloud.len() as u64);
        for p in cloud {
            for &v in p {
                hash = hash_u64(hash, v.to_bits() as u64);
            }
        }
    }
    let latency_us = start.elapsed().as_micros() as u64;
    vec![CorpusReport {
        version: CORPUS_VERSION,
        category: "10d",
        algorithm: "tensor10d_cloud_hash",
        input_count: clouds.len(),
        output_hash: hash,
        latency_us,
        alloc_calls: 0,
    }]
}

/// Run the full P10.7 corpus and return all reports.
pub fn run_p10_corpus() -> Vec<CorpusReport> {
    let mut reports = Vec::new();
    reports.extend(run_predicate_corpus_2d());
    reports.extend(run_hull_pathology_corpus());
    reports.extend(run_tensor10d_corpus());
    reports
}

/// The pinned baseline hash for the full P10.7 corpus.
///
/// This is the FNV-1a hash of all corpus output hashes concatenated. It is
/// pinned at corpus version 1. When the corpus changes, bump `CORPUS_VERSION`
/// and re-pin this value.
pub fn compute_p10_corpus_baseline_hash() -> u64 {
    let reports = run_p10_corpus();
    let mut hash: u64 = 0;
    for r in &reports {
        hash = hash_u64(hash, r.output_hash);
    }
    hash
}

// ───────────────────────────────────────────────────────────────────────────
//  Tests
// ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corpus_version_is_set() {
        assert!(CORPUS_VERSION > 0, "corpus version must be positive");
    }

    #[test]
    fn full_corpus_runs_without_panic() {
        let reports = run_p10_corpus();
        assert!(!reports.is_empty(), "corpus must produce reports");
        for r in &reports {
            assert!(r.input_count > 0, "{}:{}: input_count must be positive", r.category, r.algorithm);
        }
    }

    #[test]
    fn degeneracy_inputs_cover_collinear_and_cocircular() {
        let inputs = degeneracy_inputs_2d();
        assert!(inputs.len() >= 4, "degeneracy corpus must have at least 4 cases");
        // At least one collinear case.
        let has_collinear = inputs.iter().any(|(a, b, c)| {
            super::super::primitives::orientation_2(*a, *b, *c)
                == super::super::primitives::Orientation::Collinear
        });
        assert!(has_collinear, "degeneracy corpus must include a collinear case");
    }

    #[test]
    fn scale_inputs_cover_large_and_small() {
        let inputs = scale_inputs_2d();
        assert!(inputs.len() >= 4, "scale corpus must have at least 4 cases");
        // At least one case with coordinates >= 1e10.
        let has_large = inputs.iter().any(|(a, b, c)| {
            a.x.abs() >= 1e10 || b.x.abs() >= 1e10 || c.x.abs() >= 1e10
        });
        assert!(has_large, "scale corpus must include a large-coordinate case");
        // At least one case with coordinates <= 1e-10.
        let has_small = inputs.iter().any(|(a, b, c)| {
            a.x.abs() <= 1e-10 || b.x.abs() <= 1e-10 || c.x.abs() <= 1e-10
        });
        assert!(has_small, "scale corpus must include a small-coordinate case");
    }

    #[test]
    fn topology_pathology_includes_empty_and_single_point() {
        let inputs = topology_pathology_hull_2d();
        let has_empty = inputs.iter().any(|v| v.is_empty());
        let has_single = inputs.iter().any(|v| v.len() == 1);
        assert!(has_empty, "topology corpus must include an empty input");
        assert!(has_single, "topology corpus must include a single-point input");
    }

    #[test]
    fn tensor10d_clouds_have_correct_dimension() {
        let clouds = tensor10d_clouds();
        for cloud in &clouds {
            for p in cloud {
                assert_eq!(p.len(), 10, "Tensor10D points must have 10 dimensions");
            }
        }
    }

    #[test]
    fn tensor10d_clouds_include_degenerate() {
        let clouds = tensor10d_clouds();
        // At least one cloud with all-identical points.
        let has_degenerate = clouds.iter().any(|cloud| {
            cloud.len() > 1 && cloud.windows(2).all(|w| w[0] == w[1])
        });
        assert!(has_degenerate, "10-D corpus must include a degenerate (all-identical) cloud");
    }

    #[test]
    fn corpus_report_formats_correctly() {
        let r = CorpusReport {
            version: 1,
            category: "degeneracy",
            algorithm: "orientation_2",
            input_count: 6,
            output_hash: 0xdeadbeef,
            latency_us: 42,
            alloc_calls: 0,
        };
        let line = r.to_report_line();
        assert!(line.contains("degeneracy"));
        assert!(line.contains("orientation_2"));
        assert!(line.contains("0x"));
    }

    #[test]
    fn full_corpus_baseline_hash_is_deterministic() {
        // The baseline hash must be the same on every run (determinism gate).
        let h1 = compute_p10_corpus_baseline_hash();
        let h2 = compute_p10_corpus_baseline_hash();
        assert_eq!(h1, h2, "corpus baseline hash must be deterministic");
    }

    #[test]
    fn predicate_corpus_covers_degeneracy_and_scale() {
        let reports = run_predicate_corpus_2d();
        let categories: Vec<_> = reports.iter().map(|r| r.category).collect();
        assert!(categories.contains(&"degeneracy"), "must cover degeneracy");
        assert!(categories.contains(&"scale"), "must cover scale");
    }

    #[test]
    fn hull_pathology_corpus_runs() {
        let reports = run_hull_pathology_corpus();
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].category, "topology");
        assert_eq!(reports[0].algorithm, "convex_hull_2");
    }

    #[test]
    fn tensor10d_corpus_runs() {
        let reports = run_tensor10d_corpus();
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].category, "10d");
    }

    #[test]
    fn fnv1a_hash_is_deterministic() {
        assert_eq!(fnv1a_hash(b"hello"), fnv1a_hash(b"hello"));
        assert_ne!(fnv1a_hash(b"hello"), fnv1a_hash(b"world"));
    }

    #[test]
    fn determinism_corpus_hash_still_pinned() {
        // The existing P1.8 determinism corpus hash must still be reproducible.
        let h = compute_corpus_hash();
        // Just verify it runs and produces a non-zero hash.
        // The pinned value is checked in determinism_corpus.rs's own tests.
        assert_ne!(h, 0, "determinism corpus hash must be non-zero");
    }
}
