use super::*;
fn mesh() -> Mesh {
    Mesh {
        positions: vec![[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]],
        triangles: vec![[0, 1, 2]],
        min: [0.; 3],
        max: [1., 1., 0.],
    }
}
#[test]
fn authored_tangent_direction_and_handedness_survive_codec_read() {
    let m = mesh();
    let n = [[0., 0., 1.]; 3];
    let t = [[0., 1., 0., -1.]; 3];
    let bytes = crate::render::compile_10d::compile_mesh_to_10d_with_frames(&m, Some(&n), Some(&t))
        .unwrap();
    let (normals, tangents) = decode_asset_frames(&bytes, &m, &[[0.; 2]; 3]).unwrap();
    assert_eq!(normals.len(), 3);
    for v in tangents {
        assert!(v[1] > 0.9999);
        assert!(v[0].abs() < 0.0001);
        assert_eq!(v[3], -1.);
    }
}
#[test]
fn missing_tangents_follow_the_actual_rotated_uv_derivative() {
    let m = mesh();
    let bytes = crate::render::compile_10d::compile_mesh_to_10d(&m).unwrap();
    let (_, t) = decode_asset_frames(&bytes, &m, &[[0., 0.], [0., 1.], [1., 0.]]).unwrap();
    for v in t {
        assert!(v[1] > 0.9999);
        assert!(v[0].abs() < 0.0001);
        assert_eq!(v[3], -1.);
    }
}
#[test]
fn legacy_zero_uvs_still_get_finite_orthogonal_frames() {
    let m = mesh();
    let bytes = crate::render::compile_10d::compile_mesh_to_10d(&m).unwrap();
    let (n, t) = decode_asset_frames(&bytes, &m, &[[0.; 2]; 3]).unwrap();
    for (n, t) in n.iter().zip(t) {
        assert!(t.iter().all(|v| v.is_finite()));
        assert!((n[0] * t[0] + n[1] * t[1] + n[2] * t[2]).abs() < 1e-5);
        assert_eq!(t[3].abs(), 1.);
    }
}
#[test]
fn generation_cap_is_checked_before_scratch_allocation() {
    let positions = vec![[0.; 3]; MAX_TANGENT_VERTICES + 1];
    assert_eq!(
        generate(&positions, &[], &[], &[]).unwrap_err(),
        "per-asset tangent scratch exceeds bounded mesh budget"
    );
}
#[test]
fn fitted_assets_use_local_indices_and_retain_uv_handedness() {
    let mut positions = vec![
        [0., 0., 0.],
        [1., 1., 0.],
        [0., 0., 1.],
        [3., 0., 0.],
        [4., 1., 0.],
        [3., 0., 1.],
    ];
    let fit = crate::render::body_fit::AnatomyBodyFit {
        arm_span_scale_x: 1.7,
        chest_radial: 1.3,
        identity: false,
        ..Default::default()
    };
    fit.apply_in_place(&mut positions, [0., 0., 0.], [4., 1., 1.]);
    let uv = [[0., 0.], [1., 0.], [0., 1.], [0., 0.], [0., 1.], [1., 0.]];
    let mut n = [[0.; 3]; 6];
    let mut t = [[0.; 4]; 6];
    reframe_fitted_assets(
        &positions,
        &uv,
        &[0, 1, 2, 3, 4, 5],
        &[(0, 3), (3, 6)],
        &[(0, 3), (3, 6)],
        &mut n,
        &mut t,
    )
    .unwrap();
    for (n, t) in n.iter().zip(t) {
        assert!((n[0] * t[0] + n[1] * t[1] + n[2] * t[2]).abs() < 1e-5);
        assert_eq!(t[3].abs(), 1.);
    }
    assert_eq!(t[0][3], -t[3][3]);
}
#[test]
fn malformed_fit_span_is_refused_before_frame_mutation() {
    let p = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]];
    let mut n = [[9.; 3]; 3];
    let mut t = [[8.; 4]; 3];
    assert!(reframe_fitted_assets(
        &p,
        &[[0.; 2]; 3],
        &[0, 1, 3],
        &[(0, 3)],
        &[(0, 3)],
        &mut n,
        &mut t
    )
    .is_err());
    assert_eq!(n, [[9.; 3]; 3]);
    assert_eq!(t, [[8.; 4]; 3]);
}
#[test]
fn multiple_bounded_assets_can_exceed_the_old_merged_generation_ceiling() {
    let count = MAX_TANGENT_VERTICES / 2 + 1;
    let mut p = vec![[0.; 3]; count];
    p[1] = [1., 0., 0.];
    p[2] = [0., 1., 0.];
    let uv = vec![[0.; 2]; count];
    let n = vec![[0., 0., 1.]; count];
    let mut merged = generate(&p, &uv, &n, &[0, 1, 2]).unwrap();
    merged.extend(generate(&p, &uv, &n, &[0, 1, 2]).unwrap());
    assert!(merged.len() > MAX_TANGENT_VERTICES);
    assert!(merged
        .iter()
        .all(|v| v.iter().all(|x| x.is_finite()) && v[3].abs() == 1.));
}
