//! Optimisation-based vertex smoothing pass and its shared deterministic probe
//! direction set (also used by the exude pass).

use super::*;

// ---------------------------------------------------------------------------
//  Smooth pass
// ---------------------------------------------------------------------------

/// The 26 unit-cube neighbour directions (6 axis + 12 face-diagonal + 8
/// body-diagonal), in fixed deterministic order. Each is unit-ish (length 1
/// for axis, sqrt(2) for face-diagonal, sqrt(3) for body-diagonal); the
/// caller scales by the perturbation magnitude.
pub(super) fn probe_directions() -> &'static [Point3] {
    const NORM_SQRT2: f64 = core::f64::consts::SQRT_2;
    const NORM_SQRT3: f64 = 1.7320508075688772;
    static DIRS: [Point3; 26] = [
        // 6 axis directions
        Point3::new(1.0, 0.0, 0.0),
        Point3::new(-1.0, 0.0, 0.0),
        Point3::new(0.0, 1.0, 0.0),
        Point3::new(0.0, -1.0, 0.0),
        Point3::new(0.0, 0.0, 1.0),
        Point3::new(0.0, 0.0, -1.0),
        // 12 face-diagonals (normalised)
        Point3::new(1.0 / NORM_SQRT2, 1.0 / NORM_SQRT2, 0.0),
        Point3::new(1.0 / NORM_SQRT2, -1.0 / NORM_SQRT2, 0.0),
        Point3::new(-1.0 / NORM_SQRT2, 1.0 / NORM_SQRT2, 0.0),
        Point3::new(-1.0 / NORM_SQRT2, -1.0 / NORM_SQRT2, 0.0),
        Point3::new(1.0 / NORM_SQRT2, 0.0, 1.0 / NORM_SQRT2),
        Point3::new(1.0 / NORM_SQRT2, 0.0, -1.0 / NORM_SQRT2),
        Point3::new(-1.0 / NORM_SQRT2, 0.0, 1.0 / NORM_SQRT2),
        Point3::new(-1.0 / NORM_SQRT2, 0.0, -1.0 / NORM_SQRT2),
        Point3::new(0.0, 1.0 / NORM_SQRT2, 1.0 / NORM_SQRT2),
        Point3::new(0.0, 1.0 / NORM_SQRT2, -1.0 / NORM_SQRT2),
        Point3::new(0.0, -1.0 / NORM_SQRT2, 1.0 / NORM_SQRT2),
        Point3::new(0.0, -1.0 / NORM_SQRT2, -1.0 / NORM_SQRT2),
        // 8 body-diagonals (normalised)
        Point3::new(1.0 / NORM_SQRT3, 1.0 / NORM_SQRT3, 1.0 / NORM_SQRT3),
        Point3::new(1.0 / NORM_SQRT3, 1.0 / NORM_SQRT3, -1.0 / NORM_SQRT3),
        Point3::new(1.0 / NORM_SQRT3, -1.0 / NORM_SQRT3, 1.0 / NORM_SQRT3),
        Point3::new(1.0 / NORM_SQRT3, -1.0 / NORM_SQRT3, -1.0 / NORM_SQRT3),
        Point3::new(-1.0 / NORM_SQRT3, 1.0 / NORM_SQRT3, 1.0 / NORM_SQRT3),
        Point3::new(-1.0 / NORM_SQRT3, 1.0 / NORM_SQRT3, -1.0 / NORM_SQRT3),
        Point3::new(-1.0 / NORM_SQRT3, -1.0 / NORM_SQRT3, 1.0 / NORM_SQRT3),
        Point3::new(-1.0 / NORM_SQRT3, -1.0 / NORM_SQRT3, -1.0 / NORM_SQRT3),
    ];
    &DIRS
}

