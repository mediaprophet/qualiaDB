//! P13.6 - Screened-Poisson surface reconstruction.
//!
//! This is a real screened solve, scoped deliberately to height-field style
//! reconstruction over the sample set's x/y domain. It solves
//! `(lambda I - Laplacian) z = lambda z_sample + div(n_xy)` on a regular grid,
//! then emits a deterministic triangle mesh. It is not the older nearest-normal
//! signed-distance placeholder in `reconstruct_3d.rs`.

use super::primitives::Point3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenedPoissonOptions {
    pub grid_x: usize,
    pub grid_y: usize,
    pub screening_weight: f64,
    pub iterations: u32,
    pub tolerance: f64,
    pub padding: f64,
}

impl Default for ScreenedPoissonOptions {
    fn default() -> Self {
        Self {
            grid_x: 16,
            grid_y: 16,
            screening_weight: 4.0,
            iterations: 256,
            tolerance: 1e-8,
            padding: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenedPoissonReport {
    pub vertex_count: usize,
    pub triangle_count: usize,
    pub iterations: u32,
    pub residual: f64,
    pub max_sample_error: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenedPoissonError {
    TooFewSamples { got: usize },
    CountMismatch { points: usize, normals: usize },
    InvalidOptions,
    NonFiniteSample { index: usize },
    OutputTooSmall { vertices: usize, triangles: usize },
}

pub fn screened_poisson_reconstruct_3d(
    points: &[Point3],
    normals: &[Point3],
    options: ScreenedPoissonOptions,
    out_vertices: &mut [Point3],
    out_triangles: &mut [[u32; 3]],
) -> Result<ScreenedPoissonReport, ScreenedPoissonError> {
    if points.len() < 3 {
        return Err(ScreenedPoissonError::TooFewSamples { got: points.len() });
    }
    if points.len() != normals.len() {
        return Err(ScreenedPoissonError::CountMismatch {
            points: points.len(),
            normals: normals.len(),
        });
    }
    if options.grid_x < 2
        || options.grid_y < 2
        || !(options.screening_weight.is_finite() && options.screening_weight > 0.0)
        || !(options.tolerance.is_finite() && options.tolerance >= 0.0)
        || !options.padding.is_finite()
    {
        return Err(ScreenedPoissonError::InvalidOptions);
    }
    for (i, p) in points.iter().enumerate() {
        let n = normals[i];
        if !p.x.is_finite()
            || !p.y.is_finite()
            || !p.z.is_finite()
            || !n.x.is_finite()
            || !n.y.is_finite()
            || !n.z.is_finite()
        {
            return Err(ScreenedPoissonError::NonFiniteSample { index: i });
        }
    }

    let vertex_count = options.grid_x * options.grid_y;
    let triangle_count = (options.grid_x - 1) * (options.grid_y - 1) * 2;
    if out_vertices.len() < vertex_count || out_triangles.len() < triangle_count {
        return Err(ScreenedPoissonError::OutputTooSmall {
            vertices: vertex_count,
            triangles: triangle_count,
        });
    }

    let bounds = bounds_xy(points, options.padding);
    let dx = if options.grid_x > 1 {
        (bounds.2 - bounds.0) / (options.grid_x - 1) as f64
    } else {
        1.0
    };
    let dy = if options.grid_y > 1 {
        (bounds.3 - bounds.1) / (options.grid_y - 1) as f64
    } else {
        1.0
    };

    let mut z = vec![0.0f64; vertex_count];
    let mut rhs = vec![0.0f64; vertex_count];
    let mut weight = vec![0.0f64; vertex_count];
    for (p, n) in points.iter().zip(normals.iter()) {
        let ix = nearest_grid_index(p.x, bounds.0, dx, options.grid_x);
        let iy = nearest_grid_index(p.y, bounds.1, dy, options.grid_y);
        let idx = iy * options.grid_x + ix;
        rhs[idx] += options.screening_weight * p.z + normal_divergence_hint(*n, dx, dy);
        weight[idx] += options.screening_weight;
        z[idx] += p.z;
    }
    for i in 0..vertex_count {
        if weight[i] > 0.0 {
            z[i] /= weight[i] / options.screening_weight;
        }
    }

    let mut residual = f64::INFINITY;
    let mut ran = 0u32;
    let mut next = z.clone();
    for it in 0..options.iterations {
        residual = 0.0;
        for y in 0..options.grid_y {
            for x in 0..options.grid_x {
                let idx = y * options.grid_x + x;
                let mut neighbour_sum = 0.0;
                let mut degree = 0.0;
                for (nx, ny) in neighbours_4(x, y, options.grid_x, options.grid_y) {
                    neighbour_sum += z[ny * options.grid_x + nx];
                    degree += 1.0;
                }
                let lambda = weight[idx].max(options.screening_weight * 0.05);
                let target = if weight[idx] > 0.0 { rhs[idx] } else { 0.0 };
                next[idx] = (target + neighbour_sum) / (lambda + degree);
                residual = residual.max((next[idx] - z[idx]).abs());
            }
        }
        core::mem::swap(&mut z, &mut next);
        ran = it + 1;
        if residual <= options.tolerance {
            break;
        }
    }

    for y in 0..options.grid_y {
        for x in 0..options.grid_x {
            let idx = y * options.grid_x + x;
            out_vertices[idx] =
                Point3::new(bounds.0 + x as f64 * dx, bounds.1 + y as f64 * dy, z[idx]);
        }
    }
    let mut ti = 0usize;
    for y in 0..options.grid_y - 1 {
        for x in 0..options.grid_x - 1 {
            let a = (y * options.grid_x + x) as u32;
            let b = (y * options.grid_x + x + 1) as u32;
            let c = ((y + 1) * options.grid_x + x + 1) as u32;
            let d = ((y + 1) * options.grid_x + x) as u32;
            out_triangles[ti] = [a, b, c];
            out_triangles[ti + 1] = [a, c, d];
            ti += 2;
        }
    }

    let max_sample_error = max_error_to_samples(points, bounds, dx, dy, options.grid_x, &z);
    Ok(ScreenedPoissonReport {
        vertex_count,
        triangle_count,
        iterations: ran,
        residual,
        max_sample_error,
    })
}

pub fn required_screened_poisson_capacity(grid_x: usize, grid_y: usize) -> (usize, usize) {
    if grid_x < 2 || grid_y < 2 {
        (0, 0)
    } else {
        (grid_x * grid_y, (grid_x - 1) * (grid_y - 1) * 2)
    }
}

fn bounds_xy(points: &[Point3], padding: f64) -> (f64, f64, f64, f64) {
    let mut min_x = points[0].x;
    let mut max_x = points[0].x;
    let mut min_y = points[0].y;
    let mut max_y = points[0].y;
    for p in points {
        min_x = min_x.min(p.x);
        max_x = max_x.max(p.x);
        min_y = min_y.min(p.y);
        max_y = max_y.max(p.y);
    }
    if min_x == max_x {
        max_x += 1.0;
    }
    if min_y == max_y {
        max_y += 1.0;
    }
    (
        min_x - padding,
        min_y - padding,
        max_x + padding,
        max_y + padding,
    )
}

fn nearest_grid_index(x: f64, min: f64, step: f64, n: usize) -> usize {
    if step == 0.0 {
        0
    } else {
        (((x - min) / step).round() as isize).clamp(0, n as isize - 1) as usize
    }
}

fn neighbours_4(x: usize, y: usize, nx: usize, ny: usize) -> impl Iterator<Item = (usize, usize)> {
    let mut out = [(usize::MAX, usize::MAX); 4];
    let mut n = 0usize;
    if x > 0 {
        out[n] = (x - 1, y);
        n += 1;
    }
    if x + 1 < nx {
        out[n] = (x + 1, y);
        n += 1;
    }
    if y > 0 {
        out[n] = (x, y - 1);
        n += 1;
    }
    if y + 1 < ny {
        out[n] = (x, y + 1);
        n += 1;
    }
    out.into_iter().take(n)
}

fn normal_divergence_hint(n: Point3, dx: f64, dy: f64) -> f64 {
    if n.z.abs() < 1e-12 {
        0.0
    } else {
        -0.25 * (n.x / dx.max(1e-12) + n.y / dy.max(1e-12)) / n.z
    }
}

fn max_error_to_samples(
    points: &[Point3],
    bounds: (f64, f64, f64, f64),
    dx: f64,
    dy: f64,
    grid_x: usize,
    z: &[f64],
) -> f64 {
    let mut err = 0.0f64;
    for p in points {
        let ix = nearest_grid_index(p.x, bounds.0, dx, grid_x);
        let iy = nearest_grid_index(p.y, bounds.1, dy, z.len() / grid_x);
        err = err.max((z[iy * grid_x + ix] - p.z).abs());
    }
    err
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_inputs() {
        let p = [Point3::new(0.0, 0.0, 0.0)];
        let n = [Point3::new(0.0, 0.0, 1.0)];
        let mut v = [Point3::new(0.0, 0.0, 0.0); 4];
        let mut t = [[0u32; 3]; 2];
        assert_eq!(
            screened_poisson_reconstruct_3d(
                &p,
                &n,
                ScreenedPoissonOptions::default(),
                &mut v,
                &mut t
            ),
            Err(ScreenedPoissonError::TooFewSamples { got: 1 })
        );
    }

    #[test]
    fn reconstructs_flat_plane_samples() {
        let points = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
        ];
        let normals = vec![Point3::new(0.0, 0.0, 1.0); points.len()];
        let opts = ScreenedPoissonOptions {
            grid_x: 4,
            grid_y: 4,
            screening_weight: 8.0,
            iterations: 64,
            tolerance: 1e-10,
            padding: 0.0,
        };
        let (vc, tc) = required_screened_poisson_capacity(opts.grid_x, opts.grid_y);
        let mut v = vec![Point3::new(0.0, 0.0, 0.0); vc];
        let mut t = vec![[0u32; 3]; tc];
        let report =
            screened_poisson_reconstruct_3d(&points, &normals, opts, &mut v, &mut t).unwrap();
        assert_eq!(report.vertex_count, 16);
        assert_eq!(report.triangle_count, 18);
        assert!(v.iter().all(|p| p.z.abs() < 1e-8));
    }

    #[test]
    fn deterministic_reconstruction() {
        let points = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.2),
            Point3::new(0.0, 1.0, 0.1),
            Point3::new(1.0, 1.0, 0.3),
        ];
        let normals = vec![Point3::new(0.0, 0.0, 1.0); points.len()];
        let opts = ScreenedPoissonOptions {
            grid_x: 5,
            grid_y: 5,
            ..ScreenedPoissonOptions::default()
        };
        let (vc, tc) = required_screened_poisson_capacity(opts.grid_x, opts.grid_y);
        let mut av = vec![Point3::new(0.0, 0.0, 0.0); vc];
        let mut at = vec![[0u32; 3]; tc];
        let mut bv = av.clone();
        let mut bt = at.clone();
        let ar =
            screened_poisson_reconstruct_3d(&points, &normals, opts, &mut av, &mut at).unwrap();
        let br =
            screened_poisson_reconstruct_3d(&points, &normals, opts, &mut bv, &mut bt).unwrap();
        assert_eq!(ar, br);
        assert_eq!(av, bv);
        assert_eq!(at, bt);
    }
}
