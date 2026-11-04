//! WASM MC8 engine — encode concern (split from mc8_wasm.rs; verbatim, no behaviour change).
use super::super::*;

impl QTensorEngine {
    /// Encodes an elementwise dispatch with offset buffers.
    pub(crate) fn encode_elem_offset(
        &self,
        pipeline: &mut WasmGpuPipeline,
        op: u32,
        n: u32,
        batch: u32,
        a: &wgpu::Buffer,
        a_off: wgpu::BufferAddress,
        a_bytes: wgpu::BufferAddress,
        b: &wgpu::Buffer,
        b_off: wgpu::BufferAddress,
        b_bytes: wgpu::BufferAddress,
        out: &wgpu::Buffer,
        out_off: wgpu::BufferAddress,
        out_bytes: wgpu::BufferAddress,
        a_row_stride: u32,
        b_row_stride: u32,
        out_row_stride: u32,
        a_slot: u32,
        b_slot: u32,
        out_slot: u32,
        elem_dyn_offset: u32,
    ) {
        etalon_inference::browser_prefill::PrefillEngine { host: self }.encode_elem_offset(
            pipeline,
            op,
            n,
            batch,
            a,
            a_off,
            a_bytes,
            b,
            b_off,
            b_bytes,
            out,
            out_off,
            out_bytes,
            a_row_stride,
            b_row_stride,
            out_row_stride,
            a_slot,
            b_slot,
            out_slot,
            elem_dyn_offset,
        )
    }

    /// Encodes a GEMM dispatch with offset buffers.
    pub(crate) fn encode_gemm_bufs_offset(
        &self,
        pipeline: &mut WasmGpuPipeline,
        info: &GgufTensorInfo,
        raw: &[u8],
        n_in: usize,
        n_out: usize,
        input: &wgpu::Buffer,
        in_off: wgpu::BufferAddress,
        in_bytes: wgpu::BufferAddress,
        output: &wgpu::Buffer,
        out_off: wgpu::BufferAddress,
        out_bytes: wgpu::BufferAddress,
        n_batch: u32,
        in_row_stride: u32,
        out_row_stride: u32,
        gemm_dyn_offset: u32,
        layer: u32,
        weight_role: Mc8WeightRole,
    ) -> bool {
        etalon_inference::browser_prefill::PrefillEngine { host: self }.encode_gemm_bufs_offset(
            pipeline,
            info,
            raw,
            n_in,
            n_out,
            input,
            in_off,
            in_bytes,
            output,
            out_off,
            out_bytes,
            n_batch,
            in_row_stride,
            out_row_stride,
            gemm_dyn_offset,
            layer,
            weight_role,
        )
    }

    /// Phase 5 dispatch fusion: `silu(gate·x) * (up·x)` in a single compute pass.
    /// Binds both resident gate+up weight sub-ranges for `layer` and reuses the gate
    /// GEMM's staged `GemmParams` (`gemm_dyn_offset`) — valid because the caller has
    /// verified gate and up share ggml_type + dims. Replaces the gate GEMM + up GEMM +
    /// SiLU×mul elementwise (3 dispatches → 1) and skips two n_ffn VRAM round-trips.
    /// Requires resident weights (`mc8_weights_resident`); caller falls back otherwise.
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn encode_fused_ffn_expansion(
        &self,
        pipeline: &mut WasmGpuPipeline,
        layer: u32,
        input: &wgpu::Buffer,
        in_off: wgpu::BufferAddress,
        in_bytes: wgpu::BufferAddress,
        output: &wgpu::Buffer,
        out_off: wgpu::BufferAddress,
        out_bytes: wgpu::BufferAddress,
        n_out: u32,
        n_batch: u32,
        gemm_dyn_offset: u32,
    ) -> bool {
        etalon_inference::browser_prefill::PrefillEngine { host: self }.encode_fused_ffn_expansion(
            pipeline,
            layer,
            input,
            in_off,
            in_bytes,
            output,
            out_off,
            out_bytes,
            n_out,
            n_batch,
            gemm_dyn_offset,
        )
    }

    #[cfg(feature = "wasm-llm-diagnostics")]
    #[cfg(feature = "wasm-llm-diagnostics")]
    /// Encodes `residual-add-offset`.
    pub(crate) fn encode_residual_add_offset(
        &self,
        pipeline: &mut WasmGpuPipeline,
        base: &wgpu::Buffer,
        base_off: wgpu::BufferAddress,
        delta: &wgpu::Buffer,
        delta_off: wgpu::BufferAddress,
        dst: &wgpu::Buffer,
        dst_off: wgpu::BufferAddress,
        scratch: &wgpu::Buffer,
        scratch_off: wgpu::BufferAddress,
        dim: u32,
    ) {
        etalon_inference::browser_prefill::PrefillEngine { host: self }.encode_residual_add_offset(
            pipeline,
            base,
            base_off,
            delta,
            delta_off,
            dst,
            dst_off,
            scratch,
            scratch_off,
            dim,
        )
    }

