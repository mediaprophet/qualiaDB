use super::*;
use crate::render::quality_profiles::{QualityTier, RenderQualityProfile};
use crate::render::temporal_resolve::TemporalResolveConfig;

#[test]
fn frame_graph_empty_compiles_cleanly() {
    let builder = FrameGraphBuilder::new();
    let schedule = builder.compile(None).expect("empty schedule");
    assert_eq!(schedule.pass_count, 0);
    assert_eq!(schedule.total_resource_bytes, 0);
}

#[test]
fn frame_graph_topological_order_is_deterministic() {
    let mut builder = FrameGraphBuilder::new();
    // Register forward pass before shadow pass
    builder
        .add_pass(
            PassNode::new(PassId::ForwardLighting)
                .with_read(FrameResourceId::ShadowCascade(0))
                .with_write(FrameResourceId::SceneColor),
        )
        .unwrap();
    builder
        .add_pass(
            PassNode::new(PassId::ShadowCascade(0)).with_write(FrameResourceId::ShadowCascade(0)),
        )
        .unwrap();

    let schedule = builder.compile(None).expect("valid DAG");
    assert_eq!(schedule.pass_count, 2);
    // Shadow must precede forward lighting regardless of insertion order
    assert_eq!(schedule.passes[0], Some(PassId::ShadowCascade(0)));
    assert_eq!(schedule.passes[1], Some(PassId::ForwardLighting));
}

#[test]
fn frame_graph_detects_cycles() {
    let mut builder = FrameGraphBuilder::new();
    builder
        .add_pass(
            PassNode::new(PassId::ShadowCascade(0))
                .with_read(FrameResourceId::SceneColor)
                .with_write(FrameResourceId::ShadowCascade(0)),
        )
        .unwrap();
    builder
        .add_pass(
            PassNode::new(PassId::ForwardLighting)
                .with_read(FrameResourceId::ShadowCascade(0))
                .with_write(FrameResourceId::SceneColor),
        )
        .unwrap();

    let result = builder.compile(None);
    assert_eq!(result.err(), Some(FrameGraphError::CyclicDependency));
}

#[test]
fn frame_graph_prunes_disabled_passes() {
    let mut builder = FrameGraphBuilder::new();
    builder
        .add_pass(
            PassNode::new(PassId::AoCompute)
                .with_write(FrameResourceId::AoVisibility)
                .with_enabled(false),
        )
        .unwrap();
    builder
        .add_pass(PassNode::new(PassId::ForwardLighting).with_write(FrameResourceId::SceneColor))
        .unwrap();

    let schedule = builder.compile(None).expect("schedule");
    assert_eq!(schedule.pass_count, 1);
    assert_eq!(schedule.passes[0], Some(PassId::ForwardLighting));
}

#[test]
fn frame_graph_configures_from_balanced_profile() {
    let mut builder = FrameGraphBuilder::new();
    let profile = RenderQualityProfile {
        tier: QualityTier::Balanced,
        render_scale_bps: 8_500,
        shadows_enabled: true,
        shadow_map_dimension: 1024,
        shadow_cascade_count: 2,
        ambient_occlusion_enabled: true,
        ao_scale_bps: 5_000,
        bloom_enabled: true,
        bloom_levels: 3,
        texture_residency_bytes: 512 * 1024 * 1024,
        hdr_enabled: true,
        volume_projection_enabled: false,
    };

    builder
        .configure_from_profile(&profile, 1920, 1080)
        .expect("configure profile");

    let schedule = builder.compile(None).expect("compiled schedule");
    assert!(schedule.pass_count >= 5);
    // Depth prepass and shadow cascades must precede forward lighting
    let forward_idx = schedule
        .passes
        .iter()
        .position(|&p| p == Some(PassId::ForwardLighting))
        .expect("forward pass present");
    let depth_idx = schedule
        .passes
        .iter()
        .position(|&p| p == Some(PassId::DepthPrepass))
        .expect("depth prepass present");
    assert!(depth_idx < forward_idx);

    // Bloom composite must be after forward lighting
    let bloom_idx = schedule
        .passes
        .iter()
        .position(|&p| p == Some(PassId::BloomComposite))
        .expect("bloom composite present");
    assert!(forward_idx < bloom_idx);
}

#[test]
fn frame_graph_configures_conservative_no_ao_no_hdr_fallback() {
    let mut builder = FrameGraphBuilder::new();
    let profile = RenderQualityProfile {
        tier: QualityTier::Conservative,
        render_scale_bps: 5_000,
        shadows_enabled: false,
        shadow_map_dimension: 0,
        shadow_cascade_count: 0,
        ambient_occlusion_enabled: false,
        ao_scale_bps: 0,
        bloom_enabled: false,
        bloom_levels: 0,
        texture_residency_bytes: 64 * 1024 * 1024,
        hdr_enabled: false,
        volume_projection_enabled: false,
    };

    builder
        .configure_from_profile(&profile, 1280, 720)
        .expect("configure conservative");

    let schedule = builder.compile(None).expect("conservative schedule");
    assert_eq!(schedule.passes[0], Some(PassId::ForwardLighting));
    assert_eq!(schedule.passes[1], Some(PassId::SdrOutputComposite));
    assert_eq!(schedule.pass_count, 2);
}

