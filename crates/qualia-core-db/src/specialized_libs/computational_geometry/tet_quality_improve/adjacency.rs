//! Mesh adjacency, boundary detection, per-tet scoring, and vertex-neighbourhood
//! queries shared by the flip / smooth / insert / exude passes.

use super::*;

// ---------------------------------------------------------------------------
//  Mesh adjacency
// ---------------------------------------------------------------------------

/// A face key: the three vertex indices sorted ascending. Two tets share an
/// interior face iff they produce the same key.
#[inline]
pub(super) fn face_key(a: u32, b: u32, c: u32) -> [u32; 3] {
    let mut k = [a, b, c];
    k.sort_unstable();
    k
}

/// The four faces of a tet `[v0,v1,v2,v3]` as sorted keys.
#[inline]
pub(super) fn tet_faces(tet: &[u32; 4]) -> [[u32; 3]; 4] {
    [
        face_key(tet[0], tet[1], tet[2]),
        face_key(tet[0], tet[1], tet[3]),
        face_key(tet[0], tet[2], tet[3]),
        face_key(tet[1], tet[2], tet[3]),
    ]
}

/// An edge key: two vertex indices sorted ascending.
#[inline]
pub(super) fn edge_key(a: u32, b: u32) -> [u32; 2] {
    if a < b {
        [a, b]
    } else {
        [b, a]
    }
}

/// Build the boundary-vertex set: a vertex is on the boundary iff it is
/// incident to a face that appears in exactly one tet.
pub(super) fn boundary_vertices(tets: &[[u32; 4]]) -> std::collections::BTreeSet<u32> {
    use std::collections::BTreeMap;
    let mut face_count: BTreeMap<[u32; 3], u32> = BTreeMap::new();
    for tet in tets {
        for f in tet_faces(tet) {
            *face_count.entry(f).or_insert(0) += 1;
        }
    }
    let mut bv = std::collections::BTreeSet::new();
    for (f, c) in &face_count {
        if *c == 1 {
            for &v in f {
                bv.insert(v);
            }
        }
    }
    bv
}

/// Per-tet quality scores.
pub(super) fn score_all(
    vertices: &[Point3],
    tets: &[[u32; 4]],
    obj: TetImproveObjective,
) -> Result<Vec<f64>, TetImproveError> {
    let mut out = Vec::with_capacity(tets.len());
    for (i, tet) in tets.iter().enumerate() {
        let a = *vertices
            .get(tet[0] as usize)
            .ok_or(TetImproveError::IndexOutOfBounds {
                tet: i,
                vertex: tet[0],
            })?;
        let b = *vertices
            .get(tet[1] as usize)
            .ok_or(TetImproveError::IndexOutOfBounds {
                tet: i,
                vertex: tet[1],
            })?;
        let c = *vertices
            .get(tet[2] as usize)
            .ok_or(TetImproveError::IndexOutOfBounds {
                tet: i,
                vertex: tet[2],
            })?;
        let d = *vertices
            .get(tet[3] as usize)
            .ok_or(TetImproveError::IndexOutOfBounds {
                tet: i,
                vertex: tet[3],
            })?;
        for v in [a, b, c, d] {
            if !v.x.is_finite() || !v.y.is_finite() || !v.z.is_finite() {
                return Err(TetImproveError::NonFiniteCoordinate { index: tet[0] });
            }
        }
        out.push(score_corners(a, b, c, d, obj));
    }
    Ok(out)
}

/// Build a face -> list of (tet_index, opposite_vertex_local_index) map.
pub(super) fn face_to_tets(
    tets: &[[u32; 4]],
) -> std::collections::BTreeMap<[u32; 3], Vec<(usize, u32)>> {
    let mut map: std::collections::BTreeMap<[u32; 3], Vec<(usize, u32)>> =
        std::collections::BTreeMap::new();
    for (ti, tet) in tets.iter().enumerate() {
        // For each face, the opposite vertex is the one not in the face.
        for (fi, fk) in tet_faces(tet).iter().enumerate() {
            map.entry(*fk).or_default().push((ti, tet[fi]));
        }
    }
    map
}

/// Orient a tet `[a,b,c,d]` so its signed volume is positive. Returns the
/// re-ordered vertex indices.
#[inline]
pub(super) fn orient_positive(vertices: &[Point3], mut v: [u32; 4]) -> Option<[u32; 4]> {
    let a = vertices[v[0] as usize];
    let b = vertices[v[1] as usize];
    let c = vertices[v[2] as usize];
    let d = vertices[v[3] as usize];
    let sv = signed_volume(a, b, c, d);
    if sv > 0.0 {
        Some(v)
    } else if sv < 0.0 {
        v.swap(0, 1);
        Some(v)
    } else {
        None // degenerate
    }
}

/// Collect the tet indices incident to vertex `v`.
pub(super) fn incident_tets(tets: &[[u32; 4]], v: u32) -> Vec<usize> {
    let mut out = Vec::new();
    for (i, tet) in tets.iter().enumerate() {
        if tet.contains(&v) {
            out.push(i);
        }
    }
    out
}

/// Average incident edge length around vertex `v` (used to scale
/// perturbations). Returns a small floor if no edges.
pub(super) fn avg_incident_edge(vertices: &[Point3], tets: &[[u32; 4]], v: u32) -> f64 {
    let mut sum = 0.0f64;
    let mut n = 0u32;
    let mut seen: std::collections::BTreeSet<[u32; 2]> = std::collections::BTreeSet::new();
    for tet in tets {
        if !tet.contains(&v) {
            continue;
        }
        for (j, k) in [(0usize, 1usize), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)] {
            if tet[j] == v || tet[k] == v {
                let ek = edge_key(tet[j], tet[k]);
                if seen.insert(ek) {
                    sum += norm(sub(vertices[tet[j] as usize], vertices[tet[k] as usize]));
                    n += 1;
                }
            }
        }
    }
    if n == 0 {
        1e-6
    } else {
        (sum / n as f64).max(1e-12)
    }
}

/// One-ring neighbour vertices of `v`.
pub(super) fn one_ring(tets: &[[u32; 4]], v: u32) -> Vec<u32> {
    let mut ring: Vec<u32> = Vec::new();
    for tet in tets {
        if !tet.contains(&v) {
            continue;
        }
        for &u in tet {
            if u != v && !ring.contains(&u) {
                ring.push(u);
            }
        }
    }
    ring
}

/// Substitute position `pos` for vertex `v` in the tet corners (a,b,c,d),
/// wherever `v` appears in `tet`.
#[inline]
pub(super) fn substitute_v(
    a: Point3,
    b: Point3,
    c: Point3,
    d: Point3,
    tet: &[u32; 4],
    v: u32,
    pos: Point3,
) -> (Point3, Point3, Point3, Point3) {
    let mut arr = [a, b, c, d];
    for i in 0..4 {
        if tet[i] == v {
            arr[i] = pos;
        }
    }
    (arr[0], arr[1], arr[2], arr[3])
}
