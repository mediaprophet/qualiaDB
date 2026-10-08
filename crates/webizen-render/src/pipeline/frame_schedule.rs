//! Frame graph scheduling and capability integration for the Webizen renderer.
//!
//! Exposes pass compilation, budget enforcement, and quality profile adaptation
//! across native and WASM render adapters. Strictly zero-heap in execution.

#[cfg(feature = "qualia")]
use qualia_core_db::render::frame_graph::{CompiledSchedule, FrameGraphBuilder, FrameGraphError};
#[cfg(feature = "qualia")]
use qualia_core_db::render::quality_profiles::RenderQualityProfile;

/// High-level renderer pass scheduler.
#[cfg(feature = "qualia")]
pub struct WebizenFrameScheduler {
    builder: FrameGraphBuilder,
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
    }
}
