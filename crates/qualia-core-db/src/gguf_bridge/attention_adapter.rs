#![allow(unused_variables, unused_mut)]
use super::*;
use etalon_inference::ordinary::attention::AttentionProvider;
use etalon_model::inference::ModelHyperparams;
impl AttentionProvider for QTensorEngine {
    type Index = crate::gguf_sharder::GgufTensorIndex;
    type Layout = KvCacheLayout;
    fn kv_layout(&self) -> Option<Self::Layout> {
        self.kv_layout
    }
    fn hyperparams(index: &Self::Index) -> ModelHyperparams {
        index.hyperparams.into()
    }
    fn tensor_data_start(index: &Self::Index) -> u64 {
        index.tensor_data_start
    }
    fn model_bytes(&self) -> Option<&[u8]> {
        QTensorEngine::model_bytes(self)
    }
    fn debug_decode(&self) -> bool {
        super::execution_env::var("QUALIA_LLM_DEBUG_DECODE").is_ok()
    }
    fn prefer_cuda(&self) -> bool {
        #[cfg(all(not(target_arch = "wasm32"), feature = "cuda"))]
        {
            return crate::prefer_tensor_core_gemm();
        }
        #[cfg(not(all(not(target_arch = "wasm32"), feature = "cuda")))]
        {
            false
        }
    }
    fn attention_preproject(&self) -> bool {
        cfg!(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))
            && crate::llm_bench::attention_preproject_enabled()
    }
    fn attention_o_fuse(&self) -> bool {
        cfg!(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))
            && crate::llm_bench::attention_o_fuse_enabled()
    }
    fn dispatch_gemm_into(
        &self,
        index: &Self::Index,
        info: &GgufTensorInfo,
        input: &[f32],
        out: &mut [f32],
        n_in: usize,
        n_out: usize,
    ) -> bool {
        QTensorEngine::dispatch_gemm_into(self, index, info, input, out, n_in, n_out)
    }
    fn dispatch_ffn_block_pre_norm(
        &mut self,
        index: &Self::Index,
        hidden: &mut [f32],
        emb_dim: usize,
        tensors: &crate::gguf_sharder::LayerTensors,
        scratch_a: &mut [f32],
        scratch_b: &mut [f32],
    ) -> bool {
        QTensorEngine::dispatch_ffn_block_pre_norm(
            self, index, hidden, emb_dim, tensors, scratch_a, scratch_b,
        )
    }
    fn dispatch_attention_pass(
        &self,
        hidden: &[f32],
        n_embd: usize,
        num_tokens_in_batch: u32,
        batch_start_token_idx: u32,
        layout: &Self::Layout,
        layer: u32,
        token_idx: u32,
        h: &ModelHyperparams,
        info: &GgufTensorInfo,
        raw_weights: &[u8],
        proj_kind: u32,
        n_workgroups: u32,
        norm_weight: Option<&[f32]>,
        readback_out: Option<&mut [f32]>,
    ) -> bool {
        QTensorEngine::dispatch_attention_pass(
            self,
            hidden,
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
            norm_weight,
            readback_out,
        )
    }
    fn try_cuda_soa_attention_layer(
        &self,
        hidden_input: &[f32],
        n_embd: usize,
        q_dim: usize,
        kv_dim: usize,
        n_head: usize,
        n_kv: usize,
        head_dim: usize,
        layout: &Self::Layout,
        layer: u32,
        token_idx: u32,
        h: &ModelHyperparams,
        q_raw: &[u8],
        k_raw: &[u8],
        v_raw: &[u8],
        out_info: Option<&GgufTensorInfo>,
        mmap: &[u8],
        tensor_data_start: u64,
        scratch_a: &mut [f32],
        scratch_b: &mut [f32],
    ) -> bool {
        #[cfg(all(not(target_arch = "wasm32"), feature = "cuda"))]
        {
            return QTensorEngine::try_cuda_soa_attention_layer(
                self,
                hidden_input,
                n_embd,
                q_dim,
                kv_dim,
                n_head,
                n_kv,
                head_dim,
                layout,
                layer,
                token_idx,
                &(*h).into(),
                q_raw,
                k_raw,
                v_raw,
                out_info,
                mmap,
                tensor_data_start,
                scratch_a,
                scratch_b,
            );
        }
        #[cfg(not(all(not(target_arch = "wasm32"), feature = "cuda")))]
        {
            false
        }
    }
    fn dispatch_attention_kv_preproject_fused(
        &self,
        hidden: &[f32],
        n_embd: usize,
        layout: &Self::Layout,
        layer: u32,
        token_idx: u32,
        h: &ModelHyperparams,
        k_info: &GgufTensorInfo,
        k_raw: &[u8],
        v_info: &GgufTensorInfo,
        v_raw: &[u8],
        kv_dim: usize,
    ) -> bool {
        #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
        {
            return QTensorEngine::dispatch_attention_kv_preproject_fused(
                self,
                hidden,
                n_embd,
                layout,
                layer,
                token_idx,
                &(*h).into(),
                k_info,
                k_raw,
                v_info,
                v_raw,
                kv_dim,
            );
        }
        #[cfg(not(all(not(target_arch = "wasm32"), feature = "gpu-runtime")))]
        {
            false
        }
    }
    fn dispatch_attention_q_o_fused(
        &self,
        hidden: &[f32],
        n_embd: usize,
        layout: &Self::Layout,
        layer: u32,
        token_idx: u32,
        h: &ModelHyperparams,
        q_info: &GgufTensorInfo,
        q_raw: &[u8],
        o_info: &GgufTensorInfo,
        o_raw: &[u8],
        out: &mut [f32],
        o_in: usize,
        o_out: usize,
    ) -> bool {
        #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
        {
            return QTensorEngine::dispatch_attention_q_o_fused(
                self,
                hidden,
                n_embd,
                layout,
                layer,
                token_idx,
                &(*h).into(),
                q_info,
                q_raw,
                o_info,
                o_raw,
                out,
                o_in,
                o_out,
            );
        }
        #[cfg(not(all(not(target_arch = "wasm32"), feature = "gpu-runtime")))]
        {
            false
        }
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "cuda"))]
impl etalon_inference::ordinary::cuda_attention::CudaAttentionHost for QTensorEngine {
    fn with_cpu_kv<R>(&self, run: impl FnOnce(Option<&mut [f32]>) -> R) -> R {
        let kv = self.kv_cache_cpu.as_ref().map(|kv| {
            // SAFETY: the caller retains exclusive decode-thread custody.
            unsafe { core::slice::from_raw_parts_mut(kv.as_ptr() as *mut f32, kv.len()) }
        });
        run(kv)
    }
    fn gemm(
        &self,
        raw: &[u8],
        info: &GgufTensorInfo,
        input: &[f32],
        out: &mut [f32],
        n_in: usize,
        n_out: usize,
    ) -> bool {
        stack_gemm_quant(raw, info, input, out, n_in, n_out)
    }
    fn device_attempt(
        &self,
        hit: bool,
        layer: u32,
        token_idx: u32,
        o_in: usize,
        o_out: usize,
        q_dim: usize,
        n_embd: usize,
    ) {
        if hit {
            static LOGGED: std::sync::atomic::AtomicBool =
                std::sync::atomic::AtomicBool::new(false);
            if !LOGGED.swap(true, std::sync::atomic::Ordering::Relaxed) {
                log::info!(
                    "cuda_attn|device_sdpa|first_hit|layer={layer}|tok={token_idx}|kv_ready={}",
                    crate::device_kv_ready()
                );
            }
        } else {
            static MISSES: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
            if MISSES.fetch_add(1, std::sync::atomic::Ordering::Relaxed) < 3 {
                log::warn!("cuda_attn|device_sdpa|miss|layer={layer}|tok={token_idx}|kv_ready={}|o_in={o_in}|o_out={o_out}|q_dim={q_dim}|n_embd={n_embd}", crate::device_kv_ready());
            }
        }
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "wgsl-forge"))]
impl etalon_inference::gpu_attention::forge::ForgeAttentionHost for QTensorEngine {
    fn device(&self) -> &etalon::Device {
        QTensorEngine::device(self)
    }
    fn submit(&self, command: etalon::CommandBuffer) {
        self.submit_commands(Some(command));
    }
    fn poll_wait(&self) {
        QTensorEngine::poll_wait(self);
    }
    fn wait_for_map(
        &self,
        receiver: futures_channel::oneshot::Receiver<Result<(), etalon::BufferAsyncError>>,
    ) -> bool {
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            return handle
                .block_on(receiver)
                .ok()
                .map(|r| r.is_ok())
                .unwrap_or(false);
        }
        false
    }
}