    #[cfg(feature = "wasm-llm-diagnostics")]
    #[cfg(feature = "wasm-llm-diagnostics")]
    /// Encodes `attention-pass-gpu-offset`.
    pub(crate) fn encode_attention_pass_gpu_offset(
        &self,
        pipeline: &mut WasmGpuPipeline,
        input: &wgpu::Buffer,
        in_off: wgpu::BufferAddress,
        in_bytes: wgpu::BufferAddress,
        output: &wgpu::Buffer,
        out_off: wgpu::BufferAddress,
        out_bytes: wgpu::BufferAddress,
        n_embd: usize,
        num_tokens_in_batch: u32,
        batch_start_token_idx: u32,
        layout: &KvCacheLayout,
        layer: u32,
        token_idx: u32,
        h: &crate::gguf_sharder::GgufHyperparams,
        info: &GgufTensorInfo,
        raw_weights: &[u8],
        proj_kind: u32,
        n_workgroups: u32,
    ) -> bool {
        etalon_inference::browser_prefill::PrefillEngine { host: self }
            .encode_attention_pass_gpu_offset(
                pipeline,
                input,
                in_off,
                in_bytes,
                output,
                out_off,
                out_bytes,
                n_embd,
                num_tokens_in_batch,
                batch_start_token_idx,
                layout,
                layer,
                token_idx,
                &(*h).into(),
                info,
                raw_weights,
                proj_kind,
                n_workgroups,
            )
    }

    /// Encodes an elementwise dispatch.
    pub(crate) fn encode_elem(
        &self,
        pipeline: &mut WasmGpuPipeline,
        op: u32,
        n: u32,
        batch: u32,
        a: &wgpu::Buffer,
        b: &wgpu::Buffer,
        out: &wgpu::Buffer,
    ) {
        etalon_inference::browser_prefill::PrefillEngine { host: self }
            .encode_elem(pipeline, op, n, batch, a, b, out)
    }

    /// Residual add with disjoint storage bindings (WebGPU forbids aliasing read + read_write).
    /// `scratch` must not alias `base` or `delta` (MC8 pt3b: never reuse `gemm_ffn_buf` here —
    /// it holds SwiGLU up / Q proj and aliases `base_save` fallback).
    pub(crate) fn encode_residual_add_gpu(
        &self,
        pipeline: &mut WasmGpuPipeline,
        base: &wgpu::Buffer,
        delta: &wgpu::Buffer,
        dst: &wgpu::Buffer,
        scratch: &wgpu::Buffer,
        dim: u32,
    ) {
        etalon_inference::browser_prefill::PrefillEngine { host: self }
            .encode_residual_add_gpu(pipeline, base, delta, dst, scratch, dim)
    }

    /// Encodes a GEMM dispatch.
    pub(crate) fn encode_gemm_bufs(
        &self,
        pipeline: &mut WasmGpuPipeline,
        info: &GgufTensorInfo,
        raw: &[u8],
        n_in: usize,
        n_out: usize,
        input: &wgpu::Buffer,
        output: &wgpu::Buffer,
    ) -> bool {
        etalon_inference::browser_prefill::PrefillEngine { host: self }
            .encode_gemm_bufs(pipeline, info, raw, n_in, n_out, input, output)
    }

    /// Encodes the GPU attention pass.
    pub(crate) fn encode_attention_pass_gpu(
        &self,
        pipeline: &mut WasmGpuPipeline,
        input: &wgpu::Buffer,
        output: &wgpu::Buffer,
        n_embd: usize,
        num_tokens_in_batch: u32,
        batch_start_token_idx: u32,
        layout: &KvCacheLayout,
        layer: u32,
        token_idx: u32,
        h: &crate::gguf_sharder::GgufHyperparams,
        info: &GgufTensorInfo,
        raw_weights: &[u8],
        proj_kind: u32,
        n_workgroups: u32,
        attn_dyn_offset: u32,
        weight_role: Mc8WeightRole,
    ) -> bool {
        etalon_inference::browser_prefill::PrefillEngine { host: self }.encode_attention_pass_gpu(
            pipeline,
            input,
            output,
            n_embd,
            num_tokens_in_batch,
            batch_start_token_idx,
            layout,
            layer,
            token_idx,
            &(*h).into(),
            info,
            raw_weights,
            proj_kind,
            n_workgroups,
            attn_dyn_offset,
            weight_role,
        )
    }

    /// MC8 Part 3o/3t: batched Q-SDPA — caller pre-uploads `attn_dyn_offset` in shared attn arena.
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn encode_attention_batched_q_prefill(
        &self,
        pipeline: &mut WasmGpuPipeline,
        input: &wgpu::Buffer,
        in_off: wgpu::BufferAddress,
        in_bytes: wgpu::BufferAddress,
        output: &wgpu::Buffer,
        out_off: wgpu::BufferAddress,
        out_bytes: wgpu::BufferAddress,
        n_embd: usize,
        n_tokens: u32,
        layout: &KvCacheLayout,
        layer: u32,
        h: &crate::gguf_sharder::GgufHyperparams,
        raw_weights: &[u8],
        attn_dyn_offset: u32,
    ) -> bool {
        etalon_inference::browser_prefill::PrefillEngine { host: self }
            .encode_attention_batched_q_prefill(
                pipeline,
                input,
                in_off,
                in_bytes,
                output,
                out_off,
                out_bytes,
                n_embd,
                n_tokens,
                layout,
                layer,
                &(*h).into(),
                raw_weights,
                attn_dyn_offset,
            )
    }
}
