//! Source regressions authored for the deferred coding-complete check.
use super::*;
use super::super::{HolosonicNode, SceneAudioBusGraph, pcm::PcmRenderBudget,
    pcm_schedule::{ClipScheduleBudget, ClipScheduleWorkspace, ClipSpanReceipt, PcmClip},
    pcm::PcmSourceBinding};

fn binding() -> SoundBinding {
    SoundBinding { model_sha256: [1;32], source_sha256: [2;32], graph_sha256: [3;32],
        source_generation: 1, graph_generation: 1, transport_epoch: 1 }
}
fn budget() -> ClipScheduleBudget {
    ClipScheduleBudget { max_authored_clips: 4, max_selected_clips: 1,
        max_event_spans: 16, max_work: u64::MAX, max_transaction_bytes: usize::MAX,
        render: PcmRenderBudget { max_frames: 48, max_active_voices: 1,
            max_obstacles: 0, sample_rate: 48_000 } }
}
#[test]
fn cue_cycle_preserves_silence_and_refuses_clipping_without_publication() {
    use cycle::*;
    let samples = [0.5;2];
    let cue = CueSpan { cue_key:1, start_frame:1, end_frame:3,
        source_start_frame:0, gain:1., samples:&samples };
    let mut scratch=[0.;4]; let mut output=[7.;4];
    compile_cycle_pcm(&[cue],&mut scratch,&mut output,
        CycleBudget {max_cues:2,max_frames:4,max_work:100},&||false).unwrap();
    assert_eq!(output,[0.,0.5,0.5,0.]);
    let previous=output;
    let duplicate = CueSpan {cue_key:2,..cue};
    let loud = CueSpan {samples:&[0.75,0.75],..cue};
    assert_eq!(compile_cycle_pcm(&[loud,duplicate],&mut scratch,&mut output,
        CycleBudget {max_cues:2,max_frames:4,max_work:100},&||false),Err(CycleError::Clipping));
    assert_eq!(output,previous);
}
#[test]
fn source_or_graph_digest_cannot_change_under_the_same_generation() {
    let mut assignments=[None;4]; let mut state=ClipScheduleState::new(&mut assignments);
    let mut scope=SoundScopeState::default();
    scope.bind(binding(),&mut state).unwrap();
    scope.retire(&mut state);
    assert_eq!(scope.bind(SoundBinding {source_sha256:[9;32], transport_epoch:2,
        ..binding()},&mut state),Err(SoundError::Binding));
    assert_eq!(scope.bind(binding(),&mut state),Err(SoundError::Stale));
    scope.bind(SoundBinding {transport_epoch:2,..binding()},&mut state).unwrap();
}
#[test]
fn refused_work_budget_invalidates_lease_without_touching_unadmitted_buffers() {
    let mut graph=SceneAudioBusGraph::new();
    let pose=SoundPose {binding:binding(),pose_sequence:1,pose_tick:0,
        listener:graph.listener,emitters:[EmitterPose::default();MAX_SCENE_AUDIO_EMITTERS],emitter_count:0};
    let mut assignments=[None;4]; let mut state=ClipScheduleState::new(&mut assignments);
    let mut scope=SoundScopeState::default(); scope.bind(binding(),&mut state).unwrap();
    let mut mono=[0.;48]; let mut left=[0.;48]; let mut right=[0.;48];
    let mut selections=[0;16]; let mut spans=[ClipSpanReceipt::default();16];
    let mut workspace=ClipScheduleWorkspace::new(&mut mono,&mut left,&mut right,&mut selections,&mut spans);
    let mut l=[7.;48]; let mut r=[8.;48]; let mut refused=budget(); refused.max_work=0;
    assert!(matches!(render_guarded(&mut graph,&mut scope,&mut state,&mut workspace,&[],&pose,0,0,
        SoundBlockMode::Playing,&[],refused,&mut l,&mut r,&|_|true),Err(SoundError::Budget)));
    assert_eq!(l,[7.;48]); assert_eq!(r,[8.;48]); assert_eq!(scope.binding(),None);
}
#[test]
fn one_catalogue_preserves_histories_then_refuses_a_device_cursor_jump() {
    let mut graph=SceneAudioBusGraph::new();
    let mut node=HolosonicNode::default(); node.node_id=1; node.origin_quin_hash=2;
    node.is_ambient=true; node.emitter.amplitude=1.; node.emitter.coherence_radius=100.;
    assert!(graph.push_emitter(node));
    let mut pose=SoundPose {binding:binding(),pose_sequence:1,pose_tick:0,
        listener:graph.listener,emitters:[EmitterPose::default();MAX_SCENE_AUDIO_EMITTERS],emitter_count:1};
    pose.emitters[0]=EmitterPose {node_id:1,origin_quin_hash:2,position_m:[0.,0.,1.]};
    let samples=[0.2;96];
    let clips=[PcmClip {clip_key:7,binding:PcmSourceBinding {source_key:7,generation:1,
        node_id:1,origin_quin_hash:2},emitter_slot:0,start_frame:0,end_frame:96,
        source_start_frame:0,loop_frames:None,gain:1.,priority:0,samples:&samples}];
    let mut assignments=[None;4]; let mut state=ClipScheduleState::new(&mut assignments);
    let mut scope=SoundScopeState::default(); scope.bind(binding(),&mut state).unwrap();
    let mut mono=[0.;48]; let mut left=[0.;48]; let mut right=[0.;48];
    let mut selections=[0;16]; let mut spans=[ClipSpanReceipt::default();16];
    let mut workspace=ClipScheduleWorkspace::new(&mut mono,&mut left,&mut right,&mut selections,&mut spans);
    let mut l=[0.;48]; let mut r=[0.;48];
    render_guarded(&mut graph,&mut scope,&mut state,&mut workspace,&clips,&pose,0,1000,
        SoundBlockMode::Playing,&[],budget(),&mut l,&mut r,&|_|true).unwrap();
    pose.pose_sequence=2; pose.pose_tick=48*5_292_000;
    let second=render_guarded(&mut graph,&mut scope,&mut state,&mut workspace,&clips,&pose,48,1048,
        SoundBlockMode::Playing,&[],budget(),&mut l,&mut r,&|_|true).unwrap();
    assert!(!second.schedule.histories_reset);
    let tail=render_guarded(&mut graph,&mut scope,&mut state,&mut workspace,&clips,&pose,96,1096,
        SoundBlockMode::NaturalTail,&[],budget(),&mut l,&mut r,&|_|true).unwrap();
    assert!(!tail.schedule.histories_reset);
    assert_eq!(tail.first_source_frame,96);
    assert!(matches!(render_guarded(&mut graph,&mut scope,&mut state,&mut workspace,&clips,&pose,144,1145,
        SoundBlockMode::NaturalTail,&[],budget(),&mut l,&mut r,&|_|true),Err(SoundError::Stale)));
    assert_eq!(scope.binding(),None); assert_eq!(l,[0.;48]); assert_eq!(r,[0.;48]);
}
