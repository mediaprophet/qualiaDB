//! Quantized token embedding + fused transformer block dispatch
//! Split from gguf_bridge/mod.rs (structural refactor; no behaviour change).
use super::*;

impl QTensorEngine {
    /// Upload raw quantized embedding bytes to the GPU and matmul without CPU dequant.
    /// Returns `None` when the GGML type has no WGSL kernel (caller uses CPU fallback).
    #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
    pub fn dispatch_quantized_token_embedding(
        &self,
        raw_embd: &[u8],
        ggml_type: u32,
        n_embd: u32,
        weight_tensor: &QTensor,
    ) -> Option<Vec<f32>> {
        let descriptor = etalon_inference::projections::ProjectionTensor {
            shape: &weight_tensor.shape,
            byte_offset: weight_tensor.byte_offset,
            is_quantized_q4_k: weight_tensor.is_quantized_q4_k,
        };
        etalon_inference::projections::ProjectionGpuEngine::new(self)
            .dispatch_quantized_token_embedding(raw_embd, ggml_type, n_embd, &descriptor)
    }

    #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
    /// Async dispatch: fused-transformer-block.
    pub fn dispatch_fused_transformer_block(
        &self,
        tensor: &QTensor,
        input_activations: &[f32],
    ) -> Vec<f32> {
        let descriptor = etalon_inference::projections::ProjectionTensor {
            shape: &tensor.shape,
            byte_offset: tensor.byte_offset,
            is_quantized_q4_k: tensor.is_quantized_q4_k,
        };
        etalon_inference::projections::ProjectionGpuEngine::new(self)
            .dispatch_fused_transformer_block(&descriptor, input_activations)
    }

    /// Browser inference must await WebGPU buffer mapping. The synchronous entry
    /// point therefore reports the documented CPU-fallback sentinel.
    #[cfg(target_arch = "wasm32")]
    pub fn dispatch_quantized_token_embedding(
        &self,
        raw_embd: &[u8],
        ggml_type: u32,
        n_embd: u32,
        weight_tensor: &QTensor,
    ) -> Option<Vec<f32>> {
        wlog(&format!(
            "[embedding] synchronous browser dispatch unavailable (bytes={}, type={}, dim={}, tensor_offset={})",
            raw_embd.len(),
            ggml_type,
            n_embd,
            weight_tensor.byte_offset
        ));
        None
    }

    /// A synchronous GPU readback cannot make progress on the browser event
    /// loop. Callers must use the async inference surface instead.
    #[cfg(target_arch = "wasm32")]
    pub fn dispatch_fused_transformer_block(
        &self,
        tensor: &QTensor,
        input_activations: &[f32],
    ) -> Vec<f32> {
        panic!(
            "synchronous browser transformer dispatch is unsupported for tensor at byte offset {} ({} activations); use inferWasmAsync/inferWasmStreaming",
            tensor.byte_offset,
            input_activations.len()
        );
    }
}
