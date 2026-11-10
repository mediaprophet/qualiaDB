use super::*;

fn joint(parent: Option<u16>, y: f64) -> Joint {
    Joint {
        parent,
        rest_local: Motor::from_translation(0.0, y, 0.0),
        inverse_bind: Motor::from_translation(0.0, -y, 0.0),
    }
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

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn epic3_and_epic4_full_pipeline_absm_ik_morph_and_gpu_skin_zero_heap() {
    use crate::specialized_libs::computational_geometry::allocation_counter::assert_zero_alloc;

    // 3-joint limb hierarchy: 0=root(hip), 1=mid(knee), 2=end(ankle)
    let joints = [joint(None, 0.0), joint(Some(0), 1.0), joint(Some(1), 1.0)];
    let inv_bind = joints.map(|j| j.inverse_bind);
    let rest_local = joints.map(|j| j.rest_local);

    let mut absm = AbsmMachine::<4, 8, 8>::new();
    let mut layer = AbsmLayer::new(BoneLayerMask::ALL, LayerBlendMode::Override, 1.0);
    layer
        .add_state(AbsmStateNode {
            source: AbsmStateSource::SingleClip { clip_index: 0 },
            duration: 1.0,
            speed: 1.0,
            looping: true,
            ..Default::default()
        })
        .unwrap();
    absm.add_layer(layer).unwrap();

    let clip0 = rest_local;
    let clips: [&[Motor]; 1] = [&clip0];
    let mut local_pose = rest_local;
    let mut world_pose = [Motor::identity(); 3];
    let mut skin_palette = [Motor::identity(); 3];
    let mut prev_skin_palette = [Motor::identity(); 3];
    let mut gpu_palette = [GpuJointMotorPair::identity(); 3];
    let mut signals = SignalQueue::<8>::new();

    let morph_deltas = [GpuMorphDelta::new([0.0, 0.2, 0.0], [0.0, 0.0, 0.0]); 8];
    let base_pos = [[0.0f32, 2.0, 0.0]];
    let base_nrm = [[0.0f32, 1.0, 0.0]];
    let mut morphed_pos = [[0.0f32; 3]; 1];
    let mut morphed_nrm = [[0.0f32; 3]; 1];
    let gpu_attr = GpuSkinnedVertex {
        joint_indices: [2, 0, 0, 0],
        joint_weights: [1.0, 0.0, 0.0, 0.0],
    };

    assert_zero_alloc("epic3_epic4_integrated_pipeline", || {
        let _ = absm
            .step_and_evaluate_zero_heap(0.016, &rest_local, &clips, &mut local_pose, &mut signals)
            .unwrap();
        compose_palette_into(
            &joints,
            &local_pose,
            Motor::identity(),
            &mut world_pose,
            &mut skin_palette,
        )
        .unwrap();

        let ik = solve_two_bone_motor_ik_zero_heap(
            world_pose[0],
            world_pose[1],
            world_pose[2],
            1.0,
            1.0,
            [0.0, 1.2, 0.6],
            [0.0, 1.0, 2.0],
            0.1,
            std::f32::consts::PI,
        )
        .unwrap();
        assert!(ik.target_error < 1e-4);

        let morph_w = compute_ontogenetic_morph_weights_zero_heap(&[0.5], 8, 1, 2, 100, 900);
        evaluate_morph_deltas_zero_heap(
            &base_pos,
            &base_nrm,
            &morph_deltas,
            &morph_w,
            &mut morphed_pos,
            &mut morphed_nrm,
        )
        .unwrap();

        prev_skin_palette = skin_palette;
        evaluate_skinning_motor_palette_zero_heap(
            &inv_bind,
            &ik.world_motors,
            &prev_skin_palette,
            &mut gpu_palette,
            &mut skin_palette,
        );
        let (skinned_p, skinned_n, _, _) = gpu_shader_skin_vertex_reference(
            morphed_pos[0],
            morphed_nrm[0],
            [1.0, 0.0, 0.0, 1.0],
            &gpu_attr,
            &gpu_palette,
        );
        assert!(skinned_p.iter().all(|v| v.is_finite()));
        assert!((skinned_n.iter().map(|v| v * v).sum::<f32>().sqrt() - 1.0).abs() < 1e-5);
    });
}
