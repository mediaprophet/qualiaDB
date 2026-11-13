//! MoE FFN dispatch: routing, expert GEMV accumulation, and FTW package adoption.

use super::*;
#[cfg(not(target_arch = "wasm32"))]
use crate::inference::moe::ftw_loader::FtwModelPackage;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;

impl QTensorEngine {
    /// Adopt an FTW multi-shard model directory for native MoE execution.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn adopt_ftw_package(&mut self, dir: &std::path::Path) -> Result<GgufLoadReport, String> {
        self.adopt_ftw_package_with_budget(dir, 1024, 128 * 1024 * 1024)
    }

    /// Select the external whole-model state budget and context capacity for
    /// admitted Qwen execution. This is separate from the legacy SSM-only
    /// budget and the 42 MiB Sentinel arena; the default reserves at most 128 MiB.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn adopt_ftw_package_with_budget(
        &mut self,
        dir: &std::path::Path,
        max_context: usize,
        model_state_byte_budget: usize,
    ) -> Result<GgufLoadReport, String> {
        let metadata_bytes = std::fs::metadata(dir.join("freetoken_weight.json"))
            .map_err(|e| format!("FTW manifest metadata: {e}"))?
            .len();
        if max_context == 0 || metadata_bytes > model_state_byte_budget as u64 {
            return Err("FTW model state/metadata budget exhausted".into());
        }
        let pkg = Arc::new(FtwModelPackage::open_from_dir(dir)?);
        let index = pkg.tensor_index.clone();
        // A Qwen artifact needs an architecture-owned plan, not permission to
        // run through the generic Llama forward. Automatic admission is bound
        // to a reviewed exporter receipt; other FTW formats retain their guard.
        let qwen35 = if index.hyperparams.architecture == etalon_model::inference::ARCH_QWEN35_MOE {
            let plan = qwen35::ftw::admission::admit_known(
                Arc::clone(&pkg),
                dir,
                max_context,
                model_state_byte_budget,
            )?;
            let remaining = model_state_byte_budget
                .checked_sub(plan.reserved_bytes())
                .ok_or("Qwen plan exhausts caller byte budget")?;
            Some(
                qwen35::ftw::session::Session::new(plan, remaining)
                    .map_err(|e| format!("Qwen session admission: {e:?}"))?,
            )
        } else {
            index.hyperparams.decode_supported()?;
            self.init_ssm_state_with_budget(&index, self.ssm_state_byte_budget)
                .map_err(|e| e.to_string())?;
            None
        };
        self.qwen35_gguf = None;
        self.qwen35_gguf_required = false;
        self.gemma4 = None;
        self.gemma4_required = false;
        self.qwen35_ftw = qwen35;
        self.tensor_index_cache = Some(index.clone());
        self.hyperparams = index.hyperparams;
        self.tensor_data_offset = index.tensor_data_start;
        self.max_tensor_bytes = index.max_layer_tensor_bytes;
        #[cfg(feature = "gpu-runtime")]
        self.invalidate_resident_projection_weights();
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.prepared_native_bytes = None;
            self.prepared_native_sources = None;
        }
        self.gguf_mmap = pkg.shards.first().cloned();
        self.ftw_package = Some(pkg);
        if self.qwen35_ftw.is_some() {
            // The architecture owner includes both recurrent and full-attention
            // storage in its checked budget, outside the Sentinel arena.
            self.ssm_recurrent_state = Box::new([]);
            self.kv_layout = None;
            self.kv_cache_cpu = None;
        } else {
            self.ensure_kv_cache(&index.hyperparams);
        }

        Ok(GgufLoadReport {
            mapped_bytes: self
                .ftw_package
                .as_ref()
                .map(|p| p.manifest.total_bytes)
                .unwrap_or(0),
            tensor_data_offset: self.tensor_data_offset,
            n_layer: self.hyperparams.n_layer,
            n_head: self.hyperparams.n_head,
            n_kv_head: self.hyperparams.n_kv_head,
            max_tensor_bytes: self.max_tensor_bytes,
            kv_cache_bytes: self.kv_cache_bytes(),
            directml_enabled: false,
        })
    }

    /// Read raw bytes for a tensor from either the FTW package shards or the primary GGUF mmap.
    pub fn fetch_tensor_raw_bytes<'a>(
        &'a self,
        info: &crate::gguf_sharder::GgufTensorInfo,
    ) -> Option<&'a [u8]> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let len = crate::ggml_quants::tensor_byte_len(info)?;
            if let Some(ref pkg) = self.ftw_package {
                if let Some(b) = pkg.fetch_by_global_offset(info.byte_offset, len) {
                    return Some(b);
                }
            }
        }
        let mmap = self.model_bytes()?;
        crate::ggml_quants::fetch_tensor_bytes(mmap, self.tensor_data_offset, info).ok()
    }

    /// Dequantize norm weights from FTW shards or GGUF mmap into caller slice.
    pub(crate) fn dequant_norm_tensor_into(
        &self,
        info: &crate::gguf_sharder::GgufTensorInfo,
        out: &mut [f32],
    ) -> usize {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(raw) = self.fetch_tensor_raw_bytes(info) {
            return cpu_ops::dequant_norm_from_raw(raw, info, out);
        }
        if let Some(mmap) = self.model_bytes() {
            return cpu_ops::dequant_norm_row_into(mmap, self.tensor_data_offset, info, out);
        }
        0
    }

    pub(crate) fn dispatch_moe_ffn(
        &mut self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        emb_dim: usize,
        tensors: &crate::gguf_sharder::LayerTensors,
        scratch_a: &mut [f32],
        ffn_input: &[f32],
    ) -> bool {
        etalon_inference::ordinary::moe::execute_moe(
            self, index, emb_dim, tensors, scratch_a, ffn_input,
        )
    }

    /// Dispatches MoE layer using the zero-heap `dispatch_moe_step` driver with expert fetcher (EOS-070).
    pub fn dispatch_moe_step_routed<F>(
        &mut self,
        emb_dim: usize,
        router_weights: &[f32],
        num_experts: usize,
        topk: usize,
        expert_fetcher: F,
        shared_expert: Option<&crate::inference::moe::dispatch::ExpertWeightView<'_>>,
        shared_gate_weight: f32,
        ffn_input: &[f32],
        scratch_a: &mut [f32],
    ) -> bool
    where
        F: FnMut(u16) -> Option<crate::inference::moe::dispatch::ExpertWeightView<'static>>,
    {
        crate::inference::moe::dispatch::dispatch_moe_step(
            &ffn_input[..emb_dim],
            emb_dim,
            router_weights,
            num_experts,
            topk,
            expert_fetcher,
            shared_expert,
            shared_gate_weight,
            &mut scratch_a[..emb_dim],
        )
        .is_ok()
    }

    /// Dispatches MoE layer using the clustered shared-down MoE operator (EOS-071).
    pub fn dispatch_clustered_moe_operator(
        &mut self,
        op: &crate::inference::operator_runtime::ClusteredMoEOperator,
        routed_experts: &[crate::inference::operator_runtime::RoutedExpert],
        ffn_input: &[f32],
        output: &mut [f32],
        scratch: &mut crate::inference::operator_runtime::ClusteredMoeScratch<'_>,
    ) -> bool {
        op.dispatch(routed_experts, ffn_input, output, scratch)
            .is_ok()
    }
}
