use super::*;
use crate::audio::scene_audio_graph::{
    evaluate_acoustic_occlusion_zero_heap, AcousticMaterial, AcousticObstacleShape,
    AcousticOcclusionResult,
};
#[test]
fn positional_reflections_and_direct_history_survive_natural_clip_end() {
    let mut graph = graph();
    graph.listener.position = [0.0, 1.5, 0.0];
    graph.emitters[0].is_ambient = false;
    graph.emitters[0].emitter.position = [1.0, 1.5, -20.0];
    let obstacles = [AcousticObstacle {
        shape: AcousticObstacleShape::AnalyticalSphere {
            center: [0.5, 1.5, -10.0],
            radius: 2.0,
        },
        material: AcousticMaterial::RockLithology,
        thickness_m: 4.0,
        smooth_blend_radius: 0.25,
    }];
    let mut occlusion = AcousticOcclusionResult::default();
    evaluate_acoustic_occlusion_zero_heap(
        graph.listener.position,
        graph.emitters[0].emitter.position,
        &obstacles,
        &mut occlusion,
    );
    assert!(occlusion.early_reflection_gains[1] > 0.0);
    let source = [0.2; 256];
    let clips = [clip(1, 0, &source, 0, 256, 0)];
    let mut assignments = [None; 1];
    let mut state = ClipScheduleState::new(&mut assignments);
    let (l, r, _, _, _) = run_obstacles(&graph, &mut state, &clips, 0, 0, 4096, 1, &obstacles);
    assert!(l[256..].iter().any(|v| v.abs() > 1e-8));
    assert_ne!(l, r);
    let mut assignments = [None; 1];
    let mut split = ClipScheduleState::new(&mut assignments);
    let (a, b, _, _, _) = run_obstacles(&graph, &mut split, &clips, 0, 0, 173, 1, &obstacles);
    let (c, d, report, _, _) =
        run_obstacles(&graph, &mut split, &clips, 173, 0, 3923, 1, &obstacles);
    assert!(!report.histories_reset);
    assert_eq!(l, [a, c].concat());
    assert_eq!(r, [b, d].concat());
    // Explicit continuous-silence bus reference retains the same direct/room state.
    let mut full = vec![0.0; 4096];
    full[..256].copy_from_slice(&source);
    let silent = vec![0.0; 4096];
    let mut reference = PcmRenderState::default();
    let mut ref_l = vec![0.0; 4096];
    let mut ref_r = vec![0.0; 4096];
    let mut b = budget(1).render;
    b.max_obstacles = 1;
    graph
        .render_pcm_quantum_zero_heap(
            &mut reference,
            &[
                PcmVoiceBlock {
                    slot: 0,
                    binding: PcmSourceBinding {
                        source_key: 1,
                        generation: 1,
                        node_id: 1,
                        origin_quin_hash: 1,
                    },
                    samples: &full,
                    gain: 1.0,
                },
                PcmVoiceBlock {
                    slot: 1,
                    binding: PcmSourceBinding {
                        source_key: 2,
                        generation: 1,
                        node_id: 2,
                        origin_quin_hash: 2,
                    },
                    samples: &silent,
                    gain: 1.0,
                },
            ],
            &obstacles,
            b,
            &mut ref_l,
            &mut ref_r,
        )
        .unwrap();
    assert_eq!(l, ref_l);
    assert_eq!(r, ref_r);
}
