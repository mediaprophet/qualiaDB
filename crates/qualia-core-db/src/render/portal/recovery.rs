//! Frame-boundary detection for browser device loss.
//!
//! The WebGPU callback only sets an atomic flag. This module observes it on the
//! portal's normal frame thread, drops all invalid device resources, and asks
//! the browser host to replace the context-bound canvas and create a new device.

use super::{BodyRendererBackend, QualiaPortal};

impl QualiaPortal {
    /// Drop invalid WebGPU resources and request host-side surface recreation.
    /// Returns true only on the first frame that observes this device loss.
    pub(crate) fn note_device_loss(&mut self) -> bool {
        let webgpu_lost = self
            .gpu
            .as_ref()
            .map(crate::render::gpu::PortalGpu::is_device_lost)
            .unwrap_or(false);
        let webgl_lost = self
            .anatomy_webgl2
            .as_ref()
            .map(crate::render::anatomy::webgl2::AnatomyWebGl2::is_context_lost)
            .unwrap_or(false);
        if !webgpu_lost && !webgl_lost {
            return false;
        }

        self.gpu = None;
        self.anatomy_webgl2 = None;
        self.gpu_init_failed = true;
        self.graphics_recovery_requested = true;
        self.graphics_recovery_pending = false;
        self.pending_gpu_pick = false;
        self.present_misses = 0;
        self.tier = 0;
        if matches!(
            self.body_renderer,
            BodyRendererBackend::WebGpu | BodyRendererBackend::WebGl2
        ) {
            self.body_renderer = BodyRendererBackend::CpuCanvas;
        }
        true
    }
}
