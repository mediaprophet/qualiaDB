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

#[cfg(feature = "qualia")]
use super::temporal_contract::{
    FrameExtent, TemporalCapabilityRefusal, TemporalContractError, TemporalFrameContract,
};

/// Errors from webizen temporal admission or the underlying Qualia graph compiler.
#[cfg(feature = "qualia")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WebizenFrameScheduleError {
    FrameGraph(FrameGraphError),
    Contract(TemporalContractError),
    Temporal(TemporalCapabilityRefusal),
}

#[cfg(feature = "qualia")]
impl From<FrameGraphError> for WebizenFrameScheduleError {
    fn from(error: FrameGraphError) -> Self {
        Self::FrameGraph(error)
    }
}

/// A compiled Qualia schedule carried together with the real producer contract that admitted it.
///
/// This type contains no owned GPU resource and performs no recording. An adapter may use the
/// borrowed producer views and `schedule` later at its existing Qualia GPU seam.
#[cfg(feature = "qualia")]
#[derive(Clone, Copy)]
pub struct WebizenFramePlan<'a> {
    pub schedule: CompiledSchedule,
    pub temporal: TemporalFrameContract<'a>,
}

#[cfg(feature = "qualia")]
impl<'a> WebizenFramePlan<'a> {
    pub const fn temporal_contract(&self) -> TemporalFrameContract<'a> {
        self.temporal
    }
}

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

    /// Build a Qualia schedule only after admitting real typed temporal producer views.
    ///
    /// The contract's views are borrowed and never copied into GPU resources. Missing producer
    /// capabilities, history lifecycle mistakes, and extent mismatches refuse deterministically
    /// before the existing Qualia frame graph is invoked.
    pub fn plan_frame_with_temporal_contract<'a>(
        &mut self,
        profile: &RenderQualityProfile,
        viewport_width: u32,
        viewport_height: u32,
        vram_budget: Option<u64>,
        temporal: TemporalFrameContract<'a>,
    ) -> Result<WebizenFramePlan<'a>, WebizenFrameScheduleError> {
        let expected_extent = FrameExtent::scaled(
            viewport_width,
            viewport_height,
            profile.render_scale_bps,
        )
        .map_err(WebizenFrameScheduleError::Contract)?;
        let admission = temporal
            .admit(expected_extent)
            .map_err(WebizenFrameScheduleError::Temporal)?;
        let schedule = self.plan_frame_with_temporal(
            profile,
            viewport_width,
            viewport_height,
            vram_budget,
            TemporalResolveConfig {
                enabled: true,
                motion_vectors_available: true,
                reactive_mask_available: true,
                history_valid: admission.reads_history,
                reset_history: admission.reset_history,
            },
        )?;
        if temporal_submission_schedule(&schedule).is_none() {
            return Err(WebizenFrameScheduleError::Temporal(
                TemporalCapabilityRefusal::QualiaScheduleIncomplete,
            ));
        }
        Ok(WebizenFramePlan { schedule, temporal })
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
    use super::super::temporal_contract::{
        TemporalHistoryContract, TemporalProducerCapabilities, TemporalProducerContract,
        TemporalResource,
    };

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

    #[test]
    fn typed_contract_connects_to_qualia_schedule_without_gpu_data() {
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
        let extent = FrameExtent::scaled(800, 450, profile.render_scale_bps).unwrap();
        let capabilities = TemporalProducerCapabilities {
            extent: Some(extent),
            linear_depth: false,
            motion_vectors: false,
            reactive_mask: false,
        };
        let contract = TemporalFrameContract::new(
            TemporalProducerContract::for_extent(extent),
            TemporalHistoryContract::first_frame(0),
        );
        assert_eq!(contract.producers.capabilities(), capabilities);
        let result = scheduler.plan_frame_with_temporal_contract(
            &profile,
            800,
            450,
            None,
            contract,
        );
        match result {
            Err(error) => assert_eq!(
                error,
                WebizenFrameScheduleError::Temporal(
                    TemporalCapabilityRefusal::MissingProducer {
                        resource: TemporalResource::LinearDepth,
                    }
                )
            ),
            Ok(_) => panic!("partial producer contract must refuse temporal scheduling"),
        }
    }
}
