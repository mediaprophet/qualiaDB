//! P6.4 — Implicit / advancing-front surface reconstruction from oriented
//! point sets.
//!
//! Given a set of oriented points (position + normal), reconstruct a
//! triangle mesh surface. This module implements a simple Poisson-like
//! approach: compute an implicit function (signed distance field) from the
//! point set, then extract the isosurface via marching cubes.
//!
//! ## Algorithm
//!
//! 1. For each grid point, compute the signed distance to the nearest
//!    oriented point: `f(x) = (x - p) · n` where p is the nearest point
//!    and n is its normal.
//! 2. Run marching cubes at isolevel 0 to extract the surface.
//!
//! This is a simplified Poisson reconstruction. A full implementation
//! would solve a Poisson system over the oriented normals.
//!
//! ## Determinism
//!
//! Grid traversal is in (x, y, z) order. Nearest-point queries are
//! deterministic (brute-force, ties broken by index). Identical input →
//! bit-identical output.

use super::distance::distance_sq_3d;
use super::isosurface::{marching_cubes, IsosurfaceError};
use super::primitives::Point3;

// ───────────────────────────────────────────────────────────────────────────
//  Errors
// ───────────────────────────────────────────────────────────────────────────

/// Surface reconstruction error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReconstructionError {
    /// Too few points for reconstruction.
    TooFewPoints { got: usize },
    /// Point/normals count mismatch.
    CountMismatch { points: usize, normals: usize },
    /// Grid dimensions are zero.
    EmptyGrid,
    /// Isosurface extraction failed.
    IsosurfaceFailed(IsosurfaceError),
    /// Output buffer too small.
    BufferTooSmall { needed: usize, have: usize },
}

impl core::fmt::Display for ReconstructionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::TooFewPoints { got } => write!(f, "reconstruction: too few points: {got}"),
            Self::CountMismatch { points, normals } => {
                write!(f, "reconstruction: count mismatch, {points} points vs {normals} normals")
            }
            Self::EmptyGrid => write!(f, "reconstruction: empty grid"),
            Self::IsosurfaceFailed(e) => write!(f, "reconstruction: isosurface failed: {e}"),
            Self::BufferTooSmall { needed, have } => {
                write!(f, "reconstruction: buffer too small, need {needed}, have {have}")
            }
        }
    }
}

impl std::error::Error for ReconstructionError {}

// ───────────────────────────────────────────────────────────────────────────
//  Poisson-like surface reconstruction
// ───────────────────────────────────────────────────────────────────────────

/// Poisson-like surface reconstruction from oriented points.
///
/// Computes a signed distance field on a regular grid, then extracts the
/// isosurface at level 0 using marching cubes.
///
/// `points` and `normals` must have the same length.
/// The grid is `nx * ny * nz` with spacing `(dx, dy, dz)` and origin
/// `(origin_x, origin_y, origin_z)`.
///
/// `grid_scratch` needs `nx * ny * nz` entries.
/// `out_vertices` needs `(nx-1)*(ny-1)*(nz-1)*30` entries.
/// `out_triangles` needs `(nx-1)*(ny-1)*(nz-1)*10` entries.
///
/// Returns `(vertex_count, triangle_count)`.
pub fn poisson_reconstruct_3d(
    points: &[Point3],
    normals: &[Point3],
    nx: usize,
    ny: usize,
    nz: usize,
    dx: f64,
    dy: f64,
    dz: f64,
    origin_x: f64,
    origin_y: f64,
    origin_z: f64,
    grid_scratch: &mut [f64],
    out_vertices: &mut [Point3],
    out_triangles: &mut [[u32; 3]],
) -> Result<(usize, usize), ReconstructionError> {
    if points.len() < 4 {
        return Err(ReconstructionError::TooFewPoints { got: points.len() });
    }
    if points.len() != normals.len() {
        return Err(ReconstructionError::CountMismatch {
            points: points.len(),
            normals: normals.len(),
        });
    }
    if nx == 0 || ny == 0 || nz == 0 {
        return Err(ReconstructionError::EmptyGrid);
    }
    let grid_size = nx * ny * nz;
    if grid_scratch.len() < grid_size {
        return Err(ReconstructionError::BufferTooSmall {
            needed: grid_size,
            have: grid_scratch.len(),
        });
    }

    // Compute signed distance field.
    for k in 0..nz {
        for j in 0..ny {
            for i in 0..nx {
                let x = origin_x + i as f64 * dx;
                let y = origin_y + j as f64 * dy;
                let z = origin_z + k as f64 * dz;
                let query = Point3::new(x, y, z);

                // Find nearest point (brute force).
                let mut best_idx = 0usize;
                let mut best_dist_sq = f64::INFINITY;
                for (pi, &p) in points.iter().enumerate() {
                    let d = distance_sq_3d(query, p);
                    if d < best_dist_sq {
                        best_dist_sq = d;
                        best_idx = pi;
                    }
                }

                // Signed distance: (x - p) · n.
                let p = points[best_idx];
                let n = normals[best_idx];
                let signed_dist = (query.x - p.x) * n.x + (query.y - p.y) * n.y + (query.z - p.z) * n.z;
                grid_scratch[i + j * nx + k * nx * ny] = signed_dist;
            }
        }
    }

    // Extract isosurface at level 0.
    let (vc, tc) = marching_cubes(
        grid_scratch, nx, ny, nz, dx, dy, dz,
        origin_x, origin_y, origin_z, 0.0,
        out_vertices, out_triangles,
    ).map_err(ReconstructionError::IsosurfaceFailed)?;

    Ok((vc, tc))
}

