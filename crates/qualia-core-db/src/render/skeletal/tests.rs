use super::*;

fn joint(parent: Option<u16>, y: f64) -> Joint {
    Joint {
        parent,
        rest_local: Motor::from_translation(0.0, y, 0.0),
        inverse_bind: Motor::from_translation(0.0, -y, 0.0),
    }
}
fn near(a: [f64; 3], b: [f64; 3]) {
    for axis in 0..3 {
        assert!((a[axis] - b[axis]).abs() < 1e-6, "{a:?} != {b:?}");
    }
}

#[test]
fn bind_pose_and_root_transform_preserve_mesh_and_attachment() {
    let joints = [joint(None, 0.0), joint(Some(0), 1.0)];
    let local = joints.map(|j| j.rest_local);
    let mut globals = [Motor::identity(); 2];
    let mut palette = globals;
    compose_palette_into(
        &joints,
        &local,
        Motor::from_translation(3.0, 0.0, 0.0),
        &mut globals,
        &mut palette,
    )
    .unwrap();
    near(globals[1].transform_point([0.0; 3]), [3.0, 1.0, 0.0]);
    let weights = [SkinWeight {
        joints: [1, 0, 0, 0],
        weights: [1.0, 0.0, 0.0, 0.0],
    }];
    let mut positions = [[0.0; 3]];
    let mut normals = positions;
    skin_vertices_into(
        &[[0.0, 2.0, 0.0]],
        &[[0.0, 1.0, 0.0]],
        &weights,
        &palette,
        &mut positions,
        &mut normals,
    )
    .unwrap();
    near(positions[0].map(f64::from), [3.0, 2.0, 0.0]);
    near(normals[0].map(f64::from), [0.0, 1.0, 0.0]);
}

#[test]
fn articulated_rotation_moves_mesh_and_normal_together() {
    let joints = [joint(None, 0.0)];
    let half = std::f64::consts::FRAC_PI_4;
    let local = [Motor::from_rotation_translation(
        [half.cos(), 0.0, 0.0, half.sin()],
        [0.0; 3],
    )];
    let mut globals = [Motor::identity()];
    let mut palette = globals;
    compose_palette_into(
        &joints,
        &local,
        Motor::identity(),
        &mut globals,
        &mut palette,
    )
    .unwrap();
    let weights = [SkinWeight {
        joints: [0; 4],
        weights: [1.0, 0.0, 0.0, 0.0],
    }];
    let mut p = [[0.0; 3]];
    let mut n = p;
    skin_vertices_into(
        &[[1.0, 0.0, 0.0]],
        &[[1.0, 0.0, 0.0]],
        &weights,
        &palette,
        &mut p,
        &mut n,
    )
    .unwrap();
    near(p[0].map(f64::from), [0.0, 1.0, 0.0]);
    near(n[0].map(f64::from), [0.0, 1.0, 0.0]);
}

#[test]
fn looping_sampling_and_transition_use_engine_motor_interpolation() {
    let joints = [joint(None, 0.0)];
    let keys = [
        Key {
            time: 0.0,
            local: Motor::identity(),
        },
        Key {
            time: 1.0,
            local: Motor::from_translation(2.0, 0.0, 0.0),
        },
        Key {
            time: 2.0,
            local: Motor::identity(),
        },
    ];
    let tracks = [Track {
        joint: 0,
        first_key: 0,
        key_count: 3,
    }];
    let clip = Clip {
        duration: 2.0,
        looping: true,
        tracks: &tracks,
        keys: &keys,
    };
    let mut out = [Motor::identity()];
    sample_pose_into(&joints, clip, 2.5, None, 1.0, &mut out).unwrap();
    near(out[0].transform_point([0.0; 3]), [1.0, 0.0, 0.0]);
    sample_pose_into(
        &joints,
        clip,
        -1.5,
        Some(&[Motor::identity()]),
        0.5,
        &mut out,
    )
    .unwrap();
    near(out[0].transform_point([0.0; 3]), [0.5, 0.0, 0.0]);
}

