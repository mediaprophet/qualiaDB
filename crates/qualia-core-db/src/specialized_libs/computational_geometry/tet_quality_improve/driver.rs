//! Top-level driver that iterates the flip / smooth / insert / exude passes to
//! a fixpoint (or the `max_passes` cap).

use super::*;

// ---------------------------------------------------------------------------
//  Top-level driver
// ---------------------------------------------------------------------------

/// Improve a tetrahedral mesh's quality distribution via flip / smooth /
/// insert / exude passes.
///
/// `fixed_vertices` (optional) marks vertices that must never be moved; in
/// addition, all boundary vertices (vertices incident to a boundary face) are
/// always pinned. The input mesh must be valid (every tet positively
/// oriented); use an orientation-repair pass first if needed.
pub fn improve_tet_mesh(
    vertices: &[Point3],
    tets: &[[u32; 4]],
    fixed_vertices: Option<&[u32]>,
    options: &TetImproveOptions,
) -> Result<TetImproveResult, TetImproveError> {
    validate_input(vertices, tets)?;

    let stats_before =
        tet_mesh_quality_slice(vertices, tets).map_err(|_| TetImproveError::DegenerateInput {
            vertices: vertices.len(),
            tets: tets.len(),
        })?;

    let mut work_vertices: Vec<Point3> = vertices.to_vec();
    let mut work_tets: Vec<[u32; 4]> = tets.to_vec();
    let mut scores = score_all(&work_vertices, &work_tets, options.objective)?;

    // Fixed set = boundary vertices + caller-supplied fixed vertices.
    let mut fixed = boundary_vertices(&work_tets);
    if let Some(extra) = fixed_vertices {
        for &v in extra {
            fixed.insert(v);
        }
    }

    let sliver_rad = options.sliver_min_dihedral_deg.to_radians();
    let mut steiner = 0u32;
    let mut flips = 0u32;
    let mut smooths = 0u32;
    let mut inserts = 0u32;
    let mut exudes = 0u32;
    let mut passes_run = 0u32;

    for _ in 0..options.max_passes {
        passes_run += 1;
        let mut did_work = false;

        if options.flip_enabled {
            let n = flip_pass(
                &work_vertices,
                &mut work_tets,
                &mut scores,
                options.objective,
            );
            if n > 0 {
                flips += n;
                did_work = true;
            }
        }
        if options.smooth_enabled {
            let n = smooth_pass(
                &mut work_vertices,
                &work_tets,
                &mut scores,
                &fixed,
                options.objective,
                options.probe_count,
                &options.perturb_fractions,
            );
            if n > 0 {
                smooths += n;
                did_work = true;
            }
        }
        if options.insert_enabled && steiner < options.max_steiner {
            // Insert one Steiner point per pass (worst tet); the outer loop
            // re-evaluates.
            match insert_pass(
                &mut work_vertices,
                &mut work_tets,
                &mut scores,
                options.objective,
                &mut steiner,
                options.max_steiner,
            )? {
                0 => {}
                n => {
                    inserts += n;
                    did_work = true;
                    // Re-score the whole mesh (adjacency changed globally).
                    scores = score_all(&work_vertices, &work_tets, options.objective)?;
                    // Re-detect boundary vertices (Steiner points are interior
                    // by construction, so the boundary set is unchanged, but
                    // refresh to be safe).
                    fixed = boundary_vertices(&work_tets);
                    if let Some(extra) = fixed_vertices {
                        for &v in extra {
                            fixed.insert(v);
                        }
                    }
                }
            }
        }
        if options.exude_enabled {
            let n = exude_pass(
                &mut work_vertices,
                &work_tets,
                &mut scores,
                &fixed,
                options.objective,
                sliver_rad,
                options.probe_count,
                &options.perturb_fractions,
            );
            if n > 0 {
                exudes += n;
                did_work = true;
            }
        }

        if !did_work {
            break;
        }
    }

    let stats_after = tet_mesh_quality_slice(&work_vertices, &work_tets).map_err(|_| {
        TetImproveError::DegenerateInput {
            vertices: work_vertices.len(),
            tets: work_tets.len(),
        }
    })?;

    Ok(TetImproveResult {
        vertices: work_vertices,
        tets: work_tets,
        stats_before,
        stats_after,
        flips_applied: flips,
        smooths_applied: smooths,
        inserts_applied: inserts,
        exudes_applied: exudes,
        passes_run,
    })
}