// ───────────────────────────────────────────────────────────────────────────
//  Tests
// ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sphere_points_normals(n: usize, cx: f64, cy: f64, cz: f64, r: f64) -> (Vec<Point3>, Vec<Point3>) {
        let mut pts = Vec::with_capacity(n);
        let mut nrm = Vec::with_capacity(n);
        // Fibonacci sphere distribution.
        let golden = (1.0 + 5.0f64.sqrt()) / 2.0;
        for i in 0..n {
            let t = i as f64 / n as f64;
            let phi = (2.0 * core::f64::consts::PI * i as f64 / golden).cos().acos() * 0.0; // placeholder
            let inclination = (1.0 - 2.0 * t).acos();
            let azimuth = 2.0 * core::f64::consts::PI * i as f64 / golden;
            let x = r * inclination.sin() * azimuth.cos();
            let y = r * inclination.sin() * azimuth.sin();
            let z = r * inclination.cos();
            pts.push(Point3::new(cx + x, cy + y, cz + z));
            // Outward normal.
            let len = (x * x + y * y + z * z).sqrt();
            nrm.push(Point3::new(x / len, y / len, z / len));
        }
        (pts, nrm)
    }

    #[test]
    fn poisson_sphere_reconstruction() {
        let (pts, nrm) = sphere_points_normals(50, 5.0, 5.0, 5.0, 3.0);
        let nx = 10;
        let ny = 10;
        let nz = 10;
        let mut grid = vec![0.0f64; nx * ny * nz];
        let max_verts = (nx - 1) * (ny - 1) * (nz - 1) * 30;
        let max_tris = (nx - 1) * (ny - 1) * (nz - 1) * 10;
        let mut verts = vec![Point3::default(); max_verts];
        let mut tris = vec![[0u32; 3]; max_tris];

        let (vc, tc) = poisson_reconstruct_3d(
            &pts, &nrm, nx, ny, nz, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0,
            &mut grid, &mut verts, &mut tris,
        ).unwrap();

        assert!(vc > 0, "sphere reconstruction should produce vertices");
        assert!(tc > 0, "sphere reconstruction should produce triangles");
    }

    #[test]
    fn poisson_too_few_points() {
        let pts = vec![Point3::new(0.0, 0.0, 0.0)];
        let nrm = vec![Point3::new(1.0, 0.0, 0.0)];
        let mut grid = vec![0.0f64; 8];
        let mut verts = vec![Point3::default(); 100];
        let mut tris = vec![[0u32; 3]; 100];
        assert!(matches!(
            poisson_reconstruct_3d(&pts, &nrm, 2, 2, 2, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, &mut grid, &mut verts, &mut tris),
            Err(ReconstructionError::TooFewPoints { .. })
        ));
    }

    #[test]
    fn poisson_count_mismatch() {
        let pts = vec![Point3::new(0.0, 0.0, 0.0); 5];
        let nrm = vec![Point3::new(1.0, 0.0, 0.0); 4];
        let mut grid = vec![0.0f64; 8];
        let mut verts = vec![Point3::default(); 100];
        let mut tris = vec![[0u32; 3]; 100];
        assert!(matches!(
            poisson_reconstruct_3d(&pts, &nrm, 2, 2, 2, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, &mut grid, &mut verts, &mut tris),
            Err(ReconstructionError::CountMismatch { .. })
        ));
    }

    #[test]
    fn poisson_determinism() {
        let (pts, nrm) = sphere_points_normals(30, 4.0, 4.0, 4.0, 2.5);
        let nx = 8;
        let ny = 8;
        let nz = 8;
        let max_verts = (nx - 1) * (ny - 1) * (nz - 1) * 30;
        let max_tris = (nx - 1) * (ny - 1) * (nz - 1) * 10;

        let mut g1 = vec![0.0f64; nx * ny * nz];
        let mut v1 = vec![Point3::default(); max_verts];
        let mut t1 = vec![[0u32; 3]; max_tris];
        let mut g2 = vec![0.0f64; nx * ny * nz];
        let mut v2 = vec![Point3::default(); max_verts];
        let mut t2 = vec![[0u32; 3]; max_tris];

        let (vc1, tc1) = poisson_reconstruct_3d(&pts, &nrm, nx, ny, nz, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, &mut g1, &mut v1, &mut t1).unwrap();
        let (vc2, tc2) = poisson_reconstruct_3d(&pts, &nrm, nx, ny, nz, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, &mut g2, &mut v2, &mut t2).unwrap();

        assert_eq!(vc1, vc2);
        assert_eq!(tc1, tc2);
        // Compare vertex positions.
        for i in 0..vc1 {
            assert_eq!(v1[i].x.to_bits(), v2[i].x.to_bits(), "vertex {i} x mismatch");
            assert_eq!(v1[i].y.to_bits(), v2[i].y.to_bits(), "vertex {i} y mismatch");
            assert_eq!(v1[i].z.to_bits(), v2[i].z.to_bits(), "vertex {i} z mismatch");
        }
    }
}
