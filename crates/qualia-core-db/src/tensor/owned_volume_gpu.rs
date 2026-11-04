//! Request-owned GPU source buffers, uploaded from the retained semantic bytes.
use super::*;
use crate::tensor::resident_substrate::ResidentSnapshot;
pub(crate) const RETAINED_BYTES: usize =
    MAX_RESIDENT_NODES * 40 + 2 * MAX_KNN_HITS * 4 + 40 + 16 + 8;
pub(crate) struct OwnedTensorSearch {
    gpu: &'static crate::gpu_context::SharedGpuContext,
    volume: TensorVolumeGpu,
    count: u32,
}
impl OwnedTensorSearch {
    pub(crate) fn prepare(
        source: &ResidentSnapshot,
        budget: usize,
    ) -> Result<Option<Self>, &'static str> {
        let Some(gpu) = crate::gpu_context::try_shared_gpu() else {
            return Ok(None);
        };
        if RETAINED_BYTES + MAX_RESIDENT_NODES * 40 > budget {
            return Err("owned GPU context budget exceeded");
        }
        let Some(volume) = TensorVolumeGpu::try_new(&gpu.device) else {
            return Ok(None);
        };
        let count = source.node_count();
        let mut flat = [0f32; MAX_RESIDENT_NODES * 10];
        for i in 0..count as usize {
            if let Some(t) = source.tensor_at(i as u32) {
                flat[i * 10..i * 10 + 10]
                    .copy_from_slice(&[t.q, t.v, t.w, t.x, t.y, t.z, t.t, t.alpha, t.mu, t.sigma]);
            }
        }
        if count > 0 {
            gpu.queue_for_universe(crate::gpu_context::ComputeUniverse::Tensor10D)
                .write_buffer(
                    &volume.nodes_buf,
                    0,
                    bytemuck::cast_slice(&flat[..count as usize * 10]),
                );
        }
        Ok(Some(Self { gpu, volume, count }))
    }
    pub(crate) fn search(&self, query: &Tensor10D, distance: f32, out: &mut [usize]) -> usize {
        self.volume.search_uploaded_into(
            &self.gpu.device,
            self.gpu
                .queue_for_universe(crate::gpu_context::ComputeUniverse::Tensor10D),
            query,
            distance,
            self.count,
            out,
        )
    }
}
