//! WASM MC8 engine — readback concern (split from mc8_wasm.rs; verbatim, no behaviour change).
use super::super::*;

impl QTensorEngine {
    pub(crate) async fn pipeline_read_hidden(&self, emb_dim: usize, hidden: &mut [f32]) -> bool {
        let staging = self.gemm_output_staging.as_ref().unwrap();
        let hidden_buf = self.gemm_input_buf.as_ref().unwrap();
        let out_bytes = (emb_dim * 4) as wgpu::BufferAddress;
        let mut encoder = self
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("MC8Readback"),
            });
        encoder.copy_buffer_to_buffer(hidden_buf, 0, staging, 0, out_bytes);
        self.gpu_queue().submit(Some(encoder.finish()));
        let slice = staging.slice(..out_bytes);
        if !await_wgpu_map(slice).await {
            let _ = staging.unmap();
            return false;
        }
        let data = slice.get_mapped_range();
        let floats: &[f32] = bytemuck::cast_slice(&data);
        hidden[..emb_dim].copy_from_slice(&floats[..emb_dim]);
        drop(data);
        staging.unmap();
        true
    }

    pub(crate) async fn pipeline_read_batch(&self, batch_elems: usize, out: &mut [f32]) -> bool {
        if batch_elems > out.len() || batch_elems > self.gemm_max_input_floats {
            return false;
        }
        let staging = self.gemm_output_staging.as_ref().unwrap();
        let batch_buf = self.gemm_output_buf.as_ref().unwrap();
        let out_bytes = (batch_elems * 4) as wgpu::BufferAddress;
        if out_bytes > staging.size() {
            return false;
        }
        let mut encoder = self
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("MC8BatchReadback"),
            });
        encoder.copy_buffer_to_buffer(batch_buf, 0, staging, 0, out_bytes);
        self.gpu_queue().submit(Some(encoder.finish()));
        let slice = staging.slice(..out_bytes);
        if !await_wgpu_map(slice).await {
            let _ = staging.unmap();
            return false;
        }
        let data = slice.get_mapped_range();
        let floats: &[f32] = bytemuck::cast_slice(&data);
        out[..batch_elems].copy_from_slice(&floats[..batch_elems]);
        drop(data);
        staging.unmap();
        true
    }

    pub(crate) async fn pipeline_read_gpu_bytes_at(
        &self,
        src: &wgpu::Buffer,
        byte_offset: wgpu::BufferAddress,
        out: &mut [u8],
    ) -> bool {
        if out.is_empty() {
            return false;
        }
        let staging = self.gemm_output_staging.as_ref().unwrap();
        let nbytes = out.len() as wgpu::BufferAddress;
        if nbytes > staging.size() {
            return false;
        }
        let mut encoder = self
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("MC8ProbeReadback"),
            });
        encoder.copy_buffer_to_buffer(src, byte_offset, staging, 0, nbytes);
        self.gpu_queue().submit(Some(encoder.finish()));
        let slice = staging.slice(..nbytes);
        if !await_wgpu_map(slice).await {
            let _ = staging.unmap();
            return false;
        }
        let data = slice.get_mapped_range();
        out.copy_from_slice(&data);
        drop(data);
        staging.unmap();
        true
    }

    pub(crate) async fn pipeline_read_kv_head(
        &self,
        layout: &KvCacheLayout,
        layer: u32,
        slot: u32,
        kv_h: u32,
        head_dim: usize,
        k_not_v: bool,
        out: &mut [f32],
    ) -> bool {
        if head_dim == 0 || head_dim > out.len() {
            return false;
        }
        let kv = match self.kv_cache_gpu.as_ref() {
            Some(b) => b,
            None => return false,
        };
        let idx = if k_not_v {
            layout.k_index(layer, slot, kv_h, 0)
        } else {
            layout.v_index(layer, slot, kv_h, 0)
        };
        let byte_off = (idx * std::mem::size_of::<f32>()) as wgpu::BufferAddress;
        let mut bytes = [0u8; 512];
        let nbytes = head_dim * std::mem::size_of::<f32>();
        if nbytes > bytes.len() {
            return false;
        }
        if !self
            .pipeline_read_gpu_bytes_at(kv, byte_off, &mut bytes[..nbytes])
            .await
        {
            return false;
        }
        let floats: &[f32] = bytemuck::cast_slice(&bytes[..nbytes]);
        out[..head_dim].copy_from_slice(floats);
        true
    }
}
