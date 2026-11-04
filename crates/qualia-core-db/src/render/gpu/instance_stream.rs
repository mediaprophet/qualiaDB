//! Universe admission for canonical bounded instance uploads.
use crate::gpu_context::{global_vram_ledger, VramReservation, VramResourceClass};
pub use etalon_render::instance_stream::MeshInstanceUploadError;
pub(super) use etalon_render::instance_stream::MAX_MESH_INSTANCES;
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
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
    ) -> Result<Self, MeshInstanceUploadError> {
        etalon_render::instance_stream::GpuInstanceStream::new(device, layout, reserve).map(Self)
    }
    pub(super) fn replace_packed(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        records: &[u8],
    ) -> Result<(), MeshInstanceUploadError> {
        self.0
            .replace_packed(device, queue, layout, records, reserve)
    }
    pub(super) fn replace_culled_packed(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        records: &[u8],
    ) -> Result<(), MeshInstanceUploadError> {
        self.0
            .replace_culled_packed(device, queue, layout, records, reserve)
    }
}
