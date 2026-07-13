//! P8.7 — GPU acceleration + CPU oracle for the P8 distance/density/
//! circumradius batches (differential + determinism).
//!
//! This module provides:
//! 1. **CPU oracle**: batch distance computation, batch circumradius,
//!    batch density estimation — all deterministic reference implementations.
//! 2. **GPU kernel specs**: WGSL shader source for GPU acceleration.
//! 3. **Differential check**: compares CPU vs GPU output.
//!
//! ## Determinism
//!
//! The CPU oracle is fully deterministic. The GPU kernel must produce
//! output within tolerance of the CPU oracle.

use super::cknn_laplacian::local_density;
use super::nn_query::axis_honest_distance;
use super::vr_filtration::spatial_distance;
use crate::tensor::Tensor10D;

// ───────────────────────────────────────────────────────────────────────────
//  CPU oracle: batch distance computation
// ───────────────────────────────────────────────────────────────────────────

/// Compute pairwise distances between all points in a batch.
///
/// `points` is the point cloud. `out_distances` is a row-major n×n matrix
/// (n² entries). Returns the number of distances computed (n²).
pub fn cpu_batch_pairwise_distances(
    points: &[Tensor10D],
    out_distances: &mut [f64],
) -> Result<usize, GpuOracleError> {
    let n = points.len();
    if out_distances.len() < n * n {
        return Err(GpuOracleError::BufferTooSmall {
            needed: n * n,
            have: out_distances.len(),
        });
    }

    for i in 0..n {
        for j in 0..n {
            out_distances[i * n + j] = spatial_distance(&points[i], &points[j]);
        }
    }

    Ok(n * n)
}

/// Compute distances from a query point to all points in a batch.
///
/// `out_distances` needs `n` entries. Returns n.
pub fn cpu_batch_query_distances(
    points: &[Tensor10D],
    query: &Tensor10D,
    out_distances: &mut [f64],
) -> Result<usize, GpuOracleError> {
    let n = points.len();
    if out_distances.len() < n {
        return Err(GpuOracleError::BufferTooSmall {
            needed: n,
            have: out_distances.len(),
        });
    }

    for i in 0..n {
        out_distances[i] = axis_honest_distance(&points[i], query);
    }

    Ok(n)
}

// ───────────────────────────────────────────────────────────────────────────
//  CPU oracle: batch circumradius
// ───────────────────────────────────────────────────────────────────────────

/// Compute circumradius for a batch of triangles (specified as vertex index
/// triples into the point cloud).
///
/// `triangles` is a flat array of `[u32; 3]` vertex indices.
/// `out_radii` needs `triangles.len()` entries.
pub fn cpu_batch_circumradius(
    points: &[Tensor10D],
    triangles: &[[u32; 3]],
    out_radii: &mut [f64],
) -> Result<usize, GpuOracleError> {
    if out_radii.len() < triangles.len() {
        return Err(GpuOracleError::BufferTooSmall {
            needed: triangles.len(),
            have: out_radii.len(),
        });
    }

    for (i, tri) in triangles.iter().enumerate() {
        let a = &points[tri[0] as usize];
        let b = &points[tri[1] as usize];
        let c = &points[tri[2] as usize];

        let d_ab = spatial_distance(a, b);
        let d_bc = spatial_distance(b, c);
        let d_ca = spatial_distance(c, a);

        let s = (d_ab + d_bc + d_ca) / 2.0;
        let area_sq = s * (s - d_ab) * (s - d_bc) * (s - d_ca);
        if area_sq <= 0.0 {
            out_radii[i] = f64::INFINITY;
        } else {
            out_radii[i] = (d_ab * d_bc * d_ca) / (4.0 * area_sq.sqrt());
        }
    }

    Ok(triangles.len())
}

// ───────────────────────────────────────────────────────────────────────────
//  CPU oracle: batch density
// ───────────────────────────────────────────────────────────────────────────

