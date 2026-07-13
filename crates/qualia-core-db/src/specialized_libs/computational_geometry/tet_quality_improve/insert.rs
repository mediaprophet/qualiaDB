//! Delaunay-style cavity / star insertion pass (Steiner-point refinement).

use super::*;

// ---------------------------------------------------------------------------
//  Insert pass (cavity / star insertion)
// ---------------------------------------------------------------------------

/// Run one insert pass: locate the worst tet, compute its circumcenter, build
/// the Delaunay cavity, check star-shapedness, and replace the cavity with a
/// star of new tets. Returns the number of Steiner points inserted (0 or 1
/// per call), or an error if the Steiner cap is reached.
pub(super) fn insert_pass(
    vertices: &mut Vec<Point3>,
    tets: &mut Vec<[u32; 4]>,
    scores: &mut Vec<f64>,
    obj: TetImproveObjective,
    steiner_count: &mut u32,
    max_steiner: u32,
) -> Result<u32, TetImproveError> {
    if *steiner_count >= max_steiner {
        return Ok(0);
    }
    if tets.is_empty() {
        return Ok(0);
    }

    // Worst tet (lowest score), canonical tie-break (lowest index).
    let (worst_idx, _) = scores
        .iter()
        .enumerate()
        .min_by(|(i, a), (j, b)| {
            a.partial_cmp(b)
                .unwrap_or(core::cmp::Ordering::Equal)
                .then(i.cmp(j))
        })
        .expect("non-empty tets");

    let wt = tets[worst_idx];
    let a = vertices[wt[0] as usize];
    let b = vertices[wt[1] as usize];
    let c = vertices[wt[2] as usize];
    let d = vertices[wt[3] as usize];
    let cc = match circumcenter(a, b, c, d) {
        Some(p) => p,
        None => return Ok(0), // degenerate worst tet; skip
    };

    // Build face -> tet adjacency for cavity flood-fill.
    let face_map = face_to_tets(tets);

    // Flood-fill the cavity: start from worst_idx, include any face-neighbour
    // tet whose circumsphere contains the new point (cc). A tet is in the
    // cavity iff its circumsphere (squared radius from cc) >= the squared
    // distance from cc to the tet's circumcenter... wait: the standard Delaunay
    // cavity test is: a tet is in the cavity iff the new point lies inside (or
    // on) its circumsphere. We use: `dist2(cc, tet_circumcenter) <=
    // tet_circumradius_sq` with a small epsilon for "on the boundary" inclusion.
    let mut cavity: std::collections::BTreeSet<usize> = std::collections::BTreeSet::new();
    let mut stack: Vec<usize> = vec![worst_idx];
    while let Some(ti) = stack.pop() {
        if cavity.contains(&ti) {
            continue;
        }
        let t = tets[ti];
        let ta = vertices[t[0] as usize];
        let tb = vertices[t[1] as usize];
        let tc = vertices[t[2] as usize];
        let td = vertices[t[3] as usize];
        // In-sphere test: is cc inside the circumsphere of (ta,tb,tc,td)?
        let cc_tet = circumcenter(ta, tb, tc, td);
        let inside = match cc_tet {
            Some(c2) => {
                let r2 = {
                    let dx = c2.x - ta.x;
                    let dy = c2.y - ta.y;
                    let dz = c2.z - ta.z;
                    dx * dx + dy * dy + dz * dz
                };
                let d2 = {
                    let dx = c2.x - cc.x;
                    let dy = c2.y - cc.y;
                    let dz = c2.z - cc.z;
                    dx * dx + dy * dy + dz * dz
                };
                // Include on a small epsilon to grow the cavity past exact-sphere.
                d2 <= r2 + 1e-12 * r2.max(1.0)
            }
            None => true, // degenerate tet -> include in cavity (it will be removed)
        };
        if !inside {
            continue;
        }
        cavity.insert(ti);
        // Push face-neighbours.
        for f in tet_faces(&t) {
            if let Some(refs) = face_map.get(&f) {
                for (nti, _) in refs {
                    if !cavity.contains(nti) {
                        stack.push(*nti);
                    }
                }
            }
        }
    }

    if cavity.is_empty() {
        return Ok(0);
    }

    // Boundary triangles of the cavity: faces of cavity tets that are NOT
    // shared with another cavity tet. Each boundary triangle, joined to cc,
    // forms a new tet.
    let mut boundary_faces: Vec<[u32; 3]> = Vec::new();
    for &ti in &cavity {
        let t = tets[ti];
        // For each face, check whether the face's other tet is in the cavity.
        for (fi, fk) in tet_faces(&t).iter().enumerate() {
            let other_in_cavity = match face_map.get(fk) {
                Some(refs) => refs
                    .iter()
                    .filter(|(nti, _)| *nti != ti)
                    .any(|(nti, _)| cavity.contains(nti)),
                None => false,
            };
            if !other_in_cavity {
                // Recover the oriented face (outward from this tet). Face `fi`
                // excludes vertex `t[fi]`.
                let opp = t[fi];
                let mut face = [0u32; 3];
                let mut k = 0;
                for &v in &t {
                    if v != opp {
                        face[k] = v;
                        k += 1;
                    }
                }
                boundary_faces.push(face);
            }
        }
    }

    if boundary_faces.is_empty() {
        return Ok(0);
    }

    // Star-shapedness + new tets: for each boundary face (oriented outward
    // from the cavity tet), the new tet is (face... , cc) with the face
    // winding reversed so that cc is on the positive side. We orient each new
    // tet to positive signed volume explicitly.
    let new_v_idx = vertices.len() as u32;
    let mut new_tets: Vec<[u32; 4]> = Vec::with_capacity(boundary_faces.len());
    let mut new_worst = f64::INFINITY;
    let mut old_worst = f64::INFINITY;
    for &ti in &cavity {
        old_worst = old_worst.min(scores[ti]);
    }
    for f in &boundary_faces {
        // Candidate tet: (f0, f1, f2, cc). Orient positive.
        let cand = [f[0], f[1], f[2], new_v_idx];
        // Temporarily push cc so orient_positive can read it.
        vertices.push(cc);
        let oriented = orient_positive(vertices, cand);
        vertices.pop();
        let oriented = match oriented {
            Some(o) => o,
            None => return Ok(0), // not star-shaped here -> abort insert
        };
        // Validate with cc actually present.
        vertices.push(cc);
        let a = vertices[oriented[0] as usize];
        let b = vertices[oriented[1] as usize];
        let c = vertices[oriented[2] as usize];
        let d = vertices[oriented[3] as usize];
        let sv = signed_volume(a, b, c, d);
        let s = score_corners(a, b, c, d, obj);
        vertices.pop();
        if sv <= 0.0 || !s.is_finite() {
            return Ok(0); // not star-shaped / degenerate -> abort
        }
        new_tets.push(oriented);
        new_worst = new_worst.min(s);
    }

    // Monotonic improvement: accept only if new worst > old worst.
    if new_worst <= old_worst + 1e-15 {
        return Ok(0);
    }

    // Apply: append cc, remove cavity tets, append new tets.
    vertices.push(cc);
    let mut keep: Vec<[u32; 4]> = Vec::with_capacity(tets.len() - cavity.len() + new_tets.len());
    let mut keep_scores: Vec<f64> = Vec::with_capacity(keep.capacity());
    for (i, t) in tets.iter().enumerate() {
        if !cavity.contains(&i) {
            keep.push(*t);
            keep_scores.push(scores[i]);
        }
    }
    for nt in &new_tets {
        keep.push(*nt);
        let a = vertices[nt[0] as usize];
        let b = vertices[nt[1] as usize];
        let c = vertices[nt[2] as usize];
        let d = vertices[nt[3] as usize];
        keep_scores.push(score_corners(a, b, c, d, obj));
    }
    *tets = keep;
    *scores = keep_scores;
    *steiner_count += 1;
    Ok(1)
}
