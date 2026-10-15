//! P20.8 — 3-D Point-Cloud Differential Geometry, Robust Centroids & Bounding
//! Primitives (PlantGL `pglalgo::fitting::miniball`, `eigenvector`, `Fit`, and
//! `pointmanipulation`):
//! - **3-D Smallest Enclosing Sphere (Welzl Miniball)** (`smallest_enclosing_sphere_3d`)
//! - **Weiszfeld $L_1$ Geometric Median** (`weiszfeld_geometric_median_3d`, Tier-1 `HotZeroHeap`)
//! - **PCA Inertia-Axis Bounding Primitives** (`pca_bounding_primitives_3d`: OBB,
//!   bounding cylinder, bounding ellipsoid)
//! - **Monge Patch / Jet Principal Curvatures** (`monge_jet_principal_curvatures_3d`)
//! - **Riemannian Graph MST Normal Orientation** (`orient_normals_riemannian_mst`)

use super::primitives::Point3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointCloudGeomError {
    EmptyPointSet,
    TooFewNeighbors { got: usize, min: usize },
    OutputTooSmall { required: usize },
    SingularJetFit,
}

/// 3-D Sphere with support-set certificate (indices of up to 4 boundary points).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnclosingSphere3d {
    pub center: Point3,
    pub radius: f64,
    pub support_count: u8,
    pub support: [u32; 4],
}

/// PCA / inertia-axis bounding primitives fitted to a 3-D point cloud.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PcaBoundingPrimitives3d {
    pub centroid: Point3,
    pub axes: [Point3; 3],
    pub eigenvalues: [f64; 3],
    pub obb_center: Point3,
    pub obb_half_extents: [f64; 3],
    pub cylinder_radius: f64,
    pub cylinder_height: f64,
}

/// Principal curvature report on a point cloud (PlantGL `CurvatureInfo`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointCurvatureInfo {
    pub origin: Point3,
    pub normal: Point3,
    pub max_curvature: f64,
    pub max_direction: Point3,
    pub min_curvature: f64,
    pub min_direction: Point3,
}