/// Compute local density for a batch of points.
///
/// `out_density` needs `n` entries. Returns n.
pub fn cpu_batch_density(
    points: &[Tensor10D],
    k: usize,
    out_density: &mut [f64],
) -> Result<usize, GpuOracleError> {
    local_density(points, k, out_density).map_err(|e| match e {
        super::cknn_laplacian::CknnError::BufferTooSmall { needed, have } => {
            GpuOracleError::BufferTooSmall { needed, have }
        }
        super::cknn_laplacian::CknnError::TooFewPoints { got } => {
            GpuOracleError::TooFewPoints { got }
        }
        super::cknn_laplacian::CknnError::KTooLarge { k, n } => GpuOracleError::KTooLarge { k, n },
        super::cknn_laplacian::CknnError::NonFinite { point_index } => {
            GpuOracleError::NonFinite { point_index }
        }
    })?;
    Ok(points.len())
}

// ───────────────────────────────────────────────────────────────────────────
//  GPU kernel specifications (WGSL)
// ───────────────────────────────────────────────────────────────────────────

/// WGSL shader for batch pairwise distance computation.
///
/// Computes one row of the distance matrix per workgroup invocation.
pub const GPU_DISTANCE_KERNEL_WGSL: &str = r#"
// P8.7 — GPU batch pairwise distance kernel.
// Computes one row of the n×n distance matrix per invocation.

struct Points {
    data: array<f32>,  // n × 3 (x, y, z) packed
};

struct Distances {
    data: array<f32>,  // n × n distance matrix
};

struct Params {
    n: u32,
};

@group(0) @binding(0) var<storage, read> points: Points;
@group(0) @binding(1) var<storage, read_write> distances: Distances;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i >= params.n) {
        return;
    }

    let ai = i * 3u;
    let ax = points.data[ai];
    let ay = points.data[ai + 1u];
    let az = points.data[ai + 2u];

    for (var j = 0u; j < params.n; j = j + 1u) {
        let bj = j * 3u;
        let dx = ax - points.data[bj];
        let dy = ay - points.data[bj + 1u];
        let dz = az - points.data[bj + 2u];
        let d = sqrt(dx * dx + dy * dy + dz * dz);
        distances.data[i * params.n + j] = d;
    }
}
"#;

/// WGSL shader for batch circumradius computation.
pub const GPU_CIRCUMRADIUS_KERNEL_WGSL: &str = r#"
// P8.7 — GPU batch circumradius kernel.
// Computes circumradius for one triangle per invocation.

struct Points {
    data: array<f32>,  // n × 3 (x, y, z)
};

struct Triangles {
    data: array<u32>,  // m × 3 vertex indices
};

struct Radii {
    data: array<f32>,  // m circumradii
};

struct Params {
    n: u32,
    m: u32,
};

@group(0) @binding(0) var<storage, read> points: Points;
@group(0) @binding(1) var<storage, read> triangles: Triangles;
@group(0) @binding(2) var<storage, read_write> radii: Radii;
@group(0) @binding(3) var<uniform> params: Params;

fn dist(a: vec3<f32>, b: vec3<f32>) -> f32 {
    let d = a - b;
    return sqrt(dot(d, d));
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= params.m) {
        return;
    }

    let ia = triangles.data[idx * 3u];
    let ib = triangles.data[idx * 3u + 1u];
    let ic = triangles.data[idx * 3u + 2u];

    let a = vec3<f32>(points.data[ia * 3u], points.data[ia * 3u + 1u], points.data[ia * 3u + 2u]);
    let b = vec3<f32>(points.data[ib * 3u], points.data[ib * 3u + 1u], points.data[ib * 3u + 2u]);
    let c = vec3<f32>(points.data[ic * 3u], points.data[ic * 3u + 1u], points.data[ic * 3u + 2u]);

    let d_ab = dist(a, b);
    let d_bc = dist(b, c);
    let d_ca = dist(c, a);

    let s = (d_ab + d_bc + d_ca) * 0.5;
    let area_sq = s * (s - d_ab) * (s - d_bc) * (s - d_ca);

    if (area_sq <= 0.0) {
        radii.data[idx] = 1e30; // Large value for degenerate triangles.
    } else {
        radii.data[idx] = (d_ab * d_bc * d_ca) / (4.0 * sqrt(area_sq));
    }
}
"#;

// ───────────────────────────────────────────────────────────────────────────
//  Errors
// ───────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GpuOracleError {
    BufferTooSmall { needed: usize, have: usize },
    TooFewPoints { got: usize },
    KTooLarge { k: usize, n: usize },
    NonFinite { point_index: usize },
}

