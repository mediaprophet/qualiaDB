//! Output calibration and universe admission adapter.
use super::*;
pub(super) struct OutputChain(
    etalon_render::output_pass::OutputChain<crate::gpu_context::VramReservation<'static>>,
);
impl std::ops::Deref for OutputChain {
    type Target =
        etalon_render::output_pass::OutputChain<crate::gpu_context::VramReservation<'static>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl std::ops::DerefMut for OutputChain {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl OutputChain {
    pub(super) fn uses_hdr_scene(&self) -> bool {
        self.0.uses_hdr_scene()
    }
    pub(super) fn scene_view(&self) -> &wgpu::TextureView {
        self.0.scene_view()
    }

    pub(super) fn try_new(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        scene_format: wgpu::TextureFormat,
        surface_format: wgpu::TextureFormat,
        exposure_ev: f32,
        hdr_scene: bool,
        white_balance_gains: [f32; 3],
    ) -> Option<Self> {
        let scale = crate::render::output::exposure_scale_from_ev(exposure_ev)?;
        etalon_render::output_pass::OutputChain::try_new(
            device,
            width,
            height,
            scene_format,
            surface_format,
            scale,
            hdr_scene,
            white_balance_gains,
            |bytes| {
                global_vram_ledger()
                    .try_reserve_graphics(crate::gpu_context::VramResourceClass::FrameTarget, bytes)
                    .ok()
            },
        )
        .map(Self)
    }
    pub(super) fn set_exposure(&mut self, queue: &wgpu::Queue, ev: f32) -> bool {
        crate::render::output::exposure_scale_from_ev(ev)
            .is_some_and(|scale| self.0.set_exposure(queue, scale))
    }
}
