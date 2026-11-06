//! Universe admission for canonical bounded instance uploads.
use crate::gpu_context::{global_vram_ledger, VramReservation, VramResourceClass};
pub use etalon_render::instance_stream::{
    DirtyInstanceRange, DirtyUploadPolicy, MeshInstanceUploadError,
};
pub(super) struct GpuInstanceStream(
    etalon_render::instance_stream::GpuInstanceStream<VramReservation<'static>>,
);
impl std::ops::Deref for GpuInstanceStream {
    type Target = etalon_render::instance_stream::GpuInstanceStream<VramReservation<'static>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
fn reserve(bytes: u64) -> Option<VramReservation<'static>> {
    global_vram_ledger()
        .try_reserve_graphics(VramResourceClass::Geometry, bytes)
        .ok()
}
impl GpuInstanceStream {
    pub(super) fn new(
        device: &etalon::Device,
        layout: &etalon::BindGroupLayout,
    ) -> Result<Self, MeshInstanceUploadError> {
        etalon_render::instance_stream::GpuInstanceStream::new(device, layout, reserve).map(Self)
    }
    pub(super) fn replace_packed(
        &mut self,
        device: &etalon::Device,
        queue: &etalon::Queue,
        layout: &etalon::BindGroupLayout,
        records: &[u8],
    ) -> Result<(), MeshInstanceUploadError> {
        self.0
            .replace_packed(device, queue, layout, records, reserve)
    }
    pub(super) fn replace_culled_packed(
        &mut self,
        device: &etalon::Device,
        queue: &etalon::Queue,
        layout: &etalon::BindGroupLayout,
        records: &[u8],
    ) -> Result<(), MeshInstanceUploadError> {
        self.0
            .replace_culled_packed(device, queue, layout, records, reserve)
    }
    #[allow(dead_code)]
    pub(super) fn update_dirty_ranges_packed(
        &mut self,
        device: &etalon::Device,
        queue: &etalon::Queue,
        layout: &etalon::BindGroupLayout,
        records: &[u8],
        dirty_ranges: &[DirtyInstanceRange],
        policy: DirtyUploadPolicy,
    ) -> Result<usize, MeshInstanceUploadError> {
        self.0.update_dirty_ranges_packed(
            device,
            queue,
            layout,
            records,
            dirty_ranges,
            policy,
            reserve,
        )
    }
}
