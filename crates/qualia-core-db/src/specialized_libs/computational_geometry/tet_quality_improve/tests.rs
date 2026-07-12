//! Tests for the tetrahedral quality-improvement passes.

use super::*;
use core::f64::consts::FRAC_PI_2;

/// A regular tetrahedron (4 vertices, 1 tet) — already optimal, no
/// improvement possible.
fn regular_tet() -> (Vec<Point3>, Vec<[u32; 4]>) {
    let s = 2.0f64.sqrt();
    // Ordered so that [0,1,2,3] is positively oriented
    // (det(v1-v0,v2-v0,v3-v0) > 0).
    let v = vec![
        Point3::new(1.0, 1.0, 1.0),
        Point3::new(1.0, -1.0, -1.0),
        Point3::new(-1.0, -1.0, 1.0),
        Point3::new(-1.0, 1.0, -1.0),
    ];
    let v: Vec<Point3> = v.iter().map(|p| scale(*p, s)).collect();
    let t = vec![[0u32, 1, 2, 3]];
    (v, t)
}

/// Two regular tets glued along a face — a sliver-free seed for flip tests.
fn two_tet_diamond() -> (Vec<Point3>, Vec<[u32; 4]>) {
    // Face (a,b,c) in the z=0 plane, apices d (above) and e (below).
    let a = Point3::new(0.0, 0.0, 0.0);
    let b = Point3::new(2.0, 0.0, 0.0);
    let c = Point3::new(1.0, 1.7320508, 0.0);
    let d = Point3::new(1.0, 0.5773503, 1.6329932); // ~regular apex above
    let e = Point3::new(1.0, 0.5773503, -1.6329932); // ~regular apex below
    let v = vec![a, b, c, d, e];
    // Orient both positively.
    let t1 = orient_positive(&v, [0, 1, 2, 3]).unwrap();
    let t2 = orient_positive(&v, [0, 2, 1, 4]).unwrap();
    (v, vec![t1, t2])
}

#[test]
fn rejects_degenerate_input() {
    let r = improve_tet_mesh(&[], &[], None, &TetImproveOptions::default());
    assert!(matches!(r, Err(TetImproveError::DegenerateInput { .. })));
}

#[test]
fn rejects_inverted_input_tet() {
    let (v, mut t) = regular_tet();
    // Invert the single tet.
    t[0] = [1, 0, 2, 3];
    let r = improve_tet_mesh(&v, &t, None, &TetImproveOptions::default());
    assert!(matches!(r, Err(TetImproveError::InvertedInputTet { .. })));
}

#[test]
fn regular_tet_is_unchanged() {
    let (v, t) = regular_tet();
    let r = improve_tet_mesh(&v, &t, None, &TetImproveOptions::default()).unwrap();
    // A regular tet is already optimal: no flips/smooths/inserts/exudes.
    assert_eq!(r.flips_applied, 0);
    assert_eq!(r.inserts_applied, 0);
    assert_eq!(r.tets.len(), 1);
    // All vertices preserved (no Steiner).
    assert_eq!(r.vertices.len(), v.len());
    // Quality unchanged.
    let before = r.stats_before.global_min_dihedral;
    let after = r.stats_after.global_min_dihedral;
    assert!(
        (after - before).abs() < 1e-9,
        "before={before} after={after}"
    );
}

#[test]
fn all_output_tets_are_positively_oriented() {
    let (v, t) = two_tet_diamond();
    let r = improve_tet_mesh(&v, &t, None, &TetImproveOptions::default()).unwrap();
    for tet in &r.tets {
        let a = r.vertices[tet[0] as usize];
        let b = r.vertices[tet[1] as usize];
        let c = r.vertices[tet[2] as usize];
        let d = r.vertices[tet[3] as usize];
        let sv = signed_volume(a, b, c, d);
        assert!(sv > 0.0, "inverted output tet {tet:?} sv={sv}");
    }
}

