//! Input validation and the public post-run verification helper.

use super::*;

// ---------------------------------------------------------------------------
//  Validation
// ---------------------------------------------------------------------------

/// Validate the input mesh: every tet has strictly positive signed volume.
pub(super) fn validate_input(
    vertices: &[Point3],
    tets: &[[u32; 4]],
) -> Result<(), TetImproveError> {
    if vertices.len() < 4 || tets.is_empty() {
        return Err(TetImproveError::DegenerateInput {
            vertices: vertices.len(),
            tets: tets.len(),
        });
    }
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
        let sv = signed_volume(a, b, c, d);
        if sv <= 0.0 {
            return Err(TetImproveError::InvertedInputTet {
                tet: i,
                signed_volume: sv,
            });
        }
    }
    Ok(())
}

/// Verify the result mesh: every tet valid + positively oriented, and the
/// worst-case score is not worse than `prev_worst`.
pub fn verify_improvement(
    vertices: &[Point3],
    tets: &[[u32; 4]],
    obj: TetImproveObjective,
    prev_worst: f64,
) -> Result<bool, TetImproveError> {
    let scores = score_all(vertices, tets, obj)?;
    let all_valid = tets.iter().enumerate().all(|(_, tet)| {
        let a = vertices[tet[0] as usize];
        let b = vertices[tet[1] as usize];
        let c = vertices[tet[2] as usize];
        let d = vertices[tet[3] as usize];
        signed_volume(a, b, c, d) > 0.0
    });
    if !all_valid {
        return Ok(false);
    }
    let worst = scores.iter().copied().fold(f64::INFINITY, f64::min);
    Ok(worst + 1e-12 >= prev_worst)
}