/// Run one smooth pass. Returns the number of vertices moved.
pub(super) fn smooth_pass(
    vertices: &mut Vec<Point3>,
    tets: &[[u32; 4]],
    scores: &mut Vec<f64>,
    fixed: &std::collections::BTreeSet<u32>,
    obj: TetImproveObjective,
    probe_count: u8,
    perturb_fractions: &[f64],
) -> u32 {
    let mut applied = 0u32;
    let probes = probe_directions();
    let probe_n = (probe_count as usize).min(probes.len());

    // Process interior vertices in ascending index order (deterministic).
    for v in 0..(vertices.len() as u32) {
        if fixed.contains(&v) {
            continue;
        }
        let incident = incident_tets(tets, v);
        if incident.is_empty() {
            continue;
        }
        // Current worst score over incident tets.
        let cur_worst = incident
            .iter()
            .map(|&i| scores[i])
            .fold(f64::INFINITY, f64::min);
        if !cur_worst.is_finite() {
            continue;
        }

        let ring = one_ring(tets, v);
        if ring.is_empty() {
            continue;
        }
        let h = avg_incident_edge(vertices, tets, v);

        // Candidate positions, evaluated in deterministic order.
        // (1) Laplacian centroid of one-ring.
        let mut centroid = Point3::new(0.0, 0.0, 0.0);
        for &u in &ring {
            centroid = add(centroid, vertices[u as usize]);
        }
        centroid = scale(centroid, 1.0 / ring.len() as f64);

        // (2) Volume-weighted centroid of incident tets' centroids.
        let mut vol_weighted = Point3::new(0.0, 0.0, 0.0);
        let mut total_w = 0.0f64;
        for &i in &incident {
            let t = tets[i];
            let a = vertices[t[0] as usize];
            let b = vertices[t[1] as usize];
            let c = vertices[t[2] as usize];
            let d = vertices[t[3] as usize];
            let tc = Point3::new(
                (a.x + b.x + c.x + d.x) / 4.0,
                (a.y + b.y + c.y + d.y) / 4.0,
                (a.z + b.z + c.z + d.z) / 4.0,
            );
            let vol = signed_volume(a, b, c, d).abs().max(1e-30);
            vol_weighted = add(vol_weighted, scale(tc, vol));
            total_w += vol;
        }
        if total_w > 0.0 {
            vol_weighted = scale(vol_weighted, 1.0 / total_w);
        }

        let cur = vertices[v as usize];
        let mut best_pos = cur;
        let mut best_worst = cur_worst;

        // Evaluate candidates: centroid, vol-weighted, then probe directions
        // at each perturbation magnitude.
        let evaluate = |pos: Point3, best_pos: &mut Point3, best_worst: &mut f64| {
            // Check all incident tets remain valid + compute new worst.
            let mut new_worst = f64::INFINITY;
            for &i in &incident {
                let t = tets[i];
                let a = vertices[t[0] as usize];
                let b = vertices[t[1] as usize];
                let c = vertices[t[2] as usize];
                let d = vertices[t[3] as usize];
                // Substitute v wherever it appears.
                let (a, b, c, d) = substitute_v(a, b, c, d, &t, v, pos);
                let sv = signed_volume(a, b, c, d);
                if sv <= 0.0 {
                    return; // invalid candidate
                }
                let s = score_corners(a, b, c, d, obj);
                new_worst = new_worst.min(s);
            }
            if new_worst > *best_worst + 1e-15 {
                *best_worst = new_worst;
                *best_pos = pos;
            }
        };

        evaluate(centroid, &mut best_pos, &mut best_worst);
        evaluate(vol_weighted, &mut best_pos, &mut best_worst);
        for pi in 0..probe_n {
            let dir = probes[pi];
            for &frac in perturb_fractions {
                let pos = add(cur, scale(dir, h * frac));
                evaluate(pos, &mut best_pos, &mut best_worst);
            }
        }

        if best_pos != cur {
            vertices[v as usize] = best_pos;
            // Recompute scores for incident tets.
            for &i in &incident {
                let t = tets[i];
                let a = vertices[t[0] as usize];
                let b = vertices[t[1] as usize];
                let c = vertices[t[2] as usize];
                let d = vertices[t[3] as usize];
                scores[i] = score_corners(a, b, c, d, obj);
            }
            applied += 1;
        }
    }
    applied
}
