//! Browser API for manual scene exposure and output-transform capability.

use wasm_bindgen::prelude::wasm_bindgen;

use super::QualiaPortal;

#[wasm_bindgen]
impl QualiaPortal {
    /// Set linear-light temperature/tint biases in stops, each bounded to -1…+1.
    /// Temperature warms red while cooling blue; positive tint favours green.
    /// The value is retained and replayed when a renderer is initialized or recovered.
    pub fn set_white_balance(&mut self, temperature_ev: f32, tint_ev: f32) -> bool {
        if crate::render::output::white_balance_gains_from_stops(temperature_ev, tint_ev).is_none()
        {
            return false;
        }
        self.white_balance_ev = [
            temperature_ev.clamp(
                crate::render::output::MIN_WHITE_BALANCE_EV,
                crate::render::output::MAX_WHITE_BALANCE_EV,
            ),
            tint_ev.clamp(
                crate::render::output::MIN_WHITE_BALANCE_EV,
                crate::render::output::MAX_WHITE_BALANCE_EV,
            ),
        ];
        let webgpu_available = self.gpu.as_mut().is_some_and(|gpu| {
            gpu.set_white_balance(self.white_balance_ev[0], self.white_balance_ev[1])
        });
        let webgl2_available = self.anatomy_webgl2.as_mut().is_some_and(|renderer| {
            renderer.set_white_balance(self.white_balance_ev[0], self.white_balance_ev[1])
        });
        webgpu_available || webgl2_available
    }

    /// Configured temperature/tint bias in stops, including while graphics is unavailable.
    pub fn white_balance(&self) -> Vec<f32> {
        self.white_balance_ev.to_vec()
    }

    /// Configure manual exposure compensation in stops (EV), clamped to -8…+8.
    /// The configured value is retained while the GPU output path is unavailable.
    pub fn set_exposure_compensation(&mut self, ev: f32) -> bool {
        let Some(_) = crate::render::output::exposure_scale_from_ev(ev) else {
            return false;
        };
        self.hdr_exposure_ev = ev.clamp(
            crate::render::output::MIN_HDR_EXPOSURE_EV,
            crate::render::output::MAX_HDR_EXPOSURE_EV,
        );
        let webgpu_available = self
            .gpu
            .as_mut()
            .map(|gpu| gpu.set_exposure_compensation(self.hdr_exposure_ev))
            .unwrap_or(false);
        let webgl2_available = self
            .anatomy_webgl2
            .as_mut()
            .is_some_and(|renderer| renderer.set_exposure_compensation(self.hdr_exposure_ev));
        webgpu_available || webgl2_available
    }

    /// Compatibility alias for hosts using the original HDR-specific API name.
    pub fn set_hdr_exposure_compensation(&mut self, ev: f32) -> bool {
        self.set_exposure_compensation(ev)
    }

    /// True while an active WebGPU or WebGL2 path applies scene exposure and the SDR transform.
    pub fn exposure_transform_available(&self) -> bool {
        self.gpu
            .as_ref()
            .map(crate::render::gpu::PortalGpu::exposure_transform_available)
            .unwrap_or(false)
            || self
                .anatomy_webgl2
                .as_ref()
                .is_some_and(|renderer| renderer.exposure_transform_available())
    }

    /// True when the active WebGPU or WebGL2 scene target preserves radiance above SDR white.
    pub fn hdr_scene_available(&self) -> bool {
        self.gpu
            .as_ref()
            .map(crate::render::gpu::PortalGpu::hdr_scene_available)
            .unwrap_or(false)
            || self
                .anatomy_webgl2
                .as_ref()
                .is_some_and(|renderer| renderer.hdr_scene_available())
    }

    /// True only while the HDR bloom composite path is active.
    pub fn hdr_exposure_available(&self) -> bool {
        self.gpu
            .as_ref()
            .map(crate::render::gpu::PortalGpu::hdr_exposure_available)
            .unwrap_or(false)
    }

    /// Configured compensation in stops (EV), including while GPU rendering is unavailable.
    pub fn exposure_compensation(&self) -> f32 {
        self.hdr_exposure_ev
    }

    /// Compatibility alias for hosts using the original HDR-specific API name.
    pub fn hdr_exposure_compensation(&self) -> f32 {
        self.exposure_compensation()
    }
}