#[test]
fn frame_graph_budget_refusal_fails_closed() {
    let mut builder = FrameGraphBuilder::new();
    builder
        .add_resource(ResourceDesc {
            id: FrameResourceId::SceneColor,
            format: ResourceFormat::Rgba16Float,
            width: 3840,
            height: 2160,
            is_transient: false,
        })
        .unwrap();

    let bytes = 3840u64 * 2160 * 8;
    let budget_too_small = bytes / 2;
    let err = builder
        .compile(Some(budget_too_small))
        .expect_err("must refuse over-budget");
    assert_eq!(
        err,
        FrameGraphError::BudgetExceeded {
            required: bytes,
            budget: budget_too_small,
        }
    );
}

#[test]
fn temporal_output_schedule_resolves_before_history_and_final_transform() {
    let mut builder = FrameGraphBuilder::new();
    let profile = RenderQualityProfile {
        tier: QualityTier::Balanced,
        render_scale_bps: 10_000,
        shadows_enabled: false,
        shadow_map_dimension: 0,
        shadow_cascade_count: 0,
        ambient_occlusion_enabled: false,
        ao_scale_bps: 0,
        bloom_enabled: false,
        bloom_levels: 0,
        texture_residency_bytes: 256 * 1024 * 1024,
        hdr_enabled: false,
        volume_projection_enabled: false,
    };
    let temporal = TemporalResolveConfig {
        enabled: true,
        motion_vectors_available: true,
        reactive_mask_available: true,
        history_valid: true,
        reset_history: false,
    };

    builder
        .configure_from_profile_with_temporal(&profile, 1280, 720, temporal)
        .expect("configure temporal output");
    let schedule = builder.compile(None).expect("compile temporal output");
    let passes = &schedule.passes[..schedule.pass_count];
    let temporal_idx = passes
        .iter()
        .position(|pass| *pass == Some(PassId::TemporalResolve))
        .expect("temporal resolve pass");
    let history_idx = passes
        .iter()
        .position(|pass| *pass == Some(PassId::HistoryPublication))
        .expect("history publication pass");
    let output_idx = passes
        .iter()
        .position(|pass| *pass == Some(PassId::SdrOutputComposite))
        .expect("final output pass");
    assert!(temporal_idx < history_idx && history_idx < output_idx);
    assert_eq!(schedule.temporal.reset_history, false);
    assert!(schedule.temporal.is_complete());
    assert!(schedule.temporal_order_is_valid());
}

#[test]
fn temporal_schedule_falls_back_without_reactive_or_motion_inputs() {
    let mut builder = FrameGraphBuilder::new();
    let profile = RenderQualityProfile {
        tier: QualityTier::Conservative,
        render_scale_bps: 5_000,
        shadows_enabled: false,
        shadow_map_dimension: 0,
        shadow_cascade_count: 0,
        ambient_occlusion_enabled: false,
        ao_scale_bps: 0,
        bloom_enabled: false,
        bloom_levels: 0,
        texture_residency_bytes: 64 * 1024 * 1024,
        hdr_enabled: false,
        volume_projection_enabled: false,
    };
    builder
        .configure_from_profile_with_temporal(
            &profile,
            640,
            360,
            TemporalResolveConfig {
                enabled: true,
                motion_vectors_available: false,
                reactive_mask_available: false,
                history_valid: true,
                reset_history: false,
            },
        )
        .expect("configure fallback");
    let schedule = builder.compile(None).expect("compile fallback");
    assert!(!schedule.passes[..schedule.pass_count]
        .iter()
        .any(|pass| *pass == Some(PassId::TemporalResolve)));
    assert_eq!(schedule.passes[0], Some(PassId::ForwardLighting));
    assert_eq!(schedule.passes[1], Some(PassId::SdrOutputComposite));
    assert_eq!(schedule.temporal, TemporalOutputSchedule::disabled());
    assert!(schedule.temporal_order_is_valid());
}

#[test]
fn temporal_schedule_reset_never_reads_previous_history() {
    let mut builder = FrameGraphBuilder::new();
    let profile = RenderQualityProfile {
        tier: QualityTier::Balanced,
        render_scale_bps: 10_000,
        shadows_enabled: false,
        shadow_map_dimension: 0,
        shadow_cascade_count: 0,
        ambient_occlusion_enabled: false,
        ao_scale_bps: 0,
        bloom_enabled: true,
        bloom_levels: 2,
        texture_residency_bytes: 256 * 1024 * 1024,
        hdr_enabled: true,
        volume_projection_enabled: false,
    };
    builder
        .configure_from_profile_with_temporal(
            &profile,
            1280,
            720,
            TemporalResolveConfig {
                enabled: true,
                motion_vectors_available: true,
                reactive_mask_available: true,
                history_valid: false,
                reset_history: true,
            },
        )
        .expect("configure temporal reset");
    let schedule = builder.compile(None).expect("compile temporal reset");
    assert!(schedule.temporal.is_complete());
    assert!(schedule.temporal.reset_history);
    assert!(!schedule.temporal.reads_history);
    assert!(schedule.temporal_order_is_valid());
    assert!(schedule.pass_index(PassId::BloomComposite).is_some());
}

#[test]
fn temporal_schedule_rejects_reset_and_history_read_together() {
    let schedule = TemporalOutputSchedule {
        enabled: true,
        reset_history: true,
        reads_history: true,
        publish_history: true,
        final_output: true,
    };
    assert!(!schedule.is_valid());
    assert!(!schedule.is_complete());
    assert!(!schedule.with_reset_history().reads_history);
}
