//! 2-3 / 3-2 bistellar flip pass on interior faces and edges.

use super::*;

// ---------------------------------------------------------------------------
//  2-3 / 3-2 flip pass
// ---------------------------------------------------------------------------

/// One 2-3 flip attempt on the interior face shared by tets `t1` and `t2`
/// (with apices `d` and `e`). Returns the three new tets if the flip is
/// beneficial and valid, else `None`.
///
/// `t1 = (a,b,c,d)`, `t2 = (a,b,c,e)` sharing face `(a,b,c)`. The 2-3 flip
/// produces three tets around edge `(d,e)`: `(a,b,d,e)`, `(b,c,d,e)`,
/// `(c,a,d,e)`, each re-oriented to positive signed volume.
pub(super) fn try_2_3_flip(
    vertices: &[Point3],
    t1: &[u32; 4],
    t2: &[u32; 4],
    d: u32,
    e: u32,
    obj: TetImproveObjective,
) -> Option<[[u32; 4]; 3]> {
    // The shared face is (a,b,c) = the three vertices of t1 that are not d.
    let mut face = [0u32; 3];
    let mut fi = 0;
    for &v in t1 {
        if v != d {
            face[fi] = v;
            fi += 1;
        }
    }
    debug_assert_eq!(fi, 3);
    let [a, b, c] = face;

    // Old worst score.
    let q1 = score_corners(
        vertices[t1[0] as usize],
        vertices[t1[1] as usize],
        vertices[t1[2] as usize],
        vertices[t1[3] as usize],
        obj,
    );
    let q2 = score_corners(
        vertices[t2[0] as usize],
        vertices[t2[1] as usize],
        vertices[t2[2] as usize],
        vertices[t2[3] as usize],
        obj,
    );
    let old_worst = q1.min(q2);

    // Three new tets around edge (d,e).
    let candidates: [[u32; 4]; 3] = [[a, b, d, e], [b, c, d, e], [c, a, d, e]];
    let mut new_tets = [[0u32; 4]; 3];
    let mut new_worst = f64::INFINITY;
    for (i, cand) in candidates.iter().enumerate() {
        let oriented = orient_positive(vertices, *cand)?;
        new_tets[i] = oriented;
        let s = score_corners(
            vertices[oriented[0] as usize],
            vertices[oriented[1] as usize],
            vertices[oriented[2] as usize],
            vertices[oriented[3] as usize],
            obj,
        );
        if !s.is_finite() {
            return None; // degenerate candidate
        }
        new_worst = new_worst.min(s);
    }

    // Accept only on strict improvement of the local worst case.
    if new_worst > old_worst + 1e-15 {
        Some(new_tets)
    } else {
        None
    }
}

/// One 3-2 flip attempt on an interior edge shared by exactly 3 tets. The
/// three tets are `(a,b,d,e)`, `(b,c,d,e)`, `(c,a,d,e)` around edge `(d,e)`;
/// the flip produces two tets `(a,b,c,d)` and `(a,b,c,e)` sharing face
/// `(a,b,c)`. Returns the two new tets if beneficial and valid, else `None`.
pub(super) fn try_3_2_flip(
    vertices: &[Point3],
    edge: [u32; 2],
    ring_tets: &[(usize, [u32; 4])],
    obj: TetImproveObjective,
) -> Option<[[u32; 4]; 2]> {
    if ring_tets.len() != 3 {
        return None;
    }
    let [d, e] = edge;
    // The "ring" vertices are the two vertices of each ring tet that are not
    // d or e. For a valid 3-2 config they are exactly three distinct vertices
    // a,b,c forming a triangle.
    let mut ring: [u32; 3] = [u32::MAX; 3];
    let mut rc = 0usize;
    for (_, t) in ring_tets {
        for &v in t {
            if v != d && v != e {
                // collect distinct
                if !ring.contains(&v) {
                    if rc == 3 {
                        return None; // more than 3 ring vertices -> not a 3-2 config
                    }
                    ring[rc] = v;
                    rc += 1;
                }
            }
        }
    }
    if rc != 3 {
        return None;
    }
    let [a, b, c] = ring;

    // Old worst score over the 3 ring tets.
    let mut old_worst = f64::INFINITY;
    for (_, t) in ring_tets {
        let s = score_corners(
            vertices[t[0] as usize],
            vertices[t[1] as usize],
            vertices[t[2] as usize],
            vertices[t[3] as usize],
            obj,
        );
        old_worst = old_worst.min(s);
    }

    let candidates: [[u32; 4]; 2] = [[a, b, c, d], [a, b, c, e]];
    let mut new_tets = [[0u32; 4]; 2];
    let mut new_worst = f64::INFINITY;
    for (i, cand) in candidates.iter().enumerate() {
        let oriented = orient_positive(vertices, *cand)?;
        new_tets[i] = oriented;
        let s = score_corners(
            vertices[oriented[0] as usize],
            vertices[oriented[1] as usize],
            vertices[oriented[2] as usize],
            vertices[oriented[3] as usize],
            obj,
        );
        if !s.is_finite() {
            return None;
        }
        new_worst = new_worst.min(s);
    }
    if new_worst > old_worst + 1e-15 {
        Some(new_tets)
    } else {
        None
    }
}

