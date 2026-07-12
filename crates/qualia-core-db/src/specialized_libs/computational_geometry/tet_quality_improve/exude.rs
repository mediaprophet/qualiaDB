//! Sliver-exudation pass by local vertex perturbation.

use super::*;

// ---------------------------------------------------------------------------
//  Exude pass (sliver exudation by local perturbation)
// ---------------------------------------------------------------------------

/// Run one exude pass: for each sliver tet, perturb its interior vertices over
/// the deterministic probe set to remove the sliver without creating new
/// slivers or inversions in the one-ring. Returns the number of perturbations
/// applied.
pub(super) fn exude_pass(
    vertices: &mut Vec<Point3>,
    tets: &[[u32; 4]],
    scores: &mut Vec<f64>,
    fixed: &std::collections::BTreeSet<u32>,
    obj: TetImproveObjective,
    sliver_min_dihedral_rad: f64,
    probe_count: u8,
    perturb_fractions: &[f64],
) -> u32 {
    let mut applied = 0u32;
    let probes = probe_directions();
    let probe_n = (probe_count as usize).min(probes.len());

    // Identify slivers (min_dihedral < threshold). Sort worst-first with
    // canonical tie-break (lowest tet index).
    let mut slivers: Vec<(usize, f64)> = Vec::new();
    for (i, tet) in tets.iter().enumerate() {
        let a = vertices[tet[0] as usize];
        let b = vertices[tet[1] as usize];
        let c = vertices[tet[2] as usize];
        let d = vertices[tet[3] as usize];
        let q = tet_quality_points(a, b, c, d);
        if q.valid && q.min_dihedral < sliver_min_dihedral_rad {
            slivers.push((i, q.min_dihedral));
        }
    }
    slivers.sort_by(|a, b| {
        a.1.partial_cmp(&b.1)
            .unwrap_or(core::cmp::Ordering::Equal)
            .then(a.0.cmp(&b.0))
    });

    for (sliver_idx, _) in slivers {
        // Re-check: the sliver may have been fixed by an earlier perturbation
        // in this same pass (its score changed).
        if scores[sliver_idx] >= sliver_min_dihedral_rad {
            continue;
        }
        let tet = tets[sliver_idx];
        // Try perturbing each interior (non-fixed) vertex of the sliver.
        'outer: for &v in &tet {
            if fixed.contains(&v) {
                continue;
            }
            let incident = incident_tets(tets, v);
            let h = avg_incident_edge(vertices, tets, v);
            let cur = vertices[v as usize];

            // Current worst score over the incident tets.
            let cur_worst = incident
                .iter()
                .map(|&i| scores[i])
                .fold(f64::INFINITY, f64::min);

            let mut best_pos = cur;
            let mut best_worst = cur_worst;
            let mut best_fixes_sliver = false;

            for pi in 0..probe_n {
                let dir = probes[pi];
                for &frac in perturb_fractions {
                    let pos = add(cur, scale(dir, h * frac));
                    // Evaluate: all incident tets valid, no new sliver, and the
                    // target sliver tet's min dihedral >= threshold.
                    let mut new_worst = f64::INFINITY;
                    let mut all_valid = true;
                    let mut new_sliver_count = 0u32;
                    let mut target_min_dih = f64::INFINITY;
                    for &i in &incident {
                        let t = tets[i];
                        let a = vertices[t[0] as usize];
                        let b = vertices[t[1] as usize];
                        let c = vertices[t[2] as usize];
                        let d = vertices[t[3] as usize];
                        let (a, b, c, d) = substitute_v(a, b, c, d, &t, v, pos);
                        let sv = signed_volume(a, b, c, d);
                        if sv <= 0.0 {
                            all_valid = false;
                            break;
                        }
                        let q = tet_quality_points(a, b, c, d);
                        let s = score(&q, obj);
                        new_worst = new_worst.min(s);
                        if q.min_dihedral < sliver_min_dihedral_rad {
                            new_sliver_count += 1;
                        }
                        if i == sliver_idx {
                            target_min_dih = q.min_dihedral;
                        }
                    }
                    if !all_valid {
                        continue;
                    }
                    // Accept if: the sliver is fixed (target_min_dih >=
                    // threshold), no new sliver was created
                    // (new_sliver_count == 0), and the local worst score
                    // strictly improves.
                    let fixes = target_min_dih >= sliver_min_dihedral_rad;
                    if fixes && new_sliver_count == 0 && new_worst > best_worst + 1e-15 {
                        best_worst = new_worst;
                        best_pos = pos;
                        best_fixes_sliver = true;
                    }
                }
            }

            if best_fixes_sliver && best_pos != cur {
                vertices[v as usize] = best_pos;
                for &i in &incident {
                    let t = tets[i];
                    let a = vertices[t[0] as usize];
                    let b = vertices[t[1] as usize];
                    let c = vertices[t[2] as usize];
                    let d = vertices[t[3] as usize];
                    scores[i] = score_corners(a, b, c, d, obj);
                }
                applied += 1;
                break 'outer;
            }
        }
    }
    applied
}