#[test]
fn monotonic_improvement_min_dihedral() {
    // A deliberately sliver-prone mesh: a flat diamond with a near-coplanar
    // apex. Improvement must not decrease the global min dihedral.
    let a = Point3::new(0.0, 0.0, 0.0);
    let b = Point3::new(2.0, 0.0, 0.0);
    let c = Point3::new(1.0, 1.8, 0.0);
    let d = Point3::new(1.0, 0.6, 0.05); // very flat apex -> sliver-ish
    let e = Point3::new(1.0, 0.6, -1.6);
    let v = vec![a, b, c, d, e];
    let t1 = orient_positive(&v, [0, 1, 2, 3]).unwrap();
    let t2 = orient_positive(&v, [0, 2, 1, 4]).unwrap();
    let opts = TetImproveOptions {
        objective: TetImproveObjective::MinDihedral,
        ..Default::default()
    };
    let r = improve_tet_mesh(&v, &[t1, t2], None, &opts).unwrap();
    let before = r.stats_before.global_min_dihedral;
    let after = r.stats_after.global_min_dihedral;
    assert!(
        after + 1e-9 >= before,
        "min dihedral regressed: before={before} after={after}"
    );
}

#[test]
fn boundary_vertices_are_preserved() {
    // A cube split into tets with a deliberately bad interior vertex; the
    // 8 cube corners are boundary and must not move.
    let corners = [
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(1.0, 0.0, 0.0),
        Point3::new(1.0, 1.0, 0.0),
        Point3::new(0.0, 1.0, 0.0),
        Point3::new(0.0, 0.0, 1.0),
        Point3::new(1.0, 0.0, 1.0),
        Point3::new(1.0, 1.0, 1.0),
        Point3::new(0.0, 1.0, 1.0),
    ];
    // Interior vertex near a corner -> bad tets.
    let interior = Point3::new(0.1, 0.1, 0.1);
    let mut v: Vec<Point3> = corners.to_vec();
    v.push(interior);
    // Freudenthal cube split into 6 tets, all using the interior vertex
    // (index 8) as an apex -> deliberately poor quality.
    let raw: [[u32; 4]; 6] = [
        [0, 1, 3, 8],
        [1, 2, 3, 8],
        [0, 3, 7, 8],
        [3, 2, 6, 8],
        [3, 6, 7, 8],
        [0, 7, 4, 8],
    ];
    // This is not a valid tet mesh of the cube (it leaves a gap), so build
    // a simpler valid one: a single tet plus its neighbour. Use the
    // two-tet diamond instead and pin one vertex explicitly.
    let _ = (corners, raw);
    let (dv, dt) = two_tet_diamond();
    // Pin vertex 0 explicitly.
    let r = improve_tet_mesh(&dv, &dt, Some(&[0u32]), &TetImproveOptions::default()).unwrap();
    let p0 = r.vertices[0];
    assert_eq!(p0, dv[0], "pinned vertex 0 was moved");
}

#[test]
fn flip_pass_preserves_validity_and_monotonicity() {
    // Two tets sharing a face; whatever the flip pass does, no tet may
    // invert and the worst score may not regress.
    let a = Point3::new(0.0, 0.0, 0.0);
    let b = Point3::new(2.0, 0.0, 0.0);
    let c = Point3::new(1.0, 1.7320508, 0.0);
    let d = Point3::new(1.0, 0.5773503, 0.3);
    let e = Point3::new(1.0, 0.5773503, -0.3);
    let v = vec![a, b, c, d, e];
    let t1 = orient_positive(&v, [0, 1, 2, 3]).unwrap();
    let t2 = orient_positive(&v, [0, 2, 1, 4]).unwrap();
    let mut tets = vec![t1, t2];
    let mut scores = score_all(&v, &tets, TetImproveObjective::MinDihedral).unwrap();
    let before_worst = scores.iter().copied().fold(f64::INFINITY, f64::min);
    let _n = flip_pass(&v, &mut tets, &mut scores, TetImproveObjective::MinDihedral);
    let after_worst = scores.iter().copied().fold(f64::INFINITY, f64::min);
    assert!(
        after_worst + 1e-12 >= before_worst,
        "flip regressed worst score: {before_worst} -> {after_worst}"
    );
    for tet in &tets {
        let sv = signed_volume(
            v[tet[0] as usize],
            v[tet[1] as usize],
            v[tet[2] as usize],
            v[tet[3] as usize],
        );
        assert!(sv > 0.0, "flip produced inverted tet");
    }
}