impl core::fmt::Display for GpuOracleError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::BufferTooSmall { needed, have } => {
                write!(
                    f,
                    "gpu_oracle: buffer too small, need {needed}, have {have}"
                )
            }
            Self::TooFewPoints { got } => write!(f, "gpu_oracle: too few points: {got}"),
            Self::KTooLarge { k, n } => write!(f, "gpu_oracle: k={k} > n={n}"),
            Self::NonFinite { point_index } => write!(f, "gpu_oracle: non-finite at {point_index}"),
        }
    }
}

impl std::error::Error for GpuOracleError {}

// ───────────────────────────────────────────────────────────────────────────
//  Differential check
// ───────────────────────────────────────────────────────────────────────────

/// Tolerance for CPU vs GPU f32 differential comparison.
pub const DIFF_TOLERANCE_F32: f32 = 1e-4;

/// Compare two f32 arrays for differential testing.
/// Returns the number of elements that differ by more than `tolerance`.
pub fn diff_f32(cpu: &[f32], gpu: &[f32], tolerance: f32) -> usize {
    let n = cpu.len().min(gpu.len());
    let mut mismatches = 0;
    for i in 0..n {
        if (cpu[i] - gpu[i]).abs() > tolerance {
            mismatches += 1;
        }
    }
    mismatches
}

/// Compare two f64 arrays for differential testing.
pub fn diff_f64(cpu: &[f64], gpu: &[f64], tolerance: f64) -> usize {
    let n = cpu.len().min(gpu.len());
    let mut mismatches = 0;
    for i in 0..n {
        if (cpu[i] - gpu[i]).abs() > tolerance {
            mismatches += 1;
        }
    }
    mismatches
}

// ───────────────────────────────────────────────────────────────────────────
//  Determinism hash
// ───────────────────────────────────────────────────────────────────────────

