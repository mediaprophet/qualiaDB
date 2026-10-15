//! P21.2 — Discrete heat-method geodesics (Crane, Weischedel & Wardetzky 2013).
//!
//! Assembles the full pipeline on top of `ddg_operators`:
//!   1. Diffuse a unit Dirac δ at `source` for `dt = t_factor · h̄²`
//!      (`heat_step`, explicit Euler).
//!   2. Per-face normalized gradient X = −∇u/|∇u|.
//!   3. Vertex divergence via dual-cell flux  div_i = Σ_f ½(N̂_f × e_i^opp)·X_f.
//!   4. Solve the cotan Laplacian Poisson equation L φ = −div
//!      (`solve_poisson_jacobi`) and shift min(φ) → 0.
//!
//! `ColdBounded`: composes the existing operators (which own bounded scratch)
//! rather than duplicating them.

use super::ddg_operators::{
    cotangent_laplacian, geodesic_distances_dijkstra, heat_step, solve_poisson_jacobi, DdgError,
    SparseEntry,
};
use super::primitives::Point3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HeatGeodesicError {
    EmptyInput,
    BadSource { index: usize },
    Ddg(DdgError),
}

impl From<DdgError> for HeatGeodesicError {
    fn from(e: DdgError) -> Self {
        HeatGeodesicError::Ddg(e)
    }
}

#[inline]
fn sub(a: Point3, b: Point3) -> Point3 {
    Point3::new(a.x - b.x, a.y - b.y, a.z - b.z)
}
#[inline]
fn dot(a: Point3, b: Point3) -> f64 {
    a.x * b.x + a.y * b.y + a.z * b.z
}
#[inline]
fn cross(a: Point3, b: Point3) -> Point3 {
    Point3::new(
        a.y * b.z - a.z * b.y,
        a.z * b.x - a.x * b.z,
        a.x * b.y - a.y * b.x,
    )
}

/// Heat-method geodesic distance field from `source` over a triangulated
/// surface. `t_factor` scales the diffusion time (default ≈ 1.0).
/// `jacobi_iterations` bounds the Poisson solve.
///
/// `out_distances[i]` receives the geodesic distance estimate (min-shifted so
/// the source is 0).
pub fn heat_geodesic_distances(
    vertices: &[Point3],
    triangles: &[[u32; 3]],
    source: usize,
    t_factor: f64,
    jacobi_iterations: u32,
    out_distances: &mut [f64],
) -> Result<(), HeatGeodesicError> {
    let n = vertices.len();
    if n == 0 || triangles.is_empty() {
        return Err(HeatGeodesicError::EmptyInput);
    }
    if source >= n {
        return Err(HeatGeodesicError::BadSource { index: source });
    }
    if out_distances.len() < n || !(t_factor > 0.0 && t_factor.is_finite()) {
        return Err(HeatGeodesicError::Ddg(DdgError::DimensionMismatch));
    }

    // Cotan Laplacian + lumped mass.
    let mut lap = vec![
        SparseEntry {
            row: 0,
            col: 0,
            value: 0.0,
        };
        triangles.len() * 9
    ];
    let mut mass = vec![0.0f64; n];
    let nnz = cotangent_laplacian(vertices, triangles, &mut lap, &mut mass)?;
    let lap = &lap[..nnz];

    // Mean edge length → diffusion time dt = t_factor · h̄².
    let mut h_sum = 0.0;
    let mut h_cnt = 0usize;
    for t in triangles {
        for e in 0..3 {
            let (a, b) = (vertices[t[e] as usize], vertices[t[(e + 1) % 3] as usize]);
            h_sum += dot(sub(a, b), sub(a, b)).sqrt();
            h_cnt += 1;
        }
    }
    let h = (h_sum / (h_cnt as f64)).max(1e-9);
    let dt = t_factor * h * h;

    // Step 1 — diffuse Dirac.
    let mut u = vec![0.0f64; n];
    u[source] = 1.0;
    for _ in 0..8 {
        heat_step(n, lap, &mass, dt, &mut u)?;
    }

    // Step 2+3 — per-face X = −∇u/|∇u| and vertex divergence.
    let mut x_face = vec![Point3::default(); triangles.len()];
    let mut div = vec![0.0f64; n];
    for (fi, tri) in triangles.iter().enumerate() {
        let [i0, i1, i2] = [tri[0] as usize, tri[1] as usize, tri[2] as usize];
        if i0 >= n || i1 >= n || i2 >= n {
            return Err(HeatGeodesicError::Ddg(DdgError::DimensionMismatch));
        }
        let (p0, p1, p2) = (vertices[i0], vertices[i1], vertices[i2]);
        let nrm = cross(sub(p1, p0), sub(p2, p0));
        let a2 = dot(nrm, nrm).sqrt().max(1e-15);
        let nhat = Point3::new(nrm.x / a2, nrm.y / a2, nrm.z / a2);
        // Opposite-edge vectors (ccw): e_i = p_{i+2} − p_{i+1}.
        let e_opp = [sub(p2, p1), sub(p0, p2), sub(p1, p0)];
        let mut grad = Point3::default();
        for k in 0..3 {
            let idx = tri[k] as usize;
            // ∇λ_i ∝ N̂ × e_i (points toward vertex i).
            let r = cross(nhat, e_opp[k]);
            grad = Point3::new(
                grad.x + u[idx] * r.x / a2,
                grad.y + u[idx] * r.y / a2,
                grad.z + u[idx] * r.z / a2,
            );
        }
        let g = dot(grad, grad).sqrt();
        let x = if g > 1e-12 {
            Point3::new(-grad.x / g, -grad.y / g, -grad.z / g)
        } else {
            Point3::default()
        };
        x_face[fi] = x;
        for k in 0..3 {
            let flux = dot(cross(nhat, e_opp[k]), x) * 0.5;
            div[tri[k] as usize] += flux;
        }
    }

    // Step 4 — Poisson solve: our L is SPD (L = −∆), so solve L φ = −div.
    for d in div.iter_mut() {
        *d = -*d;
    }
    let mut phi = vec![0.0f64; n];
    let mut fixed = vec![false; n];
    fixed[source] = true;
    solve_poisson_jacobi(n, lap, &div, &fixed, &mut phi, jacobi_iterations, 1e-8)?;

    let min = phi.iter().copied().fold(f64::INFINITY, f64::min);
    let shift = if min.is_finite() { min } else { 0.0 };
    for i in 0..n {
        out_distances[i] = (phi[i] - shift).max(0.0);
    }
    out_distances[source] = 0.0;
    Ok(())
}

