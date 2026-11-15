//! Governance, cooperative, audio, scene, research, hypermedia, and social dispatch arms.

use super::{audio, cooperative, governance, hypermedia, ids, render, research, social};
use vibe::{Diagnostic, Span, Value};

pub(super) fn try_dispatch(
    id: &str,
    args: &Value,
    span: Span,
) -> Option<Result<Value, Diagnostic>> {
    let res = match id {
        ids::CAPABILITY_GRANT => governance::capability_grant(args, span),
        ids::CAPABILITY_REVOKE => governance::capability_revoke(args, span),
        ids::CAPABILITY_TEST_GATING => governance::capability_test_gating(args, span),
        ids::CAPABILITY_AUDIT => governance::capability_audit(args, span),
        ids::CAPABILITY_DECLARE => governance::capability_declare(args, span),
        ids::SENTINEL_INSPECT => governance::sentinel_inspect(args, span),
        ids::SENTINEL_GATE => governance::sentinel_gate(args, span),
        ids::AGENT_TRACE => governance::agent_trace(args, span),
        ids::AGENT_VERIFY => governance::agent_verify(args, span),
        ids::IDENTITY_CURRENT_USER => governance::current_user(args, span),
        ids::AGENCY_EVALUATE => governance::agency_evaluate(args, span),
        ids::COOPERATIVE_DELEGATION_PERMITS => cooperative::permits(args, span),
        ids::COOPERATIVE_WORK_BOARD_PROJECT => cooperative::board_project(args, span),
        "CooperativeLifecycle.evaluate" | "cooperative.lifecycle.evaluate" => {
            cooperative::evaluate_lifecycle(args, span)
        }
        "CooperativeAssertions.review" | "cooperative.assertions.review" => {
            cooperative::review_assertion(args, span)
        }
        "CooperativeInstruments.evaluate" | "cooperative.instruments.evaluate" => {
            cooperative::evaluate_instrument_readiness(args, span)
        }
        "CooperativePool.clear_bps" | "cooperative.pool.clear_bps" => {
            cooperative::clear_pool_bps(args, span)
        }
        ids::AUDIO_SPECTRUM => audio::spectrum(args, span),
        ids::AUDIO_OSCILLATOR => audio::dsp::oscillator(args, span),
        ids::AUDIO_ENVELOPE => audio::dsp::envelope(args, span),
        ids::AUDIO_FILTER => audio::dsp::filter(args, span),
        ids::AUDIO_LFO => audio::dsp::lfo(args, span),
        ids::AUDIO_DELAY => audio::dsp::delay(args, span),
        ids::AUDIO_REVERB => audio::dsp::reverb(args, span),
        ids::AUDIO_COMPRESSOR => audio::dsp::compressor(args, span),
        ids::AUDIO_EQ => audio::dsp::eq(args, span),
        ids::AUDIO_MIDI_NOTE => audio::dsp::midi_note(args, span),
        ids::AUDIO_QUANTIZE => audio::dsp::quantize(args, span),
        ids::AUDIO_TRANSPOSE => audio::dsp::transpose(args, span),
        ids::AUDIO_TRANSPORT => audio::dsp::transport(args, span),
        ids::AUDIO_WAVEFORM_METER => audio::dsp::waveform_meter(args, span),
        ids::AUDIO_PHASE_METER => audio::dsp::phase_meter(args, span),
        ids::AUDIO_LOUDNESS_METER => audio::dsp::loudness_meter(args, span),
        ids::AUDIO_EPISTEMIC_TEMPERATURE_FROM_Q => audio::epistemic_temperature_from_q(args, span),
        ids::AUDIO_EPISTEMIC_FM_INDEX => audio::epistemic_fm_index(args, span),
        ids::AUDIO_SIGMA_DOMINANT_FREQUENCY => audio::sigma_dominant_frequency(args, span),
        ids::AUDIO_PARAMETRIC_SAMPLE => audio::parametric_sample(args, span),
        ids::AUDIO_BIN_TO_FREQ_LINEAR => audio::bin_to_freq_linear(args, span),
        ids::AUDIO_BIN_TO_FREQ_LOG => audio::bin_to_freq_log(args, span),
        ids::SCENE_CREATE => render::scene_create(args, span),
        ids::SCENE_ADD_NODE => render::scene_add_node(args, span),
        ids::SCENE_SET_TRANSFORM => render::scene_set_transform(args, span),
        ids::SCENE_SET_MESH => render::scene_set_mesh(args, span),
        ids::SCENE_ADD_CAMERA => render::scene_add_camera(args, span),
        ids::SCENE_RENDER => render::scene_render(args, span),
        ids::SCENE_SET_VIEWPORT => render::scene_set_viewport(args, span),
        ids::SCENE_SET_CLEAR_COLOUR => render::scene_set_clear_colour(args, span),
        ids::SCENE_CAPTURE_FRAME => render::scene_capture_frame(args, span),
        ids::SCENE_ADD_LIGHT => render::scene_add_light(args, span),
        ids::SCENE_LINK_SEMANTIC => render::scene_link_semantic(args, span),
        ids::SCENE_DUPLICATE_NODE => render::scene_duplicate_node(args, span),
        ids::SCENE_SET_RENDER_BUDGET => render::scene_set_render_budget(args, span),
        ids::SCENE_IK_LOOK_AT => render::scene_ik_look_at(args, span),
        ids::SCENE_IK_CCD => render::scene_ik_ccd(args, span),
        ids::SCENE_SMOOTH_DAMP => render::scene_smooth_damp(args, span),
        ids::SCENE_SMOOTH_DAMP_VEC3 => render::scene_smooth_damp_vec3(args, span),
        ids::SCENE_LERP_CAMERA => render::scene_lerp_camera(args, span),
        ids::SCENE_CAMERA_FRAME_NODE => render::scene_camera_frame_node(args, span),
        ids::RESEARCH_NEW => research::research_new(args, span),
        ids::RESEARCH_SET_PURPOSE => research::research_set_purpose(args, span),
        ids::RESEARCH_DEFINE_SCOPE => research::research_define_scope(args, span),
        ids::RESEARCH_ADD_CONSTRAINT => research::research_add_constraint(args, span),
        ids::RESEARCH_ADD_QUESTION => research::research_add_question(args, span),
        ids::RESEARCH_LINK_QUESTIONS => research::research_link_questions(args, span),
        ids::RESEARCH_ADD_CORPUS_ITEM => research::research_add_corpus_item(args, span),
        ids::RESEARCH_IMPORT_LITERATURE => research::research_import_literature(args, span),
        ids::RESEARCH_IMPORT_DATASET => research::research_import_dataset(args, span),
        ids::RESEARCH_SET_CORPUS_CONFIDENCE => research::research_set_corpus_confidence(args, span),
        ids::RESEARCH_EXTRACT_FROM_CORPUS => research::research_extract_from_corpus(args, span),
        ids::RESEARCH_INFER_DARK_LINK => research::research_infer_dark_link(args, span),
        ids::RESEARCH_DETECT_PROVENANCE_GAPS => {
            research::research_detect_provenance_gaps(args, span)
        }
        ids::RESEARCH_DETECT_CONCEALMENT => research::research_detect_concealment(args, span),
        ids::RESEARCH_CONFIRM_DARK_LINK => research::research_confirm_dark_link(args, span),
        ids::RESEARCH_REFUTE_DARK_LINK => research::research_refute_dark_link(args, span),
        ids::RESEARCH_MAKE_INFERENCE => research::research_make_inference(args, span),
        ids::RESEARCH_CHAIN_INFERENCE => research::research_chain_inference(args, span),
        ids::RESEARCH_SET_INFERENCE_CONFIDENCE => {
            research::research_set_inference_confidence(args, span)
        }
        ids::RESEARCH_VALIDATE_INFERENCE => research::research_validate_inference(args, span),
        ids::RESEARCH_NEW_INVESTIGATION => research::research_new_investigation(args, span),
        ids::RESEARCH_COLLECT_EVIDENCE => research::research_collect_evidence(args, span),
        ids::RESEARCH_SET_RELIABILITY => research::research_set_reliability(args, span),
        ids::RESEARCH_PROPOSE_HYPOTHESIS => research::research_propose_hypothesis(args, span),
        ids::RESEARCH_EVALUATE_EVIDENCE => research::research_evaluate_evidence(args, span),
        ids::RESEARCH_CREATE_TIMELINE => research::research_create_timeline(args, span),
        ids::RESEARCH_ADD_LINK => research::research_add_link(args, span),
        ids::RESEARCH_FIND_PATH => research::research_find_path(args, span),
        ids::RESEARCH_CREATE_HYPOTHESIS_GRAPH => {
            research::research_create_hypothesis_graph(args, span)
        }
        ids::RESEARCH_CONTRIBUTE_EVALUATION => research::research_contribute_evaluation(args, span),
        ids::RESEARCH_BRIDGE_DARK_LINK => research::research_bridge_dark_link(args, span),
        ids::RESEARCH_REFRAME_HYPOTHESIS => research::research_reframe_hypothesis(args, span),
        ids::RESEARCH_MERGE_HYPOTHESES => research::research_merge_hypotheses(args, span),
        ids::RESEARCH_FLAG_GAP => research::research_flag_gap(args, span),
        ids::RESEARCH_CLOSE_GAP => research::research_close_gap(args, span),
        ids::RESEARCH_CREATE_REVISION => research::research_create_revision(args, span),
        ids::RESEARCH_DIFF_REVISIONS => research::research_diff_revisions(args, span),
        ids::RESEARCH_SUBSCRIBE_UPDATES => research::research_subscribe_updates(args, span),
        ids::RESEARCH_CREATE_ASSESSMENT => research::research_create_assessment(args, span),
        ids::RESEARCH_SET_EPISTEMIC_MODE => research::research_set_epistemic_mode(args, span),
        ids::RESEARCH_SET_REALITY_CATEGORY => research::research_set_reality_category(args, span),
        ids::RESEARCH_CLASSIFY_REALITY => research::research_classify_reality(args, span),
        ids::RESEARCH_DETECT_BLENDED => research::research_detect_blended(args, span),
        ids::RESEARCH_DETECT_DECEPTIVE_FICTION => {
            research::research_detect_deceptive_fiction(args, span)
        }
        ids::RESEARCH_TRACE_FICTION => research::research_trace_fiction(args, span),
        ids::RESEARCH_ASSESS_SENTIMENT => research::research_assess_sentiment(args, span),
        ids::RESEARCH_DETECT_SENTIMENT_MANIPULATION => {
            research::research_detect_sentiment_manipulation(args, span)
        }
        ids::RESEARCH_DETECT_PERFORMED_SENTIMENT => {
            research::research_detect_performed_sentiment(args, span)
        }
        ids::RESEARCH_MAP_SENTIMENT_NETWORK => research::research_map_sentiment_network(args, span),
        ids::RESEARCH_ANALYSE_SENTIMENT_TRENDS => {
            research::research_analyse_sentiment_trends(args, span)
        }
        ids::RESEARCH_REGISTER_PERSPECTIVE => research::research_register_perspective(args, span),
        ids::RESEARCH_ADD_BIAS => research::research_add_bias(args, span),
        ids::RESEARCH_COMPARE_PERSPECTIVES => research::research_compare_perspectives(args, span),
        ids::RESEARCH_DETECT_PERSPECTIVE_CONFLICT => {
            research::research_detect_perspective_conflict(args, span)
        }
        ids::RESEARCH_RECONCILE_PERSPECTIVES => {
            research::research_reconcile_perspectives(args, span)
        }
        ids::RESEARCH_ASSESS_INTENTIONALITY => research::research_assess_intentionality(args, span),
        ids::RESEARCH_CLASSIFY_MISTAKE => research::research_classify_mistake(args, span),
        ids::RESEARCH_DEFINE_SOCIAL_DYNAMICS => {
            research::research_define_social_dynamics(args, span)
        }
        ids::RESEARCH_DEFINE_ECONOMIC_DYNAMICS => {
            research::research_define_economic_dynamics(args, span)
        }
        ids::RESEARCH_DEFINE_SPATIOTEMPORAL_DYNAMICS => {
            research::research_define_spatiotemporal_dynamics(args, span)
        }
        ids::RESEARCH_ANALYSE_SOCIAL_NETWORK => {
            research::research_analyse_social_network(args, span)
        }
        ids::RESEARCH_ANALYSE_INEQUALITY => research::research_analyse_inequality(args, span),
        ids::RESEARCH_ANALYSE_DIFFUSION => research::research_analyse_diffusion(args, span),
        ids::RESEARCH_ASSESS_GROUNDING => research::research_assess_grounding(args, span),
        ids::RESEARCH_VERIFY_GROUNDING => research::research_verify_grounding(args, span),
        ids::RESEARCH_DETECT_UNGROUNDED_BEHAVIOUR => {
            research::research_detect_ungrounded_behaviour(args, span)
        }
        ids::RESEARCH_CREATE_UG_INSTANCE => research::research_create_ug_instance(args, span),
        ids::RESEARCH_SET_UG_CAUSE => research::research_set_ug_cause(args, span),
        ids::RESEARCH_SET_UG_CONSEQUENCE => research::research_set_ug_consequence(args, span),
        ids::RESEARCH_SET_UG_DETECTION => research::research_set_ug_detection(args, span),
        ids::RESEARCH_SET_UG_MITIGATION => research::research_set_ug_mitigation(args, span),
        ids::RESEARCH_SET_UG_CALIBRATION => research::research_set_ug_calibration(args, span),
        ids::RESEARCH_DETECT_UG_PATTERNS => research::research_detect_ug_patterns(args, span),
        ids::IMAGE_NEW => hypermedia::image_new(args, span),
        ids::IMAGE_ADD_LAYER => hypermedia::image_add_layer(args, span),
        ids::IMAGE_REMOVE_LAYER => hypermedia::image_remove_layer(args, span),
        ids::IMAGE_SET_PIXEL => hypermedia::image_set_pixel(args, span),
        ids::IMAGE_FILL => hypermedia::image_fill(args, span),
        ids::IMAGE_BRUSH => hypermedia::image_brush(args, span),
        ids::IMAGE_APPLY_FILTER => hypermedia::image_apply_filter(args, span),
        ids::IMAGE_SET_OPACITY => hypermedia::image_set_opacity(args, span),
        ids::IMAGE_SET_BLEND_MODE => hypermedia::image_set_blend_mode(args, span),
        ids::IMAGE_SET_VISIBLE => hypermedia::image_set_visible(args, span),
        ids::IMAGE_SET_MASK => hypermedia::image_set_mask(args, span),
        ids::IMAGE_CLEAR_MASK => hypermedia::image_clear_mask(args, span),
        ids::IMAGE_COMPOSITE => hypermedia::image_composite(args, span),
        ids::IMAGE_ADD_SELECTION => hypermedia::image_add_selection(args, span),
        ids::IMAGE_CLEAR_SELECTIONS => hypermedia::image_clear_selections(args, span),
        ids::VIDEO_NEW_PROJECT => hypermedia::video_new_project(args, span),
        ids::VIDEO_ADD_TRACK => hypermedia::video_add_track(args, span),
        ids::VIDEO_ADD_CLIP => hypermedia::video_add_clip(args, span),
        ids::VIDEO_TRIM_CLIP => hypermedia::video_trim_clip(args, span),
        ids::VIDEO_SET_SPEED => hypermedia::video_set_speed(args, span),
        ids::VIDEO_COLOUR_GRADE => hypermedia::video_colour_grade(args, span),
        ids::VIDEO_ADD_TRANSITION => hypermedia::video_add_transition(args, span),
        ids::VIDEO_SET_RENDER_FORMAT => hypermedia::video_set_render_format(args, span),
        ids::VIDEO_SET_RENDER_BITRATE => hypermedia::video_set_render_bitrate(args, span),
        ids::VIDEO_REMOVE_CLIP => hypermedia::video_remove_clip(args, span),
        ids::THREE_D_ADD_OBJECT => hypermedia::three_d_add_object(args, span),
        ids::THREE_D_SET_TRANSFORM => hypermedia::three_d_set_transform(args, span),
        ids::THREE_D_SET_MATERIAL => hypermedia::three_d_set_material(args, span),
        ids::THREE_D_ADD_CAMERA => hypermedia::three_d_add_camera(args, span),
        ids::THREE_D_ADD_LIGHT => hypermedia::three_d_add_light(args, span),
        ids::THREE_D_ADD_RIG => hypermedia::three_d_add_rig(args, span),
        ids::THREE_D_ADD_ANIMATION => hypermedia::three_d_add_animation(args, span),
        ids::THREE_D_SET_MESH => hypermedia::three_d_set_mesh(args, span),
        ids::HBBTV_NEW_APP => hypermedia::hbbtv_new_app(args, span),
        ids::HBBTV_ADD_PAGE => hypermedia::hbbtv_add_page(args, span),
        ids::HBBTV_NAVIGATE => hypermedia::hbbtv_navigate(args, span),
        ids::HBBTV_SET_STATE => hypermedia::hbbtv_set_state(args, span),
        ids::SECOND_SCREEN_SYNC => hypermedia::second_screen_sync(args, span),
        ids::INTERACTIVE_ADD_TRIGGER => hypermedia::interactive_add_trigger(args, span),
        ids::INTERACTIVE_ADD_SOCIAL_POST => hypermedia::interactive_add_social_post(args, span),
        ids::WORLD_NEW => hypermedia::world_new(args, span),
        ids::WORLD_ADD_OBJECT => hypermedia::world_add_object(args, span),
        ids::WORLD_ADD_PORTAL => hypermedia::world_add_portal(args, span),
        ids::WORLD_ADD_AVATAR => hypermedia::world_add_avatar(args, span),
        ids::WORLD_SET_GRAVITY => hypermedia::world_set_gravity(args, span),
        ids::WORLD_OBJECT_APPLY_FORCE => hypermedia::world_object_apply_force(args, span),
        ids::WORLD_OBJECT_STEP_PHYSICS => hypermedia::world_object_step_physics(args, span),
        ids::PORTAL_SET_TARGET => hypermedia::portal_set_target(args, span),
        ids::PORTAL_ACTIVATE => hypermedia::portal_activate(args, span),
        ids::PORTAL_DEACTIVATE => hypermedia::portal_deactivate(args, span),
        ids::AVATAR_MOVE => hypermedia::avatar_move(args, span),
        ids::AVATAR_SET_APPEARANCE => hypermedia::avatar_set_appearance(args, span),
        ids::DMX_NEW_UNIVERSE => hypermedia::dmx_new_universe(args, span),
        ids::DMX_SET_CHANNEL => hypermedia::dmx_set_channel(args, span),
        ids::DMX_ADD_FIXTURE => hypermedia::dmx_add_fixture(args, span),
        ids::DMX_FIXTURE_SET_COLOUR => hypermedia::dmx_fixture_set_colour(args, span),
        ids::DMX_FIXTURE_SET_INTENSITY => hypermedia::dmx_fixture_set_intensity(args, span),
        ids::DMX_FIXTURE_SET_PAN_TILT => hypermedia::dmx_fixture_set_pan_tilt(args, span),
        ids::DMX_NEW_CUE => hypermedia::dmx_new_cue(args, span),
        ids::DMX_CUE_SET_CHANNEL => hypermedia::dmx_cue_set_channel(args, span),
        ids::DMX_CUE_SET_FADE => hypermedia::dmx_cue_set_fade(args, span),
        ids::DMX_NEW_CUE_STACK => hypermedia::dmx_new_cue_stack(args, span),
        ids::DMX_CUE_STACK_ADD => hypermedia::dmx_cue_stack_add(args, span),
        ids::DMX_CUE_STACK_GO => hypermedia::dmx_cue_stack_go(args, span),
        ids::DMX_CUE_STACK_GO_BACK => hypermedia::dmx_cue_stack_go_back(args, span),
        ids::DMX_CUE_STACK_RESET => hypermedia::dmx_cue_stack_reset(args, span),
        ids::SOCIAL_GINI => social::gini(args, span),
        ids::SOCIAL_LORENZ => social::lorenz(args, span),
        ids::SOCIAL_DEGREE_CENTRALITY => social::degree_centrality(args, span),
        ids::FORENSIC_MALFEASANCE_DELTA => social::malfeasance_delta(args, span),
        ids::FORENSIC_NARRATIVE_DIVERGENCE => social::narrative_divergence(args, span),
        _ => return None,
    };
    Some(res)
}