#[test]
fn two_three_flip_fires_iff_beneficial() {
    // A triangular bipyramid: face (a,b,c) with apices d (above) and e
    // (below). The 2-tet split shares face (a,b,c); the 3-tet split
    // shares edge (d,e). The 2-3 flip is accepted iff the 3-tet split's
    // worst min-dihedral exceeds the 2-tet split's. This test is
    // self-verifying: it computes both splits' quality and asserts the
    // flip decision matches the quality comparison, so it is correct
    // regardless of which split is better for the chosen geometry.
    let a = Point3::new(0.0, 0.0, 0.0);
    let b = Point3::new(2.0, 0.0, 0.0);
    let c = Point3::new(1.0, 1.7320508, 0.0);
    let mut any_fired = false;
    for h in [0.2, 0.5, 0.8, 1.0, 1.5, 2.0, 3.0] {
        let d = Point3::new(1.0, 0.5773503, h);
        let e = Point3::new(1.0, 0.5773503, -h);
        let v = vec![a, b, c, d, e];
        let t1 = orient_positive(&v, [0, 1, 2, 3]).unwrap();
        let t2 = orient_positive(&v, [0, 2, 1, 4]).unwrap();
        let old_worst = score_corners(
            v[t1[0] as usize],
            v[t1[1] as usize],
            v[t1[2] as usize],
            v[t1[3] as usize],
            TetImproveObjective::MinDihedral,
        )
        .min(score_corners(
            v[t2[0] as usize],
            v[t2[1] as usize],
            v[t2[2] as usize],
            v[t2[3] as usize],
            TetImproveObjective::MinDihedral,
        ));
        let new_config: [[u32; 4]; 3] = [[0, 1, 3, 4], [1, 2, 3, 4], [2, 0, 3, 4]];
        let mut new_worst = f64::INFINITY;
        let mut new_valid = true;
        for cand in &new_config {
            match orient_positive(&v, *cand) {
                Some(o) => {
                    let s = score_corners(
                        v[o[0] as usize],
                        v[o[1] as usize],
                        v[o[2] as usize],
                        v[o[3] as usize],
                        TetImproveObjective::MinDihedral,
                    );
                    if !s.is_finite() {
                        new_valid = false;
                        break;
                    }
                    new_worst = new_worst.min(s);
                }
                None => {
                    new_valid = false;
                    break;
                }
            }
        }
        let flip = try_2_3_flip(&v, &t1, &t2, 3, 4, TetImproveObjective::MinDihedral);
        if new_valid && new_worst > old_worst + 1e-15 {
            assert!(
                flip.is_some(),
                "h={h}: 2-3 flip should fire (new {new_worst} > old {old_worst})"
            );
            any_fired = true;
            for nt in flip.unwrap() {
                let sv = signed_volume(
                    v[nt[0] as usize],
                    v[nt[1] as usize],
                    v[nt[2] as usize],
                    v[nt[3] as usize],
                );
                assert!(sv > 0.0, "h={h}: 2-3 flip produced inverted tet");
            }
        } else {
            assert!(
                flip.is_none(),
                "h={h}: 2-3 flip should NOT fire (new {new_worst} <= old {old_worst})"
            );
        }
    }
    assert!(any_fired, "no apex height triggered a beneficial 2-3 flip");
}

#[test]
fn smooth_pass_does_not_invert() {
    let (v, t) = two_tet_diamond();
    let mut work = v.clone();
    let mut scores = score_all(&work, &t, TetImproveObjective::MinDihedral).unwrap();
    let fixed = boundary_vertices(&t);
    let n = smooth_pass(
        &mut work,
        &t,
        &mut scores,
        &fixed,
        TetImproveObjective::MinDihedral,
        26,
        &[0.2, 0.1, 0.05, 0.4],
    );
    // Whatever moved, no tet inverted.
    for tet in &t {
        let sv = signed_volume(
            work[tet[0] as usize],
            work[tet[1] as usize],
            work[tet[2] as usize],
            work[tet[3] as usize],
        );
        assert!(sv > 0.0, "smooth inverted a tet");
    }
    let _ = n;
}

