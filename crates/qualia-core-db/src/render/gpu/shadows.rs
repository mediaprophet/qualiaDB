//! Universe-admitted cascaded shadow resources.
use crate::gpu_context::{global_vram_ledger, VramReservation, VramResourceClass};
pub use etalon_render::shadows::Csm4ShadowUniform;
pub(super) use etalon_render::shadows::{
    compare_sampler, matrix_layout, ShadowUniform, SHADOW_CASCADE_COUNT,
};
pub(super) struct ShadowTarget(etalon_render::shadows::ShadowTarget<VramReservation<'static>>);
impl std::ops::Deref for ShadowTarget {
    type Target = etalon_render::shadows::ShadowTarget<VramReservation<'static>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl ShadowTarget {
    pub fn try_new(device: &wgpu::Device) -> Option<Self> {
        etalon_render::shadows::ShadowTarget::try_new(device, |bytes| {
            global_vram_ledger()
                .try_reserve_graphics(VramResourceClass::FrameTarget, bytes)
                .ok()
        })
        .map(Self)
    }
}
#[path = "shadow_math.rs"]
mod shadow_math;
pub(super) use shadow_math::{aabb_outside_shadow_clip, camera_range_sun_view_projections};
pub use shadow_math::{
    camera_csm_4_sun_view_projections, compute_csm_4_splits_zero_heap, csm_cascade_blend_factor,
    ShadowAtlasAllocator, SpotPointShadowAtlasSlot, CSM_4_CASCADE_COUNT,
};
