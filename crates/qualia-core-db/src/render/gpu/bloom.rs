//! Bloom policy and output calibration adapter.
use super::*;
use crate::gpu_context::{global_vram_ledger, VramResourceClass};
pub(super) use etalon_render::bloom::{hdr_color_target_state, probe_hdr_format, run_bloom_passes};
/// `true` when portal bloom is enabled.
pub(super) fn portal_bloom_enabled() -> bool {
    let ledger = global_vram_ledger();
    universe_orchestrator().bloom_enabled(ComputeUniverse::Viewport, ledger.mode())
}

pub(super) fn create_bloom_chain(
    device: &etalon::Device,
    width: u32,
    height: u32,
    format: etalon::TextureFormat,
    exposure_ev: f32,
) -> Option<BloomChain> {
    if !portal_bloom_enabled() {
        return None;
    }
    let scale = crate::render::output::exposure_scale_from_ev(exposure_ev)?;
    etalon_render::bloom::create_bloom_chain(device, width, height, format, scale, |bytes| {
        global_vram_ledger()
            .try_reserve_graphics(VramResourceClass::FrameTarget, bytes)
            .ok()
    })
}
fn hdr_composite_available(a: bool, b: bool, c: bool, d: bool) -> bool {
    a && b && c && d
}
impl PortalGpu {
    /// Set bounded manual HDR exposure compensation in stops (EV).
    /// The value remains configured and is applied by whichever scene-output path is active.
    pub fn set_exposure_compensation(&mut self, compensation_ev: f32) -> bool {
        if !compensation_ev.is_finite() {
            return false;
        }
        let Some(exposure_scale) = crate::render::output::exposure_scale_from_ev(compensation_ev)
        else {
            return false;
        };
        let compensation_ev = compensation_ev.clamp(
            crate::render::output::MIN_HDR_EXPOSURE_EV,
            crate::render::output::MAX_HDR_EXPOSURE_EV,
        );
        if portal_bloom_enabled() != self.bloom_policy_snapshot {
            self.sync_bloom_targets();
        }
        self.hdr_exposure_ev = compensation_ev;
        if let Some(bloom) = self.bloom.as_mut() {
            bloom.exposure_scale = exposure_scale;
        }
        let output_available = self
            .output_chain
            .as_mut()
            .map(|output| output.set_exposure(&self.queue, compensation_ev))
            .unwrap_or(false);
        self.hdr_exposure_available() || output_available
    }

    /// Compatibility alias for the original HDR-specific API name.
    pub fn set_hdr_exposure_compensation(&mut self, compensation_ev: f32) -> bool {
        self.set_exposure_compensation(compensation_ev)
    }

    /// Set bounded pre-tone-map white balance, retaining controls across target rebuilds.
    pub fn set_white_balance(&mut self, temperature_ev: f32, tint_ev: f32) -> bool {
        let Some(gains) =
            crate::render::output::white_balance_gains_from_stops(temperature_ev, tint_ev)
        else {
            return false;
        };
        self.white_balance_gains = gains;
        if let Some(bloom) = self.bloom.as_mut() {
            bloom.white_balance_gains = gains;
        }
        let output_available = if let Some(output) = self.output_chain.as_mut() {
            output.set_white_balance(&self.queue, gains);
            true
        } else {
            false
        };
        self.hdr_exposure_available() || output_available
    }

    /// HDR exposure compensation.
    pub fn hdr_exposure_compensation(&self) -> f32 {
        self.hdr_exposure_ev
    }

    /// `true` when HDR exposure is available.
    pub fn hdr_exposure_available(&self) -> bool {
        hdr_composite_available(
            self.bloom.is_some(),
            self.ambient_pipeline_hdr.is_some(),
            self.projector_pipeline_hdr.is_some(),
            portal_bloom_enabled(),
        )
    }

    /// Whether the active HDR or SDR scene-to-display path applies manual exposure.
    pub fn exposure_transform_available(&self) -> bool {
        if portal_bloom_enabled() != self.bloom_policy_snapshot {
            return false;
        }
        self.hdr_exposure_available() || self.output_chain.is_some()
    }

    /// Whether the current WebGPU scene path retains values above SDR white before output mapping.
    pub fn hdr_scene_available(&self) -> bool {
        if portal_bloom_enabled() != self.bloom_policy_snapshot {
            return false;
        }
        let hdr_pipelines = self.ambient_pipeline_hdr.is_some()
            && self.projector_pipeline_hdr.is_some()
            && self.sky_pipeline_hdr.is_some()
            && self.mesh_pipeline_hdr.is_some();
        hdr_pipelines
            && (self.hdr_exposure_available()
                || self
                    .output_chain
                    .as_ref()
                    .is_some_and(|o| o.uses_hdr_scene()))
    }
}
