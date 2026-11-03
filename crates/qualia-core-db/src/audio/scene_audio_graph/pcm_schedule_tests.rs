use super::super::HolosonicNode;
use super::*;

fn graph() -> SceneAudioBusGraph {
    let mut graph = SceneAudioBusGraph::new();
    for id in 1..=2 {
        let mut node = HolosonicNode::default();
        node.node_id = id;
        node.origin_quin_hash = u64::from(id);
        node.is_ambient = true;
        node.emitter.amplitude = 1.0;
        node.emitter.coherence_radius = 100.0;
        assert!(graph.push_emitter(node));
    }
    graph
}
fn clip<'a>(
    key: u64,
    slot: usize,
    samples: &'a [f32],
    start: u64,
    end: u64,
    priority: i32,
) -> PcmClip<'a> {
    PcmClip {
        clip_key: key,
        binding: PcmSourceBinding {
            source_key: key,
            generation: 1,
            node_id: slot as u32 + 1,
            origin_quin_hash: slot as u64 + 1,
        },
        emitter_slot: slot,
        start_frame: start,
        end_frame: end,
        source_start_frame: 0,
        loop_frames: None,
        gain: 1.0,
        priority,
        samples,
    }
}
fn budget(selected: usize) -> ClipScheduleBudget {
    ClipScheduleBudget {
        max_authored_clips: 1000,
        max_selected_clips: selected,
        max_event_spans: 32,
        max_work: u64::MAX,
        max_transaction_bytes: usize::MAX,
        render: PcmRenderBudget {
            max_frames: 8192,
            max_active_voices: 2,
            max_obstacles: 0,
            sample_rate: 48_000,
        },
    }
}
fn run(
    graph: &SceneAudioBusGraph,
    state: &mut ClipScheduleState<'_>,
    clips: &[PcmClip<'_>],
    start: u64,
    generation: u64,
    frames: usize,
    selected: usize,
) -> (
    Vec<f32>,
    Vec<f32>,
    ClipScheduleReport,
    Vec<ClipSpanReceipt>,
    Vec<usize>,
) {
    run_obstacles(
        graph,
        state,
        clips,
        start,
        generation,
        frames,
        selected,
        &[],
    )
}
type Rendered = (
    Vec<f32>,
    Vec<f32>,
    ClipScheduleReport,
    Vec<ClipSpanReceipt>,
    Vec<usize>,
);
fn run_obstacles(
    graph: &SceneAudioBusGraph,
    state: &mut ClipScheduleState<'_>,
    clips: &[PcmClip<'_>],
    start: u64,
    generation: u64,
    frames: usize,
    selected: usize,
    obstacles: &[AcousticObstacle],
) -> Rendered {
    let mut mono = vec![0.0; graph.emitter_count * frames];
    let mut staged_l = vec![0.0; frames];
    let mut staged_r = vec![0.0; frames];
    let mut selections = vec![0; 32 * selected];
    let mut spans = [ClipSpanReceipt::default(); 32];
    let mut work = ClipScheduleWorkspace::new(
        &mut mono,
        &mut staged_l,
        &mut staged_r,
        &mut selections,
        &mut spans,
    );
    let mut l = vec![0.0; frames];
    let mut r = vec![0.0; frames];
    let report = graph
        .render_scheduled_pcm_quantum_zero_heap(
            state,
            &mut work,
            clips,
            start,
            generation,
            obstacles,
            {
                let mut b = budget(selected);
                b.render.max_obstacles = obstacles.len();
                b
            },
            &mut l,
            &mut r,
        )
        .unwrap();
    (
        l,
        r,
        report,
        work.spans().to_vec(),
        work.selections().to_vec(),
    )
}
#[test]
fn staggered_starts_ends_rank_only_current_clips_and_keep_exact_loop_phase() {
    let graph = graph();
    let source = [0.1, 0.2, 0.3, 0.4];
    let mut a = clip(1, 0, &source, 2, 5, 0);
    a.source_start_frame = 1;
    let mut b = clip(2, 1, &source, 5, 8, 10);
    b.source_start_frame = 1;
    b.loop_frames = Some(2);
    let clips = [a, b];
    let mut assignments = [None; 2];
    let mut state = ClipScheduleState::new(&mut assignments);
    let (_, _, report, spans, selections) = run(&graph, &mut state, &clips, 0, 0, 10, 1);
    assert_eq!(
        spans
            .iter()
            .map(|s| (s.start_frame, s.end_frame, s.selected_clips))
            .collect::<Vec<_>>(),
        vec![(0, 2, 0), (2, 5, 1), (5, 8, 1), (8, 10, 0)]
    );
    assert_eq!(selections[spans[1].selection_offset], 0);
    assert_eq!(selections[spans[2].selection_offset], 1);
    assert_eq!(report.selected_clip_frames, 6);
    assert_eq!(report.virtualized_clip_frames, 0);
    let first = [0.0, 0.0, 0.2, 0.3, 0.4, 0.0, 0.0, 0.0, 0.0, 0.0];
    let second = [0.0, 0.0, 0.0, 0.0, 0.0, 0.2, 0.3, 0.2, 0.0, 0.0];
    let mut reference = PcmRenderState::default();
    let mut l = [0.0; 10];
    let mut r = [0.0; 10];
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
                    samples: &first,
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
                    samples: &second,
                    gain: 1.0,
                },
            ],
            &[],
            budget(1).render,
            &mut l,
            &mut r,
        )
        .unwrap();
    let mut assignments = [None; 2];
    let mut state = ClipScheduleState::new(&mut assignments);
    let (actual_l, actual_r, _, _, _) = run(&graph, &mut state, &clips, 0, 0, 10, 1);
    assert_eq!(actual_l, l);
    assert_eq!(actual_r, r);
}
#[test]
fn nonzero_room_tails_and_virtualization_are_partition_invariant() {
    let graph = graph();
    let source: [f32; 512] = std::array::from_fn(|i| (i as f32 * 0.09).sin() * 0.2);
    let clips = [
        clip(1, 0, &source, 0, 256, 0),
        clip(2, 1, &source, 700, 1100, 10),
        clip(3, 0, &source, 850, 1250, 5),
    ];
    let mut assignments = [None; 3];
    let mut whole_state = ClipScheduleState::new(&mut assignments);
    let (whole_l, whole_r, report, _, _) = run(&graph, &mut whole_state, &clips, 0, 0, 4096, 1);
    assert!(report.virtualized_clip_frames > 0);
    assert!(whole_l[256..700].iter().any(|v| v.abs() > 1e-8));
    assert!(whole_l[1250..].iter().any(|v| v.abs() > 1e-8));
    let mut assignments = [None; 3];
    let mut state = ClipScheduleState::new(&mut assignments);
    let mut split_l = Vec::new();
    let mut split_r = Vec::new();
    let mut start = 0;
    for frames in [173, 527, 200, 350, 2846] {
        let (l, r, report, _, _) = run(&graph, &mut state, &clips, start, 0, frames, 1);
        if start != 0 {
            assert!(!report.histories_reset);
            assert!(!report.source_rebound);
        }
        split_l.extend(l);
        split_r.extend(r);
        start += frames as u64;
    }
    assert_eq!(whole_l, split_l);
    assert_eq!(whole_r, split_r);
}
#[test]
fn late_span_refusal_is_atomic_for_output_and_history() {
    let graph = graph();
    let source = [0.2; 8];
    let bad = [f32::NAN; 4];
    let clips = [clip(1, 0, &source, 0, 8, 0), clip(2, 1, &bad, 8, 12, 0)];
    let mut assignments = [None; 2];
    let mut state = ClipScheduleState::new(&mut assignments);
    let mut mono = [0.0; 24];
    let mut staged_l = [0.0; 12];
    let mut staged_r = [0.0; 12];
    let mut selections = [0; 64];
    let mut spans = [ClipSpanReceipt::default(); 32];
    let mut work = ClipScheduleWorkspace::new(
        &mut mono,
        &mut staged_l,
        &mut staged_r,
        &mut selections,
        &mut spans,
    );
    let mut l = [9.0; 12];
    let mut r = [8.0; 12];
    assert_eq!(
        graph.render_scheduled_pcm_quantum_zero_heap(
            &mut state,
            &mut work,
            &clips,
            0,
            0,
            &[],
            budget(2),
            &mut l,
            &mut r
        ),
        Err(ClipScheduleError::NumericSample)
    );
    assert_eq!(l, [9.0; 12]);
    assert_eq!(r, [8.0; 12]);
    assert_eq!(state.next_frame, None);
    assert_eq!(state.epoch, 0);
    assert!(work.spans().is_empty());
    let valid = [clip(1, 0, &source, 0, 8, 0)];
    let (a, b, _, _, _) = run(&graph, &mut state, &valid, 0, 0, 12, 1);
    let mut assignments = [None; 1];
    let mut clean = ClipScheduleState::new(&mut assignments);
    let (x, y, _, _, _) = run(&graph, &mut clean, &valid, 0, 0, 12, 1);
    assert_eq!(a, x);
    assert_eq!(b, y);
}
#[test]
fn seek_rebind_revocation_and_transport_clear_aggregate_histories() {
    let mut graph = graph();
    let source = [0.2; 16];
    let mut clips = [clip(1, 0, &source, 0, 16, 0)];
    let mut assignments = [None; 1];
    let mut state = ClipScheduleState::new(&mut assignments);
    assert!(
        run(&graph, &mut state, &clips, 0, 0, 4, 1)
            .2
            .histories_reset
    );
    assert!(
        !run(&graph, &mut state, &clips, 4, 0, 4, 1)
            .2
            .histories_reset
    );
    assert!(
        run(&graph, &mut state, &clips, 0, 0, 4, 1)
            .2
            .timeline_discontinuity
    );
    assert!(
        run(&graph, &mut state, &clips, 4, 1, 4, 1)
            .2
            .histories_reset
    );
    clips[0].binding.generation += 1;
    assert!(run(&graph, &mut state, &clips, 8, 1, 4, 1).2.source_rebound);
    graph.emitters[0].deontic_permit = 2;
    let (l, r, report, _, _) = run(&graph, &mut state, &clips, 12, 1, 4, 1);
    assert!(report.histories_reset);
    assert!(report.source_rebound);
    // Existing DSP injects denormal protection even on a silent permitted bus.
    // Revocation must equal fresh authorized silence, not mathematically zero.
    let mut assignments = [None; 1];
    let mut fresh = ClipScheduleState::new(&mut assignments);
    let (fresh_l, fresh_r, _, _, _) = run(&graph, &mut fresh, &clips, 12, 1, 4, 1);
    assert_eq!(l, fresh_l);
    assert_eq!(r, fresh_r);
    assert!(l.iter().chain(&r).all(|v| v.abs() < 1e-8));
}
#[test]
fn many_authored_clips_prioritize_with_visible_counts_and_explicit_residency_budget() {
    let graph = graph();
    let source = [0.2; 4];
    let clips: Vec<_> = (1..=130)
        .map(|key| clip(key, 0, &source, 0, 4, 0))
        .collect();
    let mut assignments = [None; 130];
    let mut state = ClipScheduleState::new(&mut assignments);
    let (_, _, report, spans, selected) = run(&graph, &mut state, &clips, 0, 0, 4, 2);
    assert_eq!(report.eligible_clip_frames, 520);
    assert_eq!(report.selected_clip_frames, 8);
    assert_eq!(spans[0].virtualized_clips, 128);
    assert_eq!(&selected[..2], &[0, 1]);
    let mut mono = [0.0; 8];
    let mut sl = [0.0; 4];
    let mut sr = [0.0; 4];
    let mut selected = [0; 64];
    let mut spans = [ClipSpanReceipt::default(); 32];
    let mut work =
        ClipScheduleWorkspace::new(&mut mono, &mut sl, &mut sr, &mut selected, &mut spans);
    let mut l = [9.0; 4];
    let mut r = [8.0; 4];
    let mut b = budget(2);
    b.render.max_active_voices = 1;
    assert_eq!(
        graph.render_scheduled_pcm_quantum_zero_heap(
            &mut state,
            &mut work,
            &clips,
            4,
            0,
            &[],
            b,
            &mut l,
            &mut r
        ),
        Err(ClipScheduleError::Budget)
    );
    assert_eq!(state.next_frame, Some(4));
    assert_eq!(l, [9.0; 4]);
}
#[test]
fn event_work_and_transaction_storage_limits_fail_without_publication() {
    let graph = graph();
    let source = [0.2; 4];
    let clips = [clip(1, 0, &source, 2, 6, 0)];
    let mut assignments = [None; 1];
    let mut state = ClipScheduleState::new(&mut assignments);
    let mut mono = [0.0; 16];
    let mut sl = [0.0; 8];
    let mut sr = [0.0; 8];
    let mut selected = [0; 32];
    let mut spans = [ClipSpanReceipt::default(); 32];
    let mut work =
        ClipScheduleWorkspace::new(&mut mono, &mut sl, &mut sr, &mut selected, &mut spans);
    let mut l = [9.0; 8];
    let mut r = [8.0; 8];
    for mode in 0..3 {
        let mut b = budget(1);
        match mode {
            0 => b.max_event_spans = 1,
            1 => b.max_work = 0,
            _ => b.max_transaction_bytes = 0,
        }
        assert_eq!(
            graph.render_scheduled_pcm_quantum_zero_heap(
                &mut state,
                &mut work,
                &clips,
                0,
                0,
                &[],
                b,
                &mut l,
                &mut r
            ),
            Err(ClipScheduleError::Budget)
        );
        assert_eq!(state.next_frame, None);
        assert_eq!(l, [9.0; 8]);
        assert!(work.spans().is_empty());
    }
}

#[path = "pcm_schedule_spatial_tests.rs"]
mod spatial;
