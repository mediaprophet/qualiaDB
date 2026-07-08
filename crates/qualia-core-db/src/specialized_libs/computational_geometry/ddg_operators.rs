//! P14 - Discrete differential geometry and surface operators.
//!
//! This module provides a compact DEC/FEM surface toolkit: oriented
//! simplicial boundaries, normals, angle-defect curvature, cotangent
//! Laplacian assembly, lumped mass, and small deterministic iterative solvers.

use bytemuck::{Pod, Zeroable};

use super::primitives::Point3;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
pub struct ChainTerm {
    pub dim: u8,
    pub vertex_count: u8,
    pub sign: i16,
    pub vertices: [u32; 4],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct SparseEntry {
    pub row: u32,
    pub col: u32,
    pub value: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DdgReport {
    pub residual: f64,
    pub iterations: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CurvatureSample {
    pub gaussian: f64,
    pub mean: f64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
pub struct HodgeSummary {
    pub vertex_count: u32,
    pub edge_count: u32,
    pub face_count: u32,
    pub euler_characteristic: i32,
    pub betti0: u32,
    pub betti1: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
pub struct DdgSectionHeader {
    pub magic: u32,
    pub version: u16,
    pub entry_count: u16,
    pub payload_bytes: u32,
    pub crc32c: u32,
}

pub const DDG_SECTION_MAGIC: u32 = u32::from_le_bytes(*b"QDDG");
pub const DDG_SECTION_VERSION: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DdgError {
    IndexOutOfBounds { element: usize, vertex: u32 },
    DegenerateElement { element: usize },
    OutputTooSmall { required: usize },
    DimensionMismatch,
    EmptyInput,
}

pub fn boundary_of_simplex(simplex: ChainTerm, out: &mut [ChainTerm]) -> Result<usize, DdgError> {
    if simplex.vertex_count == 0 {
        return Ok(0);
    }
    let n = simplex.vertex_count as usize;
    if out.len() < n {
        return Err(DdgError::OutputTooSmall { required: n });
    }
    for i in 0..n {
        let mut vertices = [0u32; 4];
        let mut k = 0usize;
        for j in 0..n {
            if i != j {
                vertices[k] = simplex.vertices[j];
                k += 1;
            }
        }
        let sign = if i % 2 == 0 {
            simplex.sign
        } else {
            -simplex.sign
        };
        out[i] = ChainTerm {
            dim: simplex.dim.saturating_sub(1),
            vertex_count: simplex.vertex_count - 1,
            sign,
            vertices,
        };
    }
    Ok(n)
}

pub fn boundary_of_boundary_is_zero(simplex: ChainTerm) -> bool {
    let mut first = [ChainTerm::zeroed(); 4];
    let Ok(n) = boundary_of_simplex(simplex, &mut first) else {
        return false;
    };
    let mut second: Vec<ChainTerm> = Vec::new();
    for term in &first[..n] {
        let mut tmp = [ChainTerm::zeroed(); 4];
        let Ok(m) = boundary_of_simplex(*term, &mut tmp) else {
            return false;
        };
        second.extend_from_slice(&tmp[..m]);
    }
    second.sort_by_key(chain_key);
    let mut i = 0usize;
    while i < second.len() {
        let key = chain_key(&second[i]);
        let mut sum = 0i32;
        while i < second.len() && chain_key(&second[i]) == key {
            sum += second[i].sign as i32;
            i += 1;
        }
        if sum != 0 {
            return false;
        }
    }
    true
}

pub fn face_vector_areas(
    vertices: &[Point3],
    triangles: &[[u32; 3]],
    out: &mut [Point3],
) -> Result<usize, DdgError> {
    if out.len() < triangles.len() {
        return Err(DdgError::OutputTooSmall {
            required: triangles.len(),
        });
    }
    for (i, tri) in triangles.iter().enumerate() {
        let [a, b, c] = fetch_tri(vertices, tri, i)?;
        let area_vec = scale(cross(sub(b, a), sub(c, a)), 0.5);
        if norm(area_vec) <= 0.0 {
            return Err(DdgError::DegenerateElement { element: i });
        }
        out[i] = area_vec;
    }
    Ok(triangles.len())
}

pub fn vertex_normals(
    vertices: &[Point3],
    triangles: &[[u32; 3]],
    out: &mut [Point3],
) -> Result<usize, DdgError> {
    if out.len() < vertices.len() {
        return Err(DdgError::OutputTooSmall {
            required: vertices.len(),
        });
    }
    out[..vertices.len()].fill(Point3::new(0.0, 0.0, 0.0));
    for (i, tri) in triangles.iter().enumerate() {
        let [a, b, c] = fetch_tri(vertices, tri, i)?;
        let area_vec = scale(cross(sub(b, a), sub(c, a)), 0.5);
        for &v in tri {
            out[v as usize] = add(out[v as usize], area_vec);
        }
    }
    for n in &mut out[..vertices.len()] {
        *n = normalize(*n);
    }
    Ok(vertices.len())
}

pub fn surface_area_gradient(
    vertices: &[Point3],
    triangles: &[[u32; 3]],
    out: &mut [Point3],
) -> Result<usize, DdgError> {
    if out.len() < vertices.len() {
        return Err(DdgError::OutputTooSmall {
            required: vertices.len(),
        });
    }
    let mut lap = vec![
        SparseEntry {
            row: 0,
            col: 0,
            value: 0.0,
        };
        triangles.len() * 9
    ];
    let mut mass = vec![0.0; vertices.len()];
    let nnz = cotangent_laplacian(vertices, triangles, &mut lap, &mut mass)?;
    out[..vertices.len()].fill(Point3::new(0.0, 0.0, 0.0));
    for e in &lap[..nnz] {
        let p = vertices[e.col as usize];
        out[e.row as usize] = add(out[e.row as usize], scale(p, e.value));
    }
    Ok(vertices.len())
}

pub fn curvature_angle_defect(
    vertices: &[Point3],
    triangles: &[[u32; 3]],
    out: &mut [CurvatureSample],
) -> Result<usize, DdgError> {
    if out.len() < vertices.len() {
        return Err(DdgError::OutputTooSmall {
            required: vertices.len(),
        });
    }
    let mut angle_sum = vec![0.0f64; vertices.len()];
    let mut area = vec![0.0f64; vertices.len()];
    for (ti, tri) in triangles.iter().enumerate() {
        let [a, b, c] = fetch_tri(vertices, tri, ti)?;
        let ar = 0.5 * norm(cross(sub(b, a), sub(c, a)));
        if ar <= 0.0 {
            return Err(DdgError::DegenerateElement { element: ti });
        }
        let angles = [
            angle_between(sub(b, a), sub(c, a)),
            angle_between(sub(c, b), sub(a, b)),
            angle_between(sub(a, c), sub(b, c)),
        ];
        for k in 0..3 {
            let vi = tri[k] as usize;
            angle_sum[vi] += angles[k];
            area[vi] += ar / 3.0;
        }
    }
    let mut grad = vec![Point3::new(0.0, 0.0, 0.0); vertices.len()];
    surface_area_gradient(vertices, triangles, &mut grad)?;
    for i in 0..vertices.len() {
        let gaussian = if area[i] > 0.0 {
            (core::f64::consts::TAU - angle_sum[i]) / area[i]
        } else {
            0.0
        };
        let mean = if area[i] > 0.0 {
            0.5 * norm(grad[i]) / area[i]
        } else {
            0.0
        };
        out[i] = CurvatureSample { gaussian, mean };
    }
    Ok(vertices.len())
}

pub fn cotangent_laplacian(
    vertices: &[Point3],
    triangles: &[[u32; 3]],
    out_entries: &mut [SparseEntry],
    out_lumped_mass: &mut [f64],
) -> Result<usize, DdgError> {
    if out_lumped_mass.len() < vertices.len() {
        return Err(DdgError::OutputTooSmall {
            required: vertices.len(),
        });
    }
    let required = triangles.len() * 9;
    if out_entries.len() < required {
        return Err(DdgError::OutputTooSmall { required });
    }
    out_lumped_mass[..vertices.len()].fill(0.0);
    let mut dense = vec![0.0f64; vertices.len() * vertices.len()];
    for (ti, tri) in triangles.iter().enumerate() {
        let [a, b, c] = fetch_tri(vertices, tri, ti)?;
        let area = 0.5 * norm(cross(sub(b, a), sub(c, a)));
        if area <= 0.0 {
            return Err(DdgError::DegenerateElement { element: ti });
        }
        for &v in tri {
            out_lumped_mass[v as usize] += area / 3.0;
        }
        let cot = [
            cotangent(sub(b, a), sub(c, a)),
            cotangent(sub(c, b), sub(a, b)),
            cotangent(sub(a, c), sub(b, c)),
        ];
        add_edge_weight(&mut dense, vertices.len(), tri[1], tri[2], 0.5 * cot[0]);
        add_edge_weight(&mut dense, vertices.len(), tri[2], tri[0], 0.5 * cot[1]);
        add_edge_weight(&mut dense, vertices.len(), tri[0], tri[1], 0.5 * cot[2]);
    }
    let mut nnz = 0usize;
    for r in 0..vertices.len() {
        let mut diag = 0.0;
        for c in 0..vertices.len() {
            if r == c {
                continue;
            }
            let w = dense[r * vertices.len() + c];
            if w != 0.0 {
                out_entries[nnz] = SparseEntry {
                    row: r as u32,
                    col: c as u32,
                    value: -w,
                };
                nnz += 1;
                diag += w;
            }
        }
        if diag != 0.0 {
            out_entries[nnz] = SparseEntry {
                row: r as u32,
                col: r as u32,
                value: diag,
            };
            nnz += 1;
        }
    }
    out_entries[..nnz].sort_by_key(|e| (e.row, e.col));
    Ok(nnz)
}

pub fn solve_poisson_jacobi(
    vertex_count: usize,
    laplacian: &[SparseEntry],
    rhs: &[f64],
    fixed: &[bool],
    values: &mut [f64],
    iterations: u32,
    tolerance: f64,
) -> Result<DdgReport, DdgError> {
    if rhs.len() < vertex_count || fixed.len() < vertex_count || values.len() < vertex_count {
        return Err(DdgError::DimensionMismatch);
    }
    let mut diag = vec![0.0f64; vertex_count];
    for e in laplacian {
        if e.row == e.col {
            diag[e.row as usize] = e.value;
        }
    }
    let mut next = values[..vertex_count].to_vec();
    let mut residual = f64::INFINITY;
    let mut ran = 0u32;
    for it in 0..iterations {
        residual = 0.0;
        for i in 0..vertex_count {
            if fixed[i] || diag[i] == 0.0 {
                next[i] = values[i];
                continue;
            }
            let mut off = 0.0;
            for e in laplacian {
                if e.row as usize == i && e.col as usize != i {
                    off += e.value * values[e.col as usize];
                }
            }
            next[i] = (rhs[i] - off) / diag[i];
            residual = residual.max((next[i] - values[i]).abs());
        }
        values[..vertex_count].copy_from_slice(&next[..vertex_count]);
        ran = it + 1;
        if residual <= tolerance {
            break;
        }
    }
    Ok(DdgReport {
        residual,
        iterations: ran,
    })
}

pub fn heat_step(
    vertex_count: usize,
    laplacian: &[SparseEntry],
    mass: &[f64],
    dt: f64,
    values: &mut [f64],
) -> Result<(), DdgError> {
    if mass.len() < vertex_count || values.len() < vertex_count || !(dt.is_finite() && dt >= 0.0) {
        return Err(DdgError::DimensionMismatch);
    }
    let old = values[..vertex_count].to_vec();
    for i in 0..vertex_count {
        if mass[i] == 0.0 {
            continue;
        }
        let mut lx = 0.0;
        for e in laplacian {
            if e.row as usize == i {
                lx += e.value * old[e.col as usize];
            }
        }
        values[i] = old[i] - dt * lx / mass[i];
    }
    Ok(())
}

pub fn mean_curvature_flow_step(
    vertices: &mut [Point3],
    triangles: &[[u32; 3]],
    dt: f64,
) -> Result<(), DdgError> {
    let n = vertices.len();
    let mut lap = vec![
        SparseEntry {
            row: 0,
            col: 0,
            value: 0.0,
        };
        triangles.len() * 9
    ];
    let mut mass = vec![0.0; n];
    let nnz = cotangent_laplacian(vertices, triangles, &mut lap, &mut mass)?;
    let old = vertices.to_vec();
    for i in 0..n {
        if mass[i] == 0.0 {
            continue;
        }
        let mut delta = Point3::new(0.0, 0.0, 0.0);
        for e in &lap[..nnz] {
            if e.row as usize == i {
                delta = add(delta, scale(old[e.col as usize], e.value));
            }
        }
        vertices[i] = sub(old[i], scale(delta, dt / mass[i]));
    }
    Ok(())
}

pub fn geodesic_distances_dijkstra(
    vertices: &[Point3],
    triangles: &[[u32; 3]],
    source: usize,
    out_distances: &mut [f64],
) -> Result<(), DdgError> {
    if vertices.is_empty() || source >= vertices.len() {
        return Err(DdgError::EmptyInput);
    }
    if out_distances.len() < vertices.len() {
        return Err(DdgError::OutputTooSmall {
            required: vertices.len(),
        });
    }
    for (i, distance) in out_distances.iter_mut().take(vertices.len()).enumerate() {
        *distance = if i == source { 0.0 } else { f64::INFINITY };
    }
    let mut settled = vec![false; vertices.len()];
    for _ in 0..vertices.len() {
        let mut best = None;
        for i in 0..vertices.len() {
            if !settled[i]
                && out_distances[i].is_finite()
                && best.map_or(true, |b| out_distances[i] < out_distances[b])
            {
                best = Some(i);
            }
        }
        let Some(u) = best else { break };
        settled[u] = true;
        for (element, tri) in triangles.iter().enumerate() {
            if let Some(&vertex) = tri.iter().find(|&&v| v as usize >= vertices.len()) {
                return Err(DdgError::IndexOutOfBounds { element, vertex });
            }
            for e in 0..3 {
                let a = tri[e] as usize;
                let b = tri[(e + 1) % 3] as usize;
                if a == u {
                    let nd = out_distances[u] + norm(sub(vertices[a], vertices[b]));
                    if nd < out_distances[b] {
                        out_distances[b] = nd;
                    }
                } else if b == u {
                    let nd = out_distances[u] + norm(sub(vertices[a], vertices[b]));
                    if nd < out_distances[a] {
                        out_distances[a] = nd;
                    }
                }
            }
        }
    }
    Ok(())
}

pub fn harmonic_parameterize_disk(
    vertices: &[Point3],
    triangles: &[[u32; 3]],
    boundary_loop: &[u32],
    out_uv: &mut [[f64; 2]],
    iterations: usize,
) -> Result<(), DdgError> {
    if vertices.is_empty() || boundary_loop.len() < 3 {
        return Err(DdgError::EmptyInput);
    }
    if out_uv.len() < vertices.len() {
        return Err(DdgError::OutputTooSmall {
            required: vertices.len(),
        });
    }
    let mut fixed = vec![false; vertices.len()];
    let mut perimeter = 0.0;
    for i in 0..boundary_loop.len() {
        let a = boundary_loop[i] as usize;
        let b = boundary_loop[(i + 1) % boundary_loop.len()] as usize;
        if a >= vertices.len() || b >= vertices.len() {
            return Err(DdgError::IndexOutOfBounds {
                element: i,
                vertex: boundary_loop[i],
            });
        }
        perimeter += norm(sub(vertices[a], vertices[b]));
    }
    if perimeter <= 0.0 {
        return Err(DdgError::DegenerateElement { element: 0 });
    }
    let mut travelled = 0.0;
    for i in 0..boundary_loop.len() {
        let idx = boundary_loop[i] as usize;
        fixed[idx] = true;
        let theta = core::f64::consts::TAU * travelled / perimeter;
        out_uv[idx] = [theta.cos(), theta.sin()];
        let next = boundary_loop[(i + 1) % boundary_loop.len()] as usize;
        travelled += norm(sub(vertices[idx], vertices[next]));
    }
    for (i, uv) in out_uv.iter_mut().take(vertices.len()).enumerate() {
        if !fixed[i] {
            *uv = [0.0, 0.0];
        }
    }
    let mut next = out_uv[..vertices.len()].to_vec();
    for _ in 0..iterations {
        for i in 0..vertices.len() {
            if fixed[i] {
                next[i] = out_uv[i];
                continue;
            }
            let mut sum = [0.0, 0.0];
            let mut count = 0usize;
            for (element, tri) in triangles.iter().enumerate() {
                if let Some(&vertex) = tri.iter().find(|&&v| v as usize >= vertices.len()) {
                    return Err(DdgError::IndexOutOfBounds { element, vertex });
                }
                if tri.iter().any(|&v| v as usize == i) {
                    for &v in tri {
                        let j = v as usize;
                        if j != i {
                            sum[0] += out_uv[j][0];
                            sum[1] += out_uv[j][1];
                            count += 1;
                        }
                    }
                }
            }
            if count > 0 {
                next[i] = [sum[0] / count as f64, sum[1] / count as f64];
            }
        }
        out_uv[..vertices.len()].copy_from_slice(&next);
    }
    Ok(())
}

pub fn hodge_decomposition_summary(
    vertex_count: usize,
    edge_count: usize,
    face_count: usize,
) -> HodgeSummary {
    let betti0 = if vertex_count == 0 { 0 } else { 1 };
    let euler = vertex_count as i64 - edge_count as i64 + face_count as i64;
    let betti1 = (betti0 as i64 - euler).max(0) as u32;
    HodgeSummary {
        vertex_count: vertex_count as u32,
        edge_count: edge_count as u32,
        face_count: face_count as u32,
        euler_characteristic: euler as i32,
        betti0,
        betti1,
    }
}

pub fn parallel_transport_angle(angle: f64, enclosed_curvature: f64) -> f64 {
    normalize_angle(angle + enclosed_curvature)
}

pub fn encode_ddg_operator_section(
    entries: &[SparseEntry],
    out: &mut [u8],
) -> Result<usize, DdgError> {
    let required = 16 + entries.len() * 16;
    if entries.len() > u16::MAX as usize || out.len() < required {
        return Err(DdgError::OutputTooSmall { required });
    }
    for (i, entry) in entries.iter().enumerate() {
        let base = 16 + i * 16;
        out[base..base + 4].copy_from_slice(&entry.row.to_le_bytes());
        out[base + 4..base + 8].copy_from_slice(&entry.col.to_le_bytes());
        out[base + 8..base + 16].copy_from_slice(&entry.value.to_le_bytes());
    }
    let crc = crc32c(&out[16..required]);
    out[0..4].copy_from_slice(&DDG_SECTION_MAGIC.to_le_bytes());
    out[4..6].copy_from_slice(&DDG_SECTION_VERSION.to_le_bytes());
    out[6..8].copy_from_slice(&(entries.len() as u16).to_le_bytes());
    out[8..12].copy_from_slice(&((entries.len() * 16) as u32).to_le_bytes());
    out[12..16].copy_from_slice(&crc.to_le_bytes());
    Ok(required)
}

fn chain_key(term: &ChainTerm) -> (u8, [u32; 4]) {
    let mut v = term.vertices;
    v[..term.vertex_count as usize].sort_unstable();
    (term.vertex_count, v)
}

fn fetch_tri(vertices: &[Point3], tri: &[u32; 3], element: usize) -> Result<[Point3; 3], DdgError> {
    let mut out = [Point3::new(0.0, 0.0, 0.0); 3];
    for (i, &vi) in tri.iter().enumerate() {
        out[i] = *vertices
            .get(vi as usize)
            .ok_or(DdgError::IndexOutOfBounds {
                element,
                vertex: vi,
            })?;
    }
    Ok(out)
}

fn add_edge_weight(dense: &mut [f64], n: usize, a: u32, b: u32, w: f64) {
    let a = a as usize;
    let b = b as usize;
    dense[a * n + b] += w;
    dense[b * n + a] += w;
}

#[inline]
fn cotangent(a: Point3, b: Point3) -> f64 {
    let cr = norm(cross(a, b));
    if cr == 0.0 {
        0.0
    } else {
        dot(a, b) / cr
    }
}

#[inline]
fn angle_between(a: Point3, b: Point3) -> f64 {
    let na = norm(a);
    let nb = norm(b);
    if na == 0.0 || nb == 0.0 {
        0.0
    } else {
        (dot(a, b) / (na * nb)).clamp(-1.0, 1.0).acos()
    }
}

#[inline]
fn normalize(a: Point3) -> Point3 {
    let n = norm(a);
    if n == 0.0 {
        Point3::new(0.0, 0.0, 0.0)
    } else {
        scale(a, 1.0 / n)
    }
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
fn norm(a: Point3) -> f64 {
    dot(a, a).sqrt()
}

#[inline]
fn normalize_angle(mut angle: f64) -> f64 {
    while angle > core::f64::consts::PI {
        angle -= core::f64::consts::TAU;
    }
    while angle <= -core::f64::consts::PI {
        angle += core::f64::consts::TAU;
    }
    angle
}

fn crc32c(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &b in bytes {
        crc ^= b as u32;
        for _ in 0..8 {
            let mask = 0u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0x82F6_3B78 & mask);
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square_mesh() -> (Vec<Point3>, Vec<[u32; 3]>) {
        (
            vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(1.0, 1.0, 0.0),
                Point3::new(0.0, 1.0, 0.0),
            ],
            vec![[0, 1, 2], [0, 2, 3]],
        )
    }

    #[test]
    fn boundary_of_boundary_zero_for_triangle_and_tet() {
        assert!(boundary_of_boundary_is_zero(ChainTerm {
            dim: 2,
            vertex_count: 3,
            sign: 1,
            vertices: [0, 1, 2, 0],
        }));
        assert!(boundary_of_boundary_is_zero(ChainTerm {
            dim: 3,
            vertex_count: 4,
            sign: 1,
            vertices: [0, 1, 2, 3],
        }));
    }

    #[test]
    fn face_and_vertex_normals_point_up_on_square() {
        let (v, t) = square_mesh();
        let mut faces = vec![Point3::new(0.0, 0.0, 0.0); t.len()];
        face_vector_areas(&v, &t, &mut faces).unwrap();
        assert!(faces.iter().all(|n| n.z > 0.0));
        let mut normals = vec![Point3::new(0.0, 0.0, 0.0); v.len()];
        vertex_normals(&v, &t, &mut normals).unwrap();
        assert!(normals.iter().all(|n| (n.z - 1.0).abs() < 1e-12));
    }

    #[test]
    fn cotangent_laplacian_has_zero_row_sum() {
        let (v, t) = square_mesh();
        let mut entries = vec![SparseEntry::zeroed(); t.len() * 9];
        let mut mass = vec![0.0; v.len()];
        let nnz = cotangent_laplacian(&v, &t, &mut entries, &mut mass).unwrap();
        for r in 0..v.len() {
            let row_sum: f64 = entries[..nnz]
                .iter()
                .filter(|e| e.row as usize == r)
                .map(|e| e.value)
                .sum();
            assert!(row_sum.abs() < 1e-12);
            assert!(mass[r] > 0.0);
        }
    }

    #[test]
    fn poisson_jacobi_preserves_fixed_values() {
        let (v, t) = square_mesh();
        let mut entries = vec![SparseEntry::zeroed(); t.len() * 9];
        let mut mass = vec![0.0; v.len()];
        let nnz = cotangent_laplacian(&v, &t, &mut entries, &mut mass).unwrap();
        let rhs = vec![0.0; v.len()];
        let fixed = vec![true, true, true, true];
        let mut values = vec![1.0, 2.0, 3.0, 4.0];
        solve_poisson_jacobi(
            v.len(),
            &entries[..nnz],
            &rhs,
            &fixed,
            &mut values,
            16,
            1e-12,
        )
        .unwrap();
        assert_eq!(values, vec![1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn heat_step_preserves_constant_field() {
        let (v, t) = square_mesh();
        let mut entries = vec![SparseEntry::zeroed(); t.len() * 9];
        let mut mass = vec![0.0; v.len()];
        let nnz = cotangent_laplacian(&v, &t, &mut entries, &mut mass).unwrap();
        let mut values = vec![7.0; v.len()];
        heat_step(v.len(), &entries[..nnz], &mass, 0.1, &mut values).unwrap();
        assert!(values.iter().all(|x| (*x - 7.0).abs() < 1e-12));
    }

    #[test]
    fn curvature_returns_finite_values() {
        let (v, t) = square_mesh();
        let mut curv = vec![
            CurvatureSample {
                gaussian: 0.0,
                mean: 0.0,
            };
            v.len()
        ];
        curvature_angle_defect(&v, &t, &mut curv).unwrap();
        assert!(curv
            .iter()
            .all(|c| c.gaussian.is_finite() && c.mean.is_finite()));
    }

    #[test]
    fn geodesic_and_parameterization_are_deterministic() {
        let (v, t) = square_mesh();
        let mut distances = vec![0.0; v.len()];
        geodesic_distances_dijkstra(&v, &t, 0, &mut distances).unwrap();
        assert!((distances[1] - 1.0).abs() < 1e-12);
        assert!((distances[2] - 2.0_f64.sqrt()).abs() < 1e-12);

        let mut uv = vec![[0.0, 0.0]; v.len()];
        harmonic_parameterize_disk(&v, &t, &[0, 1, 2, 3], &mut uv, 4).unwrap();
        assert!((uv[0][0] - 1.0).abs() < 1e-12);
        assert!(uv.iter().all(|p| p[0].is_finite() && p[1].is_finite()));
    }

    #[test]
    fn hodge_transport_and_section_encoding_are_canonical() {
        let summary = hodge_decomposition_summary(4, 5, 2);
        assert_eq!(summary.euler_characteristic, 1);
        assert_eq!(summary.betti1, 0);
        assert!(parallel_transport_angle(0.0, core::f64::consts::TAU).abs() < 1e-12);

        let entries = [SparseEntry {
            row: 1,
            col: 2,
            value: 3.5,
        }];
        let mut bytes = [0u8; 32];
        let n = encode_ddg_operator_section(&entries, &mut bytes).unwrap();
        assert_eq!(n, 32);
        assert_eq!(
            u32::from_le_bytes(bytes[0..4].try_into().unwrap()),
            DDG_SECTION_MAGIC
        );
        assert_eq!(u16::from_le_bytes(bytes[6..8].try_into().unwrap()), 1);
    }
}