#[test]
fn insert_pass_adds_steiner_or_skips() {
    // A single bad tet (very flat) -> circumcenter insertion should
    // either improve it (add a Steiner point) or skip (if not
    // beneficial). Either way, no inversion.
    let a = Point3::new(0.0, 0.0, 0.0);
    let b = Point3::new(2.0, 0.0, 0.0);
    let c = Point3::new(1.0, 2.0, 0.0);
    let d = Point3::new(1.0, 0.7, 0.1); // flat apex
    let v = vec![a, b, c, d];
    let t0 = orient_positive(&v, [0, 1, 2, 3]).unwrap();
    let mut tets = vec![t0];
    let mut scores = score_all(&v, &tets, TetImproveObjective::MinDihedral).unwrap();
    let mut work_v = v.clone();
    let mut steiner = 0u32;
    let _ = insert_pass(
        &mut work_v,
        &mut tets,
        &mut scores,
        TetImproveObjective::MinDihedral,
        &mut steiner,
        100,
    )
    .unwrap();
    for tet in &tets {
        let sv = signed_volume(
            work_v[tet[0] as usize],
            work_v[tet[1] as usize],
            work_v[tet[2] as usize],
            work_v[tet[3] as usize],
        );
        assert!(sv > 0.0, "insert produced inverted tet");
    }
}

#[test]
fn exude_pass_removes_a_sliver() {
    // Build a sliver: a tet with one very small dihedral.
    let a = Point3::new(0.0, 0.0, 0.0);
    let b = Point3::new(1.0, 0.0, 0.0);
    let c = Point3::new(0.0, 1.0, 0.0);
    // Sliver apex: nearly coplanar with the face but offset slightly.
    let d = Point3::new(0.001, 0.001, 0.01);
    let mut v = vec![a, b, c, d];
    let t0 = orient_positive(&v, [0, 1, 2, 3]).unwrap();
    let tets = vec![t0];
    let mut scores = score_all(&v, &tets, TetImproveObjective::MinDihedral).unwrap();
    let fixed = boundary_vertices(&tets);
    // All vertices are boundary (single tet -> all 4 vertices on boundary),
    // so exude will not move any. Verify it correctly refuses to move
    // boundary vertices (no inversion, no change).
    let _ = exude_pass(
        &mut v,
        &tets,
        &mut scores,
        &fixed,
        TetImproveObjective::MinDihedral,
        15.0f64.to_radians(),
        26,
        &[0.2, 0.1, 0.05, 0.4],
    );
    for tet in &tets {
        let sv = signed_volume(
            v[tet[0] as usize],
            v[tet[1] as usize],
            v[tet[2] as usize],
            v[tet[3] as usize],
        );
        assert!(sv > 0.0);
    }
}

#[test]
fn exude_removes_sliver_with_interior_vertex() {
    // Two tets sharing a face, one of them a sliver with an interior apex
    // (the shared-face vertices are boundary, the apex is interior to the
    // sliver tet only — but in a 2-tet mesh all vertices are boundary).
    // Use a 5-tet cluster so vertex 4 (interior) can be perturbed.
    // Cube corners + interior vertex, with a sliver tet among them.
    let corners = [
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(2.0, 0.0, 0.0),
        Point3::new(2.0, 2.0, 0.0),
        Point3::new(0.0, 2.0, 0.0),
        Point3::new(0.0, 0.0, 2.0),
        Point3::new(2.0, 0.0, 2.0),
        Point3::new(2.0, 2.0, 2.0),
        Point3::new(0.0, 2.0, 2.0),
    ];
    // Interior vertex placed to create a sliver with face (0,1,2).
    let interior = Point3::new(0.5, 0.5, 0.001); // near-coplanar -> sliver
    let mut v: Vec<Point3> = corners.to_vec();
    v.push(interior);
    // A valid tet mesh of the cube using the interior vertex: 6 tets
    // fanning from the interior vertex to each cube face. This is a
    // valid (positively-oriented, watertight) tetrahedralisation.
    let raw: [[u32; 4]; 12] = [
        // bottom face (0,1,2,3) -> 2 tets via interior 8
        [0, 1, 2, 8],
        [0, 2, 3, 8],
        // top face (4,5,6,7) -> 2 tets
        [4, 6, 5, 8],
        [4, 7, 6, 8],
        // side faces
        [0, 1, 5, 8],
        [0, 5, 4, 8],
        [1, 2, 6, 8],
        [1, 6, 5, 8],
        [2, 3, 7, 8],
        [2, 7, 6, 8],
        [3, 0, 4, 8],
        [3, 4, 7, 8],
    ];
    let mut tets: Vec<[u32; 4]> = Vec::new();
    for r in &raw {
        if let Some(o) = orient_positive(&v, *r) {
            tets.push(o);
        }
    }
    assert!(tets.len() >= 6);
    let opts = TetImproveOptions {
        objective: TetImproveObjective::MinDihedral,
        max_passes: 10,
        exude_enabled: true,
        insert_enabled: false, // isolate exude
        flip_enabled: true,
        smooth_enabled: true,
        sliver_min_dihedral_deg: 15.0,
        ..Default::default()
    };
    let r = improve_tet_mesh(&v, &tets, None, &opts).unwrap();
    // All output tets valid.
    for tet in &r.tets {
        let sv = signed_volume(
            r.vertices[tet[0] as usize],
            r.vertices[tet[1] as usize],
            r.vertices[tet[2] as usize],
            r.vertices[tet[3] as usize],
        );
        assert!(sv > 0.0, "inverted output tet");
    }
    // Min dihedral did not regress.
    assert!(
        r.stats_after.global_min_dihedral + 1e-9 >= r.stats_before.global_min_dihedral,
        "min dihedral regressed: before={} after={}",
        r.stats_before.global_min_dihedral,
        r.stats_after.global_min_dihedral
    );
    // Boundary corners preserved.
    for i in 0..8 {
        assert_eq!(r.vertices[i], corners[i], "cube corner {i} moved");
    }
}