#[test]
fn malformed_hierarchy_clip_and_skin_fail_before_writes() {
    let sentinel = Motor::from_translation(9.0, 8.0, 7.0);
    let mut globals = [sentinel];
    let mut palette = globals;
    assert_eq!(
        compose_palette_into(
            &[joint(Some(0), 0.0)],
            &[Motor::identity()],
            Motor::identity(),
            &mut globals,
            &mut palette
        ),
        Err(SkeletalError::InvalidInput)
    );
    assert_eq!(globals, [sentinel]);
    assert_eq!(palette, [sentinel]);
    let keys = [
        Key {
            time: 0.0,
            local: Motor::identity(),
        },
        Key {
            time: 1.0,
            local: Motor::from_translation(1.0, 0.0, 0.0),
        },
    ];
    let tracks = [Track {
        joint: 0,
        first_key: 0,
        key_count: 2,
    }];
    let clip = Clip {
        duration: 1.0,
        looping: true,
        tracks: &tracks,
        keys: &keys,
    };
    assert_eq!(
        sample_pose_into(&[joint(None, 0.0)], clip, 0.5, None, 1.0, &mut globals),
        Err(SkeletalError::InvalidInput)
    );
    assert_eq!(globals, [sentinel]);
    let bad = [SkinWeight {
        joints: [2, 0, 0, 0],
        weights: [1.0, 0.0, 0.0, 0.0],
    }];
    let mut p = [[9.0; 3]];
    let mut n = p;
    assert_eq!(
        skin_vertices_into(
            &[[0.0; 3]],
            &[[0.0, 1.0, 0.0]],
            &bad,
            &[Motor::identity()],
            &mut p,
            &mut n
        ),
        Err(SkeletalError::InvalidInput)
    );
    assert_eq!(p, [[9.0; 3]]);
    assert_eq!(n, [[9.0; 3]]);
}

#[test]
fn truncated_outputs_leave_other_outputs_untouched() {
    let joints = [joint(None, 0.0)];
    let sentinel = Motor::from_translation(9.0, 0.0, 0.0);
    let mut globals = [sentinel];
    assert_eq!(
        compose_palette_into(
            &joints,
            &[Motor::identity()],
            Motor::identity(),
            &mut globals,
            &mut []
        ),
        Err(SkeletalError::OutputTooSmall)
    );
    assert_eq!(globals, [sentinel]);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn animation_frame_pipeline_allocates_no_heap() {
    use crate::specialized_libs::computational_geometry::allocation_counter::assert_zero_alloc;
    let joints = [joint(None, 0.0)];
    let keys = [
        Key {
            time: 0.0,
            local: Motor::identity(),
        },
        Key {
            time: 1.0,
            local: Motor::from_translation(1.0, 0.0, 0.0),
        },
    ];
    let tracks = [Track {
        joint: 0,
        first_key: 0,
        key_count: 2,
    }];
    let clip = Clip {
        duration: 1.0,
        looping: false,
        tracks: &tracks,
        keys: &keys,
    };
    let weights = [SkinWeight {
        joints: [0; 4],
        weights: [1.0, 0.0, 0.0, 0.0],
    }];
    let mut local = [Motor::identity()];
    let mut globals = local;
    let mut palette = local;
    let mut positions = [[0.0; 3]];
    let mut normals = positions;
    assert_zero_alloc("skeletal_frame", || {
        sample_pose_into(&joints, clip, 0.5, None, 1.0, &mut local).unwrap();
        compose_palette_into(
            &joints,
            &local,
            Motor::identity(),
            &mut globals,
            &mut palette,
        )
        .unwrap();
        skin_vertices_into(
            &[[0.0; 3]],
            &[[0.0, 1.0, 0.0]],
            &weights,
            &palette,
            &mut positions,
            &mut normals,
        )
        .unwrap();
    });
}