/// Cross-check heat geodesics against the Dijkstra baseline: returns the max
/// |heat − dijkstra| over the mesh. ColdBounded diagnostic.
pub fn heat_vs_dijkstra_max_deviation(
    vertices: &[Point3],
    triangles: &[[u32; 3]],
    source: usize,
    t_factor: f64,
    heat_out: &mut [f64],
    dijkstra_out: &mut [f64],
) -> Result<f64, HeatGeodesicError> {
    heat_geodesic_distances(vertices, triangles, source, t_factor, 400, heat_out)?;
    geodesic_distances_dijkstra(vertices, triangles, source, dijkstra_out)
        .map_err(HeatGeodesicError::Ddg)?;
    let mut dev = 0.0f64;
    for i in 0..vertices.len() {
        let a = heat_out[i];
        let b = dijkstra_out[i];
        if a.is_finite() && b.is_finite() {
            dev = dev.max((a - b).abs());
        }
    }
    Ok(dev)
}

#[cfg(test)]
mod tests {
    use super::*;

    // 3×3 grid mesh (8 triangles).
    fn grid() -> (Vec<Point3>, Vec<[u32; 3]>) {
        let mut v = Vec::new();
        for y in 0..3 {
            for x in 0..3 {
                v.push(Point3::new(x as f64, y as f64, 0.0));
            }
        }
        let t = vec![
            [0, 1, 4],
            [0, 4, 3],
            [1, 2, 5],
            [1, 5, 4],
            [3, 4, 7],
            [3, 7, 6],
            [4, 5, 8],
            [4, 8, 7],
        ];
        (v, t)
    }

    #[test]
    fn heat_geodesics_monotone_away_from_source() {
        let (v, t) = grid();
        let mut d = [0.0f64; 9];
        heat_geodesic_distances(&v, &t, 0, 1.0, 400, &mut d).unwrap();
        assert_eq!(d[0], 0.0);
        assert!(d[8] > d[1], "far corner must exceed near neighbour");
        assert!(d.iter().all(|x| x.is_finite() && *x >= 0.0));
    }
}
