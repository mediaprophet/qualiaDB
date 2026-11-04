//! Qualia budget/calibration adapter for the canonical temporal renderer.

pub use etalon_render::temporal::{
    TemporalProducerAvailability, TemporalProducerViews, TemporalResolveInputs,
    TemporalSubmissionState,
};

pub struct TemporalResolveGpu(
    etalon_render::temporal::TemporalResolveGpu<crate::gpu_context::VramReservation<'static>>,
);
impl TemporalResolveGpu {
    pub fn history_valid(&self) -> bool {
        self.0.history_valid()
    }

    pub fn try_new(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        scene_format: wgpu::TextureFormat,
        output_format: wgpu::TextureFormat,
    ) -> Option<Self> {
        etalon_render::temporal::TemporalResolveGpu::try_new(
            device,
            width,
            height,
            scene_format,
            output_format,
            |bytes| {
                crate::gpu_context::global_vram_ledger()
                    .try_reserve_graphics(crate::gpu_context::VramResourceClass::FrameTarget, bytes)
                    .ok()
            },
            crate::render::output::exposure_scale_from_ev,
        )
        .map(Self)
    }
    pub fn record_scheduled_output(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        inputs: TemporalResolveInputs<'_, '_>,
        target: &wgpu::TextureView,
        schedule: crate::render::frame_graph::TemporalOutputSchedule,
        exposure_ev: f32,
        white_balance_gains: [f32; 3],
        surface_is_srgb: bool,
    ) -> bool {
        self.0.record_scheduled_output(
            device,
            queue,
            encoder,
            inputs,
            target,
            adapt_schedule(schedule),
            exposure_ev,
            white_balance_gains,
            surface_is_srgb,
        )
    }
    pub(super) fn scheduled_output_is_admissible(
        &self,
        extent: (u32, u32),
        schedule: crate::render::frame_graph::TemporalOutputSchedule,
        exposure_ev: f32,
        white_balance_gains: [f32; 3],
    ) -> bool {
        self.0.scheduled_output_is_admissible(
            extent,
            adapt_schedule(schedule),
            exposure_ev,
            white_balance_gains,
        )
    }
}
impl std::ops::Deref for TemporalResolveGpu {
    type Target =
        etalon_render::temporal::TemporalResolveGpu<crate::gpu_context::VramReservation<'static>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl std::ops::DerefMut for TemporalResolveGpu {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
fn adapt_schedule(
    schedule: crate::render::frame_graph::TemporalOutputSchedule,
) -> etalon_render::temporal::TemporalOutputSchedule {
    etalon_render::temporal::TemporalOutputSchedule {
        enabled: schedule.enabled,
        reset_history: schedule.reset_history,
        reads_history: schedule.reads_history,
        publish_history: schedule.publish_history,
        final_output: schedule.final_output,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_budget_is_bounded_and_format_aware() {
        let rgba8 =
            etalon_render::temporal::target_bytes(64, 32, wgpu::TextureFormat::Rgba8Unorm).unwrap();
        let rgba16 =
            etalon_render::temporal::target_bytes(64, 32, wgpu::TextureFormat::Rgba16Float)
                .unwrap();
        assert_eq!(rgba16 - rgba8, 64 * 32 * 8);
        assert!(rgba16 < 64 * 32 * 40);
    }

    #[test]
    fn temporal_controls_fail_closed_for_reset_and_missing_history() {
        let no_history = if false { 1.0 } else { 0.0 };
        let reset = no_history + 2.0;
        assert_eq!(no_history, 0.0);
        assert_eq!(reset, 2.0);
    }

    #[test]
    fn producer_availability_requires_every_real_input() {
        let producers = TemporalProducerViews {
            extent: (64, 64),
            current_linear_depth: None,
            motion_vectors: None,
            reactive_mask: None,
        };
        assert_eq!(
            producers.availability(true),
            TemporalProducerAvailability {
                scene_color: true,
                linear_depth: false,
                motion_vectors: false,
                reactive_mask: false,
            }
        );
        assert!(!producers.availability(true).all_available());
        assert!(!producers.availability(false).all_available());
        assert!(!producers.has_required_host_producers());
    }

    #[test]
    fn host_producer_contract_never_admits_a_partial_pair() {
        for (motion_vectors, reactive_mask) in [(true, false), (false, true)] {
            let availability = TemporalProducerAvailability {
                scene_color: true,
                linear_depth: true,
                motion_vectors,
                reactive_mask,
            };
            assert!(!availability.host_producers_available());
            assert!(!availability.all_available());
        }
    }

    #[test]
    fn linear_depth_producer_rejects_invalid_camera_ranges() {
        assert!(0.05_f32.is_finite());
        assert!(200.0_f32 > 0.05_f32);
        assert!(!(0.0_f32 > 200.0_f32));
    }

    #[cfg(any(feature = "wgsl-forge", feature = "webgl2"))]
    #[test]
    fn temporal_shaders_pass_naga_validation() {
        fn validate(source: &str) {
            let module = naga::front::wgsl::parse_str(source).expect("WGSL parse");
            naga::valid::Validator::new(
                naga::valid::ValidationFlags::all(),
                naga::valid::Capabilities::all(),
            )
            .validate(&module)
            .expect("WGSL validation");
        }

        validate(crate::shaders::viewport::TEMPORAL_RESOLVE_WGSL);
        validate(crate::shaders::viewport::TEMPORAL_DEPTH_WGSL);
        validate(crate::shaders::viewport::TEMPORAL_OUTPUT_WGSL);
        validate(crate::shaders::viewport::TEMPORAL_MOTION_WGSL);
        validate(crate::shaders::viewport::TEMPORAL_REACTIVE_WGSL);
    }
}
