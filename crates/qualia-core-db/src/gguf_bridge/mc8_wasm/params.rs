//! WASM MC8 engine — params concern (split from mc8_wasm.rs; verbatim, no behaviour change).
use super::super::*;

impl QTensorEngine {
    pub(crate) fn mc8_dynamic_uniform_binding(buf: &wgpu::Buffer) -> wgpu::BindingResource<'_> {
        wgpu::BindingResource::Buffer(wgpu::BufferBinding {
            buffer: buf,
            offset: 0,
            size: std::num::NonZeroU64::new(MC8_UNIFORM_ALIGN as u64),
        })
    }

    pub(crate) fn mc8_gemm_params(
        info: &GgufTensorInfo,
        raw_len: usize,
        n_in: usize,
        n_out: usize,
        n_batch: u32,
        in_row_stride: u32,
        out_row_stride: u32,
    ) -> GemmGpuParams {
        GemmGpuParams {
            n_in: n_in as u32,
            n_out: n_out as u32,
            weight_ggml_type: info.ggml_type,
            weight_row_elems: info.dims[0] as u32,
            weight_byte_len: raw_len as u32,
            n_batch: n_batch.max(1),
            in_row_stride,
            out_row_stride,
        }
    }

    pub(crate) fn mc8_upload_attn_param(&self, params: &AttentionGpuParams) -> u32 {
        let mut arena = Mc8UniformArena {
            bytes: [0u8; MC8_MAX_GEMM_UNIFORM_SLOTS * MC8_UNIFORM_ALIGN],
            slots: 0,
        };
        let off = arena.push(params);
        arena.upload(
            self.gpu_queue(),
            self.attention_params_buf.as_ref().expect("attn params buf"),
        );
        off
    }

    pub(crate) fn mc8_elem_params(
        op: u32,
        n: u32,
        batch: u32,
        a_row_stride: u32,
        b_row_stride: u32,
        out_row_stride: u32,
        a_slot: u32,
        b_slot: u32,
        out_slot: u32,
    ) -> ElemGpuParams {
        ElemGpuParams {
            n,
            batch: batch.max(1),
            op,
            eps: RMS_NORM_EPS,
            a_row_stride,
            b_row_stride,
            out_row_stride,
            a_slot,
            b_slot,
            out_slot,
            _pad: 0,
        }
    }

    pub(crate) fn mc8_buf_slice(
        buf: &wgpu::Buffer,
        byte_off: wgpu::BufferAddress,
        byte_len: wgpu::BufferAddress,
    ) -> wgpu::BindingResource<'_> {
        wgpu::BindingResource::Buffer(wgpu::BufferBinding {
            buffer: buf,
            offset: byte_off,
            size: std::num::NonZeroU64::new(byte_len.max(4)),
        })
    }

    pub(crate) fn mc8_prefill_row_off(t: u32, row_stride_floats: usize) -> wgpu::BufferAddress {
        (t as usize * row_stride_floats * 4) as wgpu::BufferAddress
    }

    pub(crate) fn mc8_emb_off(t: u32, n_embd: usize) -> wgpu::BufferAddress {
        (t as usize * n_embd * 4) as wgpu::BufferAddress
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn mc8_prefill_row_stride(
        n_embd: usize,
        n_ffn_est: usize,
        gemm_max_out_dim: u32,
    ) -> usize {
        (n_embd + 2 * n_ffn_est + n_embd)
            .max(gemm_max_out_dim as usize * 2)
            .max(n_embd)
    }
}
