//! WASM MC8 engine — params concern (split from mc8_wasm.rs; verbatim, no behaviour change).
use super::super::*;

impl QTensorEngine {
    /// Dynamic-offset uniform binding for `buf`.
    pub(crate) fn mc8_dynamic_uniform_binding(buf: &wgpu::Buffer) -> wgpu::BindingResource<'_> {
        etalon_inference::browser_prefill::encoding::dynamic_uniform_binding(buf)
    }

    /// Packs GEMM dispatch params for the MC8 kernel.
    pub(crate) fn mc8_gemm_params(
        info: &GgufTensorInfo,
        raw_len: usize,
        n_in: usize,
        n_out: usize,
        n_batch: u32,
        in_row_stride: u32,
        out_row_stride: u32,
    ) -> GemmGpuParams {
        etalon_inference::browser_prefill::params::mc8_gemm_params(
            info,
            raw_len,
            n_in,
            n_out,
            n_batch,
            in_row_stride,
            out_row_stride,
        )
    }

    /// Uploads attention params; returns the binding offset.
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

    /// Packs elementwise-op dispatch params.
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
        etalon_inference::browser_prefill::params::mc8_elem_params(
            op,
            n,
            batch,
            a_row_stride,
            b_row_stride,
            out_row_stride,
            a_slot,
            b_slot,
            out_slot,
        )
    }

    /// A bounded slice of a buffer for a dispatch.
    pub(crate) fn mc8_buf_slice(
        buf: &wgpu::Buffer,
        byte_off: wgpu::BufferAddress,
        byte_len: wgpu::BufferAddress,
    ) -> wgpu::BindingResource<'_> {
        etalon_inference::browser_prefill::encoding::buf_slice(buf, byte_off, byte_len)
    }

    /// Byte offset of prefill row `t` given `row_stride_floats`.
    pub(crate) fn mc8_prefill_row_off(t: u32, row_stride_floats: usize) -> wgpu::BufferAddress {
        etalon_inference::browser_prefill::params::mc8_prefill_row_off(t, row_stride_floats)
    }

    /// Byte offset of embedding `t` in an `n_embd`-wide matrix.
    pub(crate) fn mc8_emb_off(t: u32, n_embd: usize) -> wgpu::BufferAddress {
        etalon_inference::browser_prefill::params::mc8_emb_off(t, n_embd)
    }

    #[cfg(target_arch = "wasm32")]
    /// MC8 helper: `prefill-row-stride`.
    pub(crate) fn mc8_prefill_row_stride(
        n_embd: usize,
        n_ffn_est: usize,
        gemm_max_out_dim: u32,
    ) -> usize {
        etalon_inference::browser_prefill::params::mc8_prefill_row_stride(
            n_embd,
            n_ffn_est,
            gemm_max_out_dim,
        )
    }
}
