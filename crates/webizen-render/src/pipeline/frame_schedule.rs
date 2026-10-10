//! Frame graph scheduling and capability integration for the Webizen renderer.
//!
//! Exposes pass compilation, budget enforcement, and quality profile adaptation
//! across native and WASM render adapters. Strictly zero-heap in execution.

#[cfg(feature = "qualia")]
use qualia_core_db::render::frame_graph::{
    CompiledSchedule, FrameGraphBuilder, FrameGraphError, TemporalOutputSchedule,
};
#[cfg(feature = "qualia")]
use qualia_core_db::render::quality_profiles::RenderQualityProfile;
#[cfg(feature = "qualia")]
use qualia_core_db::render::temporal_resolve::TemporalResolveConfig;

/// High-level renderer pass scheduler.
#[cfg(feature = "qualia")]
pub struct WebizenFrameScheduler {
    builder: FrameGraphBuilder,
}

/// Return the only temporal handoff that a backend may submit for a compiled frame.
///
/// The scheduler does not own GPU attachments. It supplies the ordered contract; the host must
/// provide real producer views through `PortalGpu::record_scheduled_temporal_output`.
#[cfg(feature = "qualia")]
pub fn temporal_submission_schedule(
    schedule: &CompiledSchedule,
) -> Option<TemporalOutputSchedule> {
    (schedule.temporal.is_complete() && schedule.temporal_order_is_valid())
        .then_some(schedule.temporal)
}

#[cfg(feature = "qualia")]
impl Default for WebizenFrameScheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "qualia")]
impl WebizenFrameScheduler {
    pub const fn new() -> Self {
        Self {
            builder: FrameGraphBuilder::new(),
        }
    }

    /// Build a frame execution schedule from the given quality profile and viewport dimensions.
    pub fn plan_frame(
        &mut self,
        profile: &RenderQualityProfile,
        viewport_width: u32,
        viewport_height: u32,
        vram_budget: Option<u64>,
    ) -> Result<CompiledSchedule, FrameGraphError> {
        self.builder = FrameGraphBuilder::new();
        self.builder
            .configure_from_profile(profile, viewport_width, viewport_height)?;
        self.builder.compile(vram_budget)
    }

    /// Build a schedule with an explicitly admitted temporal resolve/output path.
    ///
    /// Callers must declare motion-vector and reactive-mask ownership. Missing inputs preserve
    /// the existing direct/bloom output schedule instead of silently accumulating stale history.
    pub fn plan_frame_with_temporal(
        &mut self,
        profile: &RenderQualityProfile,
        viewport_width: u32,
        viewport_height: u32,
        vram_budget: Option<u64>,
        temporal: TemporalResolveConfig,
    ) -> Result<CompiledSchedule, FrameGraphError> {
        self.builder = FrameGraphBuilder::new();
        self.builder.configure_from_profile_with_temporal(
            profile,
            viewport_width,
            viewport_height,
            temporal,
        )?;
        self.builder.compile(vram_budget)
    }

    /// Access the underlying builder to inject custom passes or resources.
    pub fn builder_mut(&mut self) -> &mut FrameGraphBuilder {
        &mut self.builder
    }
}

#[cfg(all(test, feature = "qualia"))]
mod tests {
    use super::*;
    use qualia_core_db::render::quality_profiles::QualityTier;

    #[test]
    fn scheduler_produces_valid_plan_from_high_profile() {
        let mut scheduler = WebizenFrameScheduler::new();
        let profile = RenderQualityProfile {
            tier: QualityTier::High,
            render_scale_bps: 10_000,
            shadows_enabled: true,
            shadow_map_dimension: 2048,
            shadow_cascade_count: 2,
            ambient_occlusion_enabled: true,
            ao_scale_bps: 5_000,
            bloom_enabled: true,
            bloom_levels: 3,
            texture_residency_bytes: 1024 * 1024 * 1024,
            hdr_enabled: true,
            volume_projection_enabled: false,
        };

        let schedule = scheduler
            .plan_frame(&profile, 1920, 1080, None)
            .expect("plan frame");

        assert!(schedule.pass_count > 0);
        assert!(schedule.total_resource_bytes > 0);
        assert!(temporal_submission_schedule(&schedule).is_none());
    }

    #[test]
    fn scheduler_can_admit_temporal_output_only_with_declared_inputs() {
        let mut scheduler = WebizenFrameScheduler::new();
        let profile = RenderQualityProfile {
            tier: QualityTier::Balanced,
            render_scale_bps: 8_500,
            shadows_enabled: false,
            shadow_map_dimension: 0,
            shadow_cascade_count: 0,
            ambient_occlusion_enabled: false,
            ao_scale_bps: 0,
            bloom_enabled: false,
            bloom_levels: 0,
            texture_residency_bytes: 128 * 1024 * 1024,
            hdr_enabled: false,
            volume_projection_enabled: false,
        };
        let schedule = scheduler
            .plan_frame_with_temporal(
                &profile,
                800,
                450,
                None,
                TemporalResolveConfig {
                    enabled: true,
                    motion_vectors_available: true,
                    reactive_mask_available: true,
                    history_valid: false,
                    reset_history: true,
                },
            )
            .expect("temporal schedule");
        let passes = &schedule.passes[..schedule.pass_count];
        assert!(passes.iter().any(|pass| {
            *pass == Some(qualia_core_db::render::frame_graph::PassId::TemporalResolve)
        }));
        assert!(passes.iter().any(|pass| {
            *pass == Some(qualia_core_db::render::frame_graph::PassId::HistoryPublication)
        }));
        let temporal = temporal_submission_schedule(&schedule).expect("ordered temporal seam");
        assert!(temporal.enabled);
        assert!(temporal.reset_history);
        assert!(!temporal.reads_history);
        assert!(temporal.publish_history && temporal.final_output);
        assert!(schedule.temporal_order_is_valid());
    }
}