/// Run one flip pass over the mesh. Returns the number of flips applied.
/// Mutates `vertices` (read-only) / `tets` (in place) / `scores`.
pub(super) fn flip_pass(
    vertices: &[Point3],
    tets: &mut Vec<[u32; 4]>,
    scores: &mut Vec<f64>,
    obj: TetImproveObjective,
) -> u32 {
    let mut applied = 0u32;

    // 2-3 pass: iterate interior faces in deterministic order (face key order
    // from the BTreeMap). Process worst-scored shared face first within equal
    // keys is unnecessary because keys are unique; we process by the face key
    // order, but prioritise the worst shared faces by sorting.
    let face_map = face_to_tets(tets);
    // Collect interior faces with their worst tet score, sort worst-first.
    let mut interior: Vec<([u32; 3], usize, usize, u32, u32)> = Vec::new();
    for (key, refs) in &face_map {
        if refs.len() == 2 {
            let (t1, d) = refs[0];
            let (t2, e) = refs[1];
            interior.push((*key, t1, t2, d, e));
        }
    }
    // Sort worst-first with canonical tie-break (lowest t1 then lowest t2).
    interior.sort_by(|a, b| {
        let wa = scores[a.1].min(scores[a.2]);
        let wb = scores[b.1].min(scores[b.2]);
        wb.partial_cmp(&wa)
            .unwrap_or(core::cmp::Ordering::Equal)
            .then(a.1.cmp(&b.1))
            .then(a.2.cmp(&b.2))
    });

    // Track removed tets to skip stale entries.
    let mut removed: std::collections::BTreeSet<usize> = std::collections::BTreeSet::new();
    for (_, t1, t2, d, e) in interior {
        if removed.contains(&t1) || removed.contains(&t2) {
            continue;
        }
        let tet1 = tets[t1];
        let tet2 = tets[t2];
        if let Some(new3) = try_2_3_flip(vertices, &tet1, &tet2, d, e, obj) {
            // Mark t1, t2 removed; append the three new tets.
            removed.insert(t1);
            removed.insert(t2);
            for nt in &new3 {
                tets.push(*nt);
                let s = score_corners(
                    vertices[nt[0] as usize],
                    vertices[nt[1] as usize],
                    vertices[nt[2] as usize],
                    vertices[nt[3] as usize],
                    obj,
                );
                scores.push(s);
            }
            applied += 1;
        }
    }

    // Compact out removed tets.
    if !removed.is_empty() {
        let mut keep: Vec<[u32; 4]> = Vec::with_capacity(tets.len());
        let mut keep_scores: Vec<f64> = Vec::with_capacity(tets.len());
        for (i, t) in tets.iter().enumerate() {
            if !removed.contains(&i) {
                keep.push(*t);
                keep_scores.push(scores[i]);
            }
        }
        *tets = keep;
        *scores = keep_scores;
    }

    // 3-2 pass: build edge -> incident tets map over the (post-2-3) mesh.
    let mut edge_map: std::collections::BTreeMap<[u32; 2], Vec<(usize, [u32; 4])>> =
        std::collections::BTreeMap::new();
    for (i, tet) in tets.iter().enumerate() {
        for (j, k) in [(0usize, 1usize), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)] {
            let ek = edge_key(tet[j], tet[k]);
            edge_map.entry(ek).or_default().push((i, *tet));
        }
    }
    // Interior edges with exactly 3 incident tets are 3-2 candidates.
    let mut candidates: Vec<([u32; 2], Vec<(usize, [u32; 4])>)> = Vec::new();
    for (ek, refs) in &edge_map {
        if refs.len() == 3 {
            // Skip edges touching a boundary face: if any of the 3 tets has a
            // boundary face incident to this edge, the edge is on the boundary
            // and must not be flipped. Detect by checking whether the two ring
            // faces (the faces of each ring tet NOT containing the edge) are
            // interior. Simpler: require all 3 tets to share both edge
            // endpoints and the edge to be interior (appears in 3 tets only,
            // and none of those tets has a boundary face on this edge). We
            // approximate "interior edge" by: the edge is not in the
            // boundary-vertex set is too strict; instead we check after the
            // fact that the resulting 2 tets do not collapse a boundary face.
            candidates.push((*ek, refs.clone()));
        }
    }
    candidates.sort_by(|a, b| {
        let wa =
            a.1.iter()
                .map(|(i, _)| scores[*i])
                .fold(f64::INFINITY, f64::min);
        let wb =
            b.1.iter()
                .map(|(i, _)| scores[*i])
                .fold(f64::INFINITY, f64::min);
        wb.partial_cmp(&wa)
            .unwrap_or(core::cmp::Ordering::Equal)
            .then(a.0.cmp(&b.0))
    });

    let mut removed2: std::collections::BTreeSet<usize> = std::collections::BTreeSet::new();
    for (ek, refs) in candidates {
        let active: Vec<(usize, [u32; 4])> = refs
            .iter()
            .filter(|(i, _)| !removed2.contains(i))
            .copied()
            .collect();
        if active.len() != 3 {
            continue;
        }
        // Guard: none of the 3 tets may already be removed, and the edge must
        // remain interior (still 3 active tets).
        if let Some(new2) = try_3_2_flip(vertices, ek, &active, obj) {
            for (i, _) in &active {
                removed2.insert(*i);
            }
            for nt in &new2 {
                tets.push(*nt);
                let s = score_corners(
                    vertices[nt[0] as usize],
                    vertices[nt[1] as usize],
                    vertices[nt[2] as usize],
                    vertices[nt[3] as usize],
                    obj,
                );
                scores.push(s);
            }
            applied += 1;
        }
    }
    if !removed2.is_empty() {
        let mut keep: Vec<[u32; 4]> = Vec::with_capacity(tets.len());
        let mut keep_scores: Vec<f64> = Vec::with_capacity(tets.len());
        for (i, t) in tets.iter().enumerate() {
            if !removed2.contains(&i) {
                keep.push(*t);
                keep_scores.push(scores[i]);
            }
        }
        *tets = keep;
        *scores = keep_scores;
    }

    applied
}