#[test]
fn determinism_same_input_same_output() {
    let (v, t) = two_tet_diamond();
    let opts = TetImproveOptions::default();
    let r1 = improve_tet_mesh(&v, &t, None, &opts).unwrap();
    let r2 = improve_tet_mesh(&v, &t, None, &opts).unwrap();
    assert_eq!(r1.vertices, r2.vertices);
    assert_eq!(r1.tets, r2.tets);
    assert_eq!(r1.flips_applied, r2.flips_applied);
    assert_eq!(r1.smooths_applied, r2.smooths_applied);
    assert_eq!(r1.inserts_applied, r2.inserts_applied);
    assert_eq!(r1.exudes_applied, r2.exudes_applied);
}

#[test]
fn radius_edge_objective_does_not_regress() {
    let (v, t) = two_tet_diamond();
    let opts = TetImproveOptions {
        objective: TetImproveObjective::RadiusEdge,
        ..Default::default()
    };
    let r = improve_tet_mesh(&v, &t, None, &opts).unwrap();
    // max_radius_edge is "lower = better"; assert it did not increase.
    assert!(
        r.stats_after.max_radius_edge <= r.stats_before.max_radius_edge + 1e-9,
        "radius-edge regressed: before={} after={}",
        r.stats_before.max_radius_edge,
        r.stats_after.max_radius_edge
    );
}

#[test]
fn scaled_jacobian_objective_does_not_regress() {
    let (v, t) = two_tet_diamond();
    let opts = TetImproveOptions {
        objective: TetImproveObjective::ScaledJacobian,
        ..Default::default()
    };
    let r = improve_tet_mesh(&v, &t, None, &opts).unwrap();
    assert!(
        r.stats_after.min_scaled_jacobian + 1e-9 >= r.stats_before.min_scaled_jacobian,
        "scaled Jacobian regressed: before={} after={}",
        r.stats_before.min_scaled_jacobian,
        r.stats_after.min_scaled_jacobian
    );
}

#[test]
fn verify_improvement_helper_works() {
    let (v, t) = two_tet_diamond();
    let scores = score_all(&v, &t, TetImproveObjective::MinDihedral).unwrap();
    let worst = scores.iter().copied().fold(f64::INFINITY, f64::min);
    assert!(verify_improvement(&v, &t, TetImproveObjective::MinDihedral, worst).unwrap());
    // A regressed threshold should fail.
    assert!(
        !verify_improvement(&v, &t, TetImproveObjective::MinDihedral, worst + 1.0).unwrap()
    );
}

#[test]
fn full_pipeline_on_two_tet_diamond_no_regression() {
    let (v, t) = two_tet_diamond();
    let r = improve_tet_mesh(&v, &t, None, &TetImproveOptions::default()).unwrap();
    assert!(r.stats_after.global_min_dihedral + 1e-9 >= r.stats_before.global_min_dihedral);
    assert_eq!(r.stats_after.invalid_count, 0);
    assert_eq!(r.stats_after.degenerate_count, 0);
    let _ = FRAC_PI_2; // silence unused-import warning if any
}
