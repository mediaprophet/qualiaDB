use super::*;
use super::super::HolosonicNode;

fn graph() -> SceneAudioBusGraph {
    let mut graph = SceneAudioBusGraph::new();
    let mut node = HolosonicNode::default();
    node.node_id = 7; node.origin_quin_hash = 70; node.emitter.position = [2.0,0.0,-2.0];
    node.emitter.coherence_radius = 100.0; node.emitter.amplitude = 1.0;
    assert!(graph.push_emitter(node));
    graph
}
fn binding(generation:u64) -> PcmSourceBinding {
    PcmSourceBinding {source_key:100,generation,node_id:7,origin_quin_hash:70}
}
fn budget() -> PcmRenderBudget {
    PcmRenderBudget {max_frames:1024,max_active_voices:64,max_obstacles:16,sample_rate:48_000}
}
#[test]
fn pcm_samples_and_histories_survive_block_partition() {
    let graph = graph();
    let source:[f32;512] = std::array::from_fn(|i| (i as f32*0.07).sin()*0.3);
    let mut whole_l=[0.0;512]; let mut whole_r=[0.0;512];
    graph.render_pcm_quantum_zero_heap(&mut PcmRenderState::default(),
        &[PcmVoiceBlock {slot:0,binding:binding(1),samples:&source,gain:1.0}], &[],budget(),&mut whole_l,&mut whole_r).unwrap();
    let mut state = PcmRenderState::default();
    let mut split_l=[0.0;512];let mut split_r=[0.0;512];
    let first=graph.render_pcm_quantum_zero_heap(&mut state,
        &[PcmVoiceBlock {slot:0,binding:binding(1),samples:&source[..173],gain:1.0}], &[],budget(),&mut split_l[..173],&mut split_r[..173]).unwrap();
    let next=graph.render_pcm_quantum_zero_heap(&mut state,
        &[PcmVoiceBlock {slot:0,binding:binding(1),samples:&source[173..],gain:1.0}], &[],budget(),&mut split_l[173..],&mut split_r[173..]).unwrap();
    assert!(first.histories_reset); assert!(!next.histories_reset);
    assert_eq!(whole_l,split_l);assert_eq!(whole_r,split_r);
    assert!(whole_l.iter().any(|v|v.abs()>0.001));
    assert_ne!(whole_l,whole_r);
}
#[test]
fn invalid_binding_samples_and_budget_leave_output_and_state_unchanged() {
    let graph=graph();let mut state=PcmRenderState::default();
    let mut left=[9.0;4];let mut right=[8.0;4];
    let bad=[f32::NAN,0.0,0.0,0.0];
    assert_eq!(graph.render_pcm_quantum_zero_heap(&mut state,
        &[PcmVoiceBlock{slot:0,binding:binding(1),samples:&bad,gain:1.0}],&[],budget(),&mut left,&mut right),Err(PcmRenderError::NumericInput));
    assert_eq!(left,[9.0;4]);assert_eq!(right,[8.0;4]);assert!(!state.initialized);
    let good=[0.2;4]; let mut wrong=binding(1);wrong.node_id=9;
    assert_eq!(graph.render_pcm_quantum_zero_heap(&mut state,
        &[PcmVoiceBlock{slot:0,binding:wrong,samples:&good,gain:1.0}],&[],budget(),&mut left,&mut right),Err(PcmRenderError::Binding));
    let mut rate=budget();rate.sample_rate=44_100;
    assert_eq!(graph.render_pcm_quantum_zero_heap(&mut state,&[],&[],rate,&mut left,&mut right),Err(PcmRenderError::UnsupportedRate));
    let mut bad_graph=graph;bad_graph.emitter_count=MAX_SCENE_AUDIO_EMITTERS+1;
    assert_eq!(bad_graph.render_pcm_quantum_zero_heap(&mut state,&[],&[],budget(),&mut left,&mut right),Err(PcmRenderError::Budget));
    assert_eq!(left,[9.0;4]);assert!(!state.initialized);
}
#[test]
fn reassignment_and_forbidden_sources_do_not_retain_old_pcm_tails() {
    let mut graph=graph();let mut state=PcmRenderState::default();
    let source=[0.4;256];let zero=[0.0;256];let mut l=[0.0;256];let mut r=[0.0;256];
    graph.render_pcm_quantum_zero_heap(&mut state,
        &[PcmVoiceBlock{slot:0,binding:binding(1),samples:&source,gain:1.0}],&[],budget(),&mut l,&mut r).unwrap();
    let report=graph.render_pcm_quantum_zero_heap(&mut state,
        &[PcmVoiceBlock{slot:0,binding:binding(2),samples:&zero,gain:1.0}],&[],budget(),&mut l,&mut r).unwrap();
    assert!(report.histories_reset);assert!(l.iter().chain(&r).all(|v|v.abs()<1e-10));
    graph.emitters[0].deontic_permit=2;
    let report=graph.render_pcm_quantum_zero_heap(&mut state,
        &[PcmVoiceBlock{slot:0,binding:binding(2),samples:&source,gain:1.0}],&[],budget(),&mut l,&mut r).unwrap();
    assert!(report.histories_reset);assert_eq!(report.active_voices,0);
    assert!(l.iter().chain(&r).all(|v|v.abs()<1e-10));
}