/// FNV-1a hash over a distance matrix.
pub fn distance_matrix_hash(distances: &[f64]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for d in distances {
        hash ^= d.to_bits();
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

// ───────────────────────────────────────────────────────────────────────────
//  Tests
// ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_point(x: f32, y: f32, z: f32) -> Tensor10D {
        Tensor10D::new(0.0, 0.0, 0.0, x, y, z, 0.0, 0.0, 0.0, 0.0)
    }

    fn grid_points(nx: usize, ny: usize, spacing: f32) -> Vec<Tensor10D> {
        let mut pts = Vec::new();
        for i in 0..nx {
            for j in 0..ny {
                pts.push(make_point(i as f32 * spacing, j as f32 * spacing, 0.0));
            }
        }
        pts
    }

    #[test]
    fn batch_pairwise_distances_deterministic() {
        let pts = grid_points(3, 3, 1.0);
        let n = pts.len();
        let mut d1 = vec![0.0f64; n * n];
        let mut d2 = vec![0.0f64; n * n];
        cpu_batch_pairwise_distances(&pts, &mut d1).unwrap();
        cpu_batch_pairwise_distances(&pts, &mut d2).unwrap();
        assert_eq!(d1, d2, "pairwise distances must be deterministic");
        assert_eq!(distance_matrix_hash(&d1), distance_matrix_hash(&d2));
    }

    #[test]
    fn batch_pairwise_distances_diagonal_zero() {
        let pts = grid_points(3, 3, 1.0);
        let n = pts.len();
        let mut d = vec![0.0f64; n * n];
        cpu_batch_pairwise_distances(&pts, &mut d).unwrap();
        for i in 0..n {
            assert!(d[i * n + i] < 1e-10, "diagonal must be 0");
        }
    }

    #[test]
    fn batch_pairwise_distances_symmetric() {
        let pts = grid_points(3, 3, 1.0);
        let n = pts.len();
        let mut d = vec![0.0f64; n * n];
        cpu_batch_pairwise_distances(&pts, &mut d).unwrap();
        for i in 0..n {
            for j in 0..n {
                assert!(
                    (d[i * n + j] - d[j * n + i]).abs() < 1e-10,
                    "distance matrix must be symmetric"
                );
            }
        }
    }

    #[test]
    fn batch_query_distances_deterministic() {
        let pts = grid_points(4, 4, 1.0);
        let query = make_point(1.5, 1.5, 0.0);
        let mut d1 = vec![0.0f64; pts.len()];
        let mut d2 = vec![0.0f64; pts.len()];
        cpu_batch_query_distances(&pts, &query, &mut d1).unwrap();
        cpu_batch_query_distances(&pts, &query, &mut d2).unwrap();
        assert_eq!(d1, d2, "query distances must be deterministic");
    }

    #[test]
    fn batch_circumradius_deterministic() {
        let pts = grid_points(3, 3, 1.0);
        let tris = [[0u32, 1, 4], [0, 3, 4], [1, 2, 5]];
        let mut r1 = vec![0.0f64; 3];
        let mut r2 = vec![0.0f64; 3];
        cpu_batch_circumradius(&pts, &tris, &mut r1).unwrap();
        cpu_batch_circumradius(&pts, &tris, &mut r2).unwrap();
        assert_eq!(r1, r2, "circumradii must be deterministic");
    }

    #[test]
    fn batch_circumradius_unit_triangle() {
        // Right triangle with legs 1, 1 → hypotenuse sqrt(2).
        // Circumradius = hypotenuse / 2 = sqrt(2)/2.
        let pts = vec![
            make_point(0.0, 0.0, 0.0),
            make_point(1.0, 0.0, 0.0),
            make_point(0.0, 1.0, 0.0),
        ];
        let tris = [[0u32, 1, 2]];
        let mut r = vec![0.0f64; 1];
        cpu_batch_circumradius(&pts, &tris, &mut r).unwrap();
        let expected = (2.0f64).sqrt() / 2.0;
        assert!(
            (r[0] - expected).abs() < 1e-10,
            "circumradius of unit right triangle should be {}, got {}",
            expected,
            r[0]
        );
    }

    #[test]
    fn batch_circumradius_degenerate_is_infinite() {
        // Collinear points → degenerate triangle.
        let pts = vec![
            make_point(0.0, 0.0, 0.0),
            make_point(1.0, 0.0, 0.0),
            make_point(2.0, 0.0, 0.0),
        ];
        let tris = [[0u32, 1, 2]];
        let mut r = vec![0.0f64; 1];
        cpu_batch_circumradius(&pts, &tris, &mut r).unwrap();
        assert!(
            r[0].is_infinite(),
            "degenerate triangle should have infinite circumradius"
        );
    }

    #[test]
    fn batch_density_deterministic() {
        let pts = grid_points(4, 4, 1.0);
        let mut d1 = vec![0.0f64; pts.len()];
        let mut d2 = vec![0.0f64; pts.len()];
        cpu_batch_density(&pts, 3, &mut d1).unwrap();
        cpu_batch_density(&pts, 3, &mut d2).unwrap();
        assert_eq!(d1, d2, "density must be deterministic");
    }

    #[test]
    fn gpu_distance_kernel_source_valid() {
        assert!(GPU_DISTANCE_KERNEL_WGSL.contains("workgroup_size(64)"));
        assert!(GPU_DISTANCE_KERNEL_WGSL.contains("sqrt"));
        assert!(GPU_DISTANCE_KERNEL_WGSL.contains("distances"));
    }

    #[test]
    fn gpu_circumradius_kernel_source_valid() {
        assert!(GPU_CIRCUMRADIUS_KERNEL_WGSL.contains("workgroup_size(64)"));
        assert!(GPU_CIRCUMRADIUS_KERNEL_WGSL.contains("area_sq"));
        assert!(GPU_CIRCUMRADIUS_KERNEL_WGSL.contains("radii"));
    }

    #[test]
    fn diff_f32_self_zero_mismatches() {
        let data = [1.0f32, 2.0, 3.0];
        assert_eq!(diff_f32(&data, &data, 1e-6), 0);
    }

    #[test]
    fn diff_f32_detects_mismatches() {
        let a = [1.0f32, 2.0, 3.0];
        let b = [1.1f32, 2.0, 3.5];
        assert_eq!(diff_f32(&a, &b, 1e-4), 2, "two elements differ by > 1e-4");
    }

    #[test]
    fn diff_f64_self_zero_mismatches() {
        let data = [1.0f64, 2.0, 3.0];
        assert_eq!(diff_f64(&data, &data, 1e-10), 0);
    }

    #[test]
    fn batch_buffer_too_small_errors() {
        let pts = grid_points(3, 3, 1.0);
        let mut d = vec![0.0f64; 5]; // too small for 9×9
        let err = cpu_batch_pairwise_distances(&pts, &mut d).unwrap_err();
        assert!(matches!(err, GpuOracleError::BufferTooSmall { .. }));
    }
}