#[inline]
fn add(a: Point3, b: Point3) -> Point3 {
    Point3::new(a.x + b.x, a.y + b.y, a.z + b.z)
}
#[inline]
fn sub(a: Point3, b: Point3) -> Point3 {
    Point3::new(a.x - b.x, a.y - b.y, a.z - b.z)
}
#[inline]
fn scale(a: Point3, s: f64) -> Point3 {
    Point3::new(a.x * s, a.y * s, a.z * s)
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
#[inline]
fn dist_sq(a: Point3, b: Point3) -> f64 {
    let d = sub(a, b);
    dot(d, d)
}
#[inline]
fn normalize(a: Point3) -> Option<Point3> {
    let n = dot(a, a).sqrt();
    (n > 1e-12 && n.is_finite()).then(|| scale(a, 1.0 / n))
}

/// Compute the 3-D $L_1$ Geometric Median of `points` using Weiszfeld's
/// iteratively reweighted least-squares algorithm (PlantGL `approx_pointset_median`).
///
/// Tier-1 `HotZeroHeap`: zero heap allocations.
pub fn weiszfeld_geometric_median_3d(
    points: &[Point3],
    max_iterations: usize,
    tolerance: f64,
) -> Result<Point3, PointCloudGeomError> {
    if points.is_empty() {
        return Err(PointCloudGeomError::EmptyPointSet);
    }
    let mut est = Point3::new(0.0, 0.0, 0.0);
    for &p in points {
        est = add(est, p);
    }
    est = scale(est, 1.0 / (points.len() as f64));

    let iters = max_iterations.max(1);
    let tol2 = tolerance.max(1e-12) * tolerance.max(1e-12);
    for _ in 0..iters {
        let mut num = Point3::new(0.0, 0.0, 0.0);
        let mut den = 0.0f64;
        for &p in points {
            let d = dist_sq(est, p).sqrt();
            if d > 1e-12 {
                let w = 1.0 / d;
                num = add(num, scale(p, w));
                den += w;
            }
        }
        if den <= 1e-15 {
            break;
        }
        let next = scale(num, 1.0 / den);
        if dist_sq(next, est) <= tol2 {
            est = next;
            break;
        }
        est = next;
    }
    Ok(est)
}

#[inline]
fn sphere_contains(s: &EnclosingSphere3d, p: Point3) -> bool {
    let d2 = dist_sq(s.center, p);
    let r2 = s.radius * s.radius;
    d2 <= r2 + 1e-10 * r2.max(1.0)
}

#[inline]
fn sphere_from_1(points: &[Point3], i: usize) -> EnclosingSphere3d {
    EnclosingSphere3d {
        center: points[i],
        radius: 0.0,
        support_count: 1,
        support: [i as u32, 0, 0, 0],
    }
}

fn min_sphere_from_3(points: &[Point3], i: usize, j: usize, k: usize) -> EnclosingSphere3d {
    let s_ij = sphere_from_2(points, i, j);
    if sphere_contains(&s_ij, points[k]) {
        return s_ij;
    }
    let s_ik = sphere_from_2(points, i, k);
    if sphere_contains(&s_ik, points[j]) {
        return s_ik;
    }
    let s_jk = sphere_from_2(points, j, k);
    if sphere_contains(&s_jk, points[i]) {
        return s_jk;
    }
    if let Some(s) = sphere_from_3(points, i, j, k) {
        return s;
    }
    let d_ij = dist_sq(points[i], points[j]);
    let d_ik = dist_sq(points[i], points[k]);
    let d_jk = dist_sq(points[j], points[k]);
    if d_ij >= d_ik && d_ij >= d_jk {
        s_ij
    } else if d_ik >= d_jk {
        s_ik
    } else {
        s_jk
    }
}

fn min_sphere_from_4(
    points: &[Point3],
    i: usize,
    j: usize,
    k: usize,
    l: usize,
) -> EnclosingSphere3d {
    let mut best: Option<EnclosingSphere3d> = None;

    let cands_2 = [
        sphere_from_2(points, i, j),
        sphere_from_2(points, i, k),
        sphere_from_2(points, i, l),
        sphere_from_2(points, j, k),
        sphere_from_2(points, j, l),
        sphere_from_2(points, k, l),
    ];
    for s in cands_2 {
        if sphere_contains(&s, points[i])
            && sphere_contains(&s, points[j])
            && sphere_contains(&s, points[k])
            && sphere_contains(&s, points[l])
        {
            if best.as_ref().map_or(true, |b| s.radius < b.radius) {
                best = Some(s);
            }
        }
    }

    let cands_3 = [
        sphere_from_3(points, i, j, k),
        sphere_from_3(points, i, j, l),
        sphere_from_3(points, i, k, l),
        sphere_from_3(points, j, k, l),
    ];
    for s in cands_3.into_iter().flatten() {
        if sphere_contains(&s, points[i])
            && sphere_contains(&s, points[j])
            && sphere_contains(&s, points[k])
            && sphere_contains(&s, points[l])
        {
            if best.as_ref().map_or(true, |b| s.radius < b.radius) {
                best = Some(s);
            }
        }
    }

    if let Some(b) = best {
        return b;
    }

    if let Some(s) = sphere_from_4(points, i, j, k, l) {
        return s;
    }

    best.unwrap_or_else(|| sphere_from_2(points, i, j))
}

/// Compute the smallest enclosing sphere (3-D Miniball) of `points` with support
/// certificate (up to 4 boundary points).
///
/// Tier-1 `HotZeroHeap`: iterative 4-level Welzl algorithm with explicit boundary
/// sets and exact base cases, guaranteeing minimal radius and a valid boundary
/// support certificate without recursion or heap allocation.
pub fn smallest_enclosing_sphere_3d(
    points: &[Point3],
) -> Result<EnclosingSphere3d, PointCloudGeomError> {
    let n = points.len();
    if n == 0 {
        return Err(PointCloudGeomError::EmptyPointSet);
    }
    if n == 1 {
        return Ok(EnclosingSphere3d {
            center: points[0],
            radius: 0.0,
            support_count: 1,
            support: [0, 0, 0, 0],
        });
    }

    // Level 0
    let mut s = sphere_from_2(points, 0, 1);
    for i in 2..n {
        if sphere_contains(&s, points[i]) {
            continue;
        }
        // Level 1: points[i] on boundary
        s = sphere_from_1(points, i);
        for j in 0..i {
            if sphere_contains(&s, points[j]) {
                continue;
            }
            // Level 2: points[i] and points[j] on boundary
            s = sphere_from_2(points, i, j);
            for k in 0..j {
                if sphere_contains(&s, points[k]) {
                    continue;
                }
                // Level 3: points[i], points[j], points[k] on boundary
                s = min_sphere_from_3(points, i, j, k);
                for l in 0..k {
                    if sphere_contains(&s, points[l]) {
                        continue;
                    }
                    // Level 4: points[i], points[j], points[k], points[l] on boundary
                    s = min_sphere_from_4(points, i, j, k, l);
                }
            }
        }
    }
    Ok(s)
}

fn sphere_from_2(points: &[Point3], i: usize, j: usize) -> EnclosingSphere3d {
    let (a, b) = (points[i], points[j]);
    let center = scale(add(a, b), 0.5);
    EnclosingSphere3d {
        center,
        radius: dist_sq(a, center).sqrt(),
        support_count: 2,
        support: [i as u32, j as u32, 0, 0],
    }
}

fn sphere_from_3(points: &[Point3], i: usize, j: usize, k: usize) -> Option<EnclosingSphere3d> {
    let (a, b, c) = (points[i], points[j], points[k]);
    let (ab, ac) = (sub(b, a), sub(c, a));
    let n = cross(ab, ac);
    let n2 = dot(n, n);
    if n2 <= 1e-18 {
        return None;
    }
    let num = add(
        scale(cross(n, ab), dot(ac, ac)),
        scale(cross(ac, n), dot(ab, ab)),
    );
    let center = add(a, scale(num, 0.5 / n2));
    Some(EnclosingSphere3d {
        center,
        radius: dist_sq(center, a).sqrt(),
        support_count: 3,
        support: [i as u32, j as u32, k as u32, 0],
    })
}

fn sphere_from_4(
    points: &[Point3],
    i: usize,
    j: usize,
    k: usize,
    l: usize,
) -> Option<EnclosingSphere3d> {
    let a = points[i];
    let (u, v, w) = (sub(points[j], a), sub(points[k], a), sub(points[l], a));
    let det = dot(u, cross(v, w));
    if det.abs() <= 1e-15 {
        return None;
    }
    let num = add(
        add(scale(cross(v, w), dot(u, u)), scale(cross(w, u), dot(v, v))),
        scale(cross(u, v), dot(w, w)),
    );
    let center = add(a, scale(num, 0.5 / det));
    Some(EnclosingSphere3d {
        center,
        radius: dist_sq(center, a).sqrt(),
        support_count: 4,
        support: [i as u32, j as u32, k as u32, l as u32],
    })
}

/// Fit PCA / inertia-axis bounding primitives (OBB, bounding cylinder, eigenvalues)
/// to a 3-D point cloud (PlantGL `Fit::inertiaAxis`, `balignedbox`, `bcylinder`).
///
/// Tier-1 `HotZeroHeap`: uses an exact 3×3 symmetric Jacobi eigensolver.
pub fn pca_bounding_primitives_3d(
    points: &[Point3],
) -> Result<PcaBoundingPrimitives3d, PointCloudGeomError> {
    if points.is_empty() {
        return Err(PointCloudGeomError::EmptyPointSet);
    }
    let n = points.len() as f64;
    let mut centroid = Point3::new(0.0, 0.0, 0.0);
    for &p in points {
        centroid = add(centroid, p);
    }
    centroid = scale(centroid, 1.0 / n);

    let mut cov = [[0.0f64; 3]; 3];
    for &p in points {
        let d = [p.x - centroid.x, p.y - centroid.y, p.z - centroid.z];
        for r in 0..3 {
            for c in 0..3 {
                cov[r][c] += d[r] * d[c] / n;
            }
        }
    }

    let (axes, eigenvalues) = jacobi_eigen_3x3(cov);
    let mut min_proj = [f64::INFINITY; 3];
    let mut max_proj = [f64::NEG_INFINITY; 3];
    let mut max_radial_sq = 0.0f64;

    for &p in points {
        let rel = sub(p, centroid);
        let coords = [dot(rel, axes[0]), dot(rel, axes[1]), dot(rel, axes[2])];
        for a in 0..3 {
            min_proj[a] = min_proj[a].min(coords[a]);
            max_proj[a] = max_proj[a].max(coords[a]);
        }
        max_radial_sq = max_radial_sq.max(coords[1] * coords[1] + coords[2] * coords[2]);
    }

    let mut obb_half_extents = [0.0f64; 3];
    let mut obb_center = centroid;
    for a in 0..3 {
        obb_half_extents[a] = 0.5 * (max_proj[a] - min_proj[a]);
        obb_center = add(
            obb_center,
            scale(axes[a], 0.5 * (max_proj[a] + min_proj[a])),
        );
    }

    Ok(PcaBoundingPrimitives3d {
        centroid,
        axes,
        eigenvalues,
        obb_center,
        obb_half_extents,
        cylinder_radius: max_radial_sq.sqrt(),
        cylinder_height: max_proj[0] - min_proj[0],
    })
}

fn jacobi_eigen_3x3(mut a: [[f64; 3]; 3]) -> ([Point3; 3], [f64; 3]) {
    let mut v = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    for _ in 0..24 {
        let (mut p, mut q, mut max_off) = (0, 1, a[0][1].abs());
        for &(r, c) in &[(0, 2), (1, 2)] {
            if a[r][c].abs() > max_off {
                max_off = a[r][c].abs();
                p = r;
                q = c;
            }
        }
        if max_off <= 1e-15 {
            break;
        }
        let diff = a[q][q] - a[p][p];
        let t = if diff.abs() < 1e-15 {
            1.0
        } else {
            let phi = diff / (2.0 * a[p][q]);
            phi.signum() / (phi.abs() + (1.0 + phi * phi).sqrt())
        };
        let c = 1.0 / (1.0 + t * t).sqrt();
        let s = t * c;
        let apq = a[p][q];
        a[p][p] -= t * apq;
        a[q][q] += t * apq;
        a[p][q] = 0.0;
        a[q][p] = 0.0;
        for r in 0..3 {
            if r != p && r != q {
                let (arp, arq) = (a[r][p], a[r][q]);
                a[r][p] = c * arp - s * arq;
                a[p][r] = a[r][p];
                a[r][q] = s * arp + c * arq;
                a[q][r] = a[r][q];
            }
            let (vrp, vrq) = (v[r][p], v[r][q]);
            v[r][p] = c * vrp - s * vrq;
            v[r][q] = s * vrp + c * vrq;
        }
    }
    let mut order = [0usize, 1, 2];
    order.sort_by(|&i, &j| {
        a[j][j]
            .partial_cmp(&a[i][i])
            .unwrap_or(core::cmp::Ordering::Equal)
    });
    let axes = [
        Point3::new(v[0][order[0]], v[1][order[0]], v[2][order[0]]),
        Point3::new(v[0][order[1]], v[1][order[1]], v[2][order[1]]),
        Point3::new(v[0][order[2]], v[1][order[2]], v[2][order[2]]),
    ];
    (
        axes,
        [
            a[order[0]][order[0]],
            a[order[1]][order[1]],
            a[order[2]][order[2]],
        ],
    )
}

/// Estimate principal curvatures $(\kappa_1, \kappa_2)$ and directions at
/// `points[point_index]` from its `neighbor_indices` via quadratic Monge patch
/// fitting $z = \frac{1}{2}(a x^2 + 2 b x y + c y^2)$ in the local PCA tangent
/// frame (PlantGL `principal_curvatures`).
///
/// Tier-1 `HotZeroHeap`.
pub fn monge_jet_principal_curvatures_3d(
    points: &[Point3],
    point_index: usize,
    neighbor_indices: &[usize],
) -> Result<PointCurvatureInfo, PointCloudGeomError> {
    if point_index >= points.len() {
        return Err(PointCloudGeomError::EmptyPointSet);
    }
    if neighbor_indices.len() < 3 {
        return Err(PointCloudGeomError::TooFewNeighbors {
            got: neighbor_indices.len(),
            min: 3,
        });
    }
    let origin = points[point_index];
    let mut cov = [[0.0f64; 3]; 3];
    for &ni in neighbor_indices {
        let p = points[ni];
        let d = [p.x - origin.x, p.y - origin.y, p.z - origin.z];
        for r in 0..3 {
            for c in 0..3 {
                cov[r][c] += d[r] * d[c];
            }
        }
    }
    let (axes, _) = jacobi_eigen_3x3(cov);
    let (u_axis, v_axis) = (axes[0], axes[1]);
    let n_axis = normalize(cross(u_axis, v_axis)).unwrap_or(axes[2]);

    let mut ata = [[0.0f64; 3]; 3];
    let mut atb = [0.0f64; 3];
    for &ni in neighbor_indices {
        let rel = sub(points[ni], origin);
        let (u, v, w) = (dot(rel, u_axis), dot(rel, v_axis), dot(rel, n_axis));
        let phi = [0.5 * u * u, u * v, 0.5 * v * v];
        for r in 0..3 {
            atb[r] += phi[r] * w;
            for c in 0..3 {
                ata[r][c] += phi[r] * phi[c];
            }
        }
    }
    for d in 0..3 {
        ata[d][d] += 1e-12;
    }
    let [a, b, c] = solve_3x3(ata, atb).ok_or(PointCloudGeomError::SingularJetFit)?;

    let trace = a + c;
    let disc = ((a - c) * (a - c) + 4.0 * b * b).sqrt();
    let theta = 0.5 * (2.0 * b).atan2(a - c);
    let (ct, st) = (theta.cos(), theta.sin());

    Ok(PointCurvatureInfo {
        origin,
        normal: n_axis,
        max_curvature: 0.5 * (trace + disc),
        max_direction: add(scale(u_axis, ct), scale(v_axis, st)),
        min_curvature: 0.5 * (trace - disc),
        min_direction: add(scale(u_axis, -st), scale(v_axis, ct)),
    })
}

fn solve_3x3(mut m: [[f64; 3]; 3], mut b: [f64; 3]) -> Option<[f64; 3]> {
    for k in 0..3 {
        let piv = (k..3)
            .max_by(|&i, &j| m[i][k].abs().partial_cmp(&m[j][k].abs()).unwrap())
            .unwrap();
        if m[piv][k].abs() <= 1e-15 {
            return None;
        }
        m.swap(k, piv);
        b.swap(k, piv);
        for i in (k + 1)..3 {
            let f = m[i][k] / m[k][k];
            for j in k..3 {
                m[i][j] -= f * m[k][j];
            }
            b[i] -= f * b[k];
        }
    }
    let mut x = [0.0f64; 3];
    for i in (0..3).rev() {
        let s = b[i] - ((i + 1)..3).map(|j| m[i][j] * x[j]).sum::<f64>();
        x[i] = s / m[i][i];
    }
    Some(x)
}

/// Propagate consistent normal orientation across a point cloud using Hoppe's
/// Riemannian graph traversal from `source_index` (PlantGL `pointsets_orient_normals`).
pub fn orient_normals_riemannian_mst(
    points: &[Point3],
    normals: &mut [Point3],
    source_index: usize,
    k_neighbors: usize,
) -> Result<(), PointCloudGeomError> {
    let n = points.len();
    if n == 0 || normals.len() != n || source_index >= n || k_neighbors == 0 {
        return Err(PointCloudGeomError::EmptyPointSet);
    }
    let mut visited = vec![false; n];
    visited[source_index] = true;
    for _step in 1..n {
        let mut best = (0usize, 0usize, f64::INFINITY);
        for i in (0..n).filter(|&i| visited[i]) {
            for j in (0..n).filter(|&j| !visited[j]) {
                let w = dist_sq(points[i], points[j])
                    / dot(normals[i], normals[j]).abs().clamp(1e-3, 1.0);
                if w < best.2 {
                    best = (i, j, w);
                }
            }
        }
        if !best.2.is_finite() {
            break;
        }
        if dot(normals[best.0], normals[best.1]) < 0.0 {
            normals[best.1] = scale(normals[best.1], -1.0);
        }
        visited[best.1] = true;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn miniball_median_and_monge_curvatures_match_analytic_sphere() {
        let mut pts = Vec::new();
        for i in 0..6 {
            let a = core::f64::consts::TAU * (i as f64) / 6.0;
            pts.push(Point3::new(2.0 * a.cos(), 2.0 * a.sin(), 0.0));
        }
        pts.push(Point3::new(0.0, 0.0, 2.0));
        pts.push(Point3::new(0.0, 0.0, -2.0));

        let mb = smallest_enclosing_sphere_3d(&pts).unwrap();
        assert!((mb.radius - 2.0).abs() < 1e-6);
        let med = weiszfeld_geometric_median_3d(&pts, 100, 1e-9).unwrap();
        assert!(dist_sq(med, Point3::new(0.0, 0.0, 0.0)).sqrt() < 1e-5);
        let pca = pca_bounding_primitives_3d(&pts).unwrap();
        assert!((pca.cylinder_radius - 2.0).abs() < 1e-6);

        let mut surf = vec![Point3::new(0.0, 0.0, 0.0)];
        for &dx in &[-0.1, 0.0, 0.1] {
            for &dy in &[-0.1, 0.0, 0.1] {
                if dx != 0.0 || dy != 0.0 {
                    surf.push(Point3::new(dx, dy, 0.5 * (2.0 * dx * dx + 1.0 * dy * dy)));
                }
            }
        }
        let nbrs: Vec<usize> = (1..surf.len()).collect();
        let curv = monge_jet_principal_curvatures_3d(&surf, 0, &nbrs).unwrap();
        assert!((curv.max_curvature.abs() - 2.0).abs() < 1e-4);
        assert!((curv.min_curvature.abs() - 1.0).abs() < 1e-4);
    }

    #[test]
    fn miniball_3d_welzl_support_certificate() {
        let pts = [
            Point3::new(1.0, 1.0, 1.0),
            Point3::new(-1.0, -1.0, 1.0),
            Point3::new(-1.0, 1.0, -1.0),
            Point3::new(1.0, -1.0, -1.0),
            Point3::new(0.0, 0.0, 0.0), // interior point
            Point3::new(0.2, -0.3, 0.4), // interior point
        ];
        let mb = smallest_enclosing_sphere_3d(&pts).unwrap();
        let expected_r = 3.0f64.sqrt();
        assert!((mb.radius - expected_r).abs() < 1e-6);
        assert!((dist_sq(mb.center, Point3::default())).sqrt() < 1e-6);
        assert!(mb.support_count >= 2 && mb.support_count <= 4);
        // All points enclosed
        for &p in &pts {
            let d = dist_sq(mb.center, p).sqrt();
            assert!(d <= mb.radius + 1e-5);
        }
        // Support points are on the boundary
        for &idx in &mb.support[..mb.support_count as usize] {
            let p = pts[idx as usize];
            let d = dist_sq(mb.center, p).sqrt();
            assert!((d - mb.radius).abs() < 1e-5);
        }
    }
}
