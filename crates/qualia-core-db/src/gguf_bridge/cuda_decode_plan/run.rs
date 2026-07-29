use super::types::{CudaDecodePlan, CudaDecodePlanState};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CudaPreparedTelemetry {
    /// Device kernel nodes executed by one decode graph.
    pub device_dispatches_per_token: u64,
    /// Dynamic activation and step-state bytes copied H2D per token.
    pub host_to_device_bytes_per_token: u64,
    /// Final argmax token bytes copied D2H per token.
    pub readback_bytes_per_token: u64,
    /// Stable instantiated graph key. `None` means the graph was not captured.
    pub graph_key: Option<u64>,
    /// Human-readable cold tuning record selected for this model shape.
    pub tuning_profile: &'static str,
    /// Prepared CUDA KV capacity.
    pub context_window: u32,
}

impl CudaDecodePlan {
    fn run(&self, hidden: &mut [f32], token_idx: u32) -> Option<u32> {
        self.run_inner(hidden, token_idx, true, self.n_layer, None)
    }

    fn run_token(&self, token_id: u32, hidden: &mut [f32], token_idx: u32) -> Option<u32> {
        if token_id as usize >= self.token_embedding_vocab {
            return None;
        }
        let embedding_key = self.token_embedding_key?;
        self.run_inner(
            hidden,
            token_idx,
            true,
            self.n_layer,
            Some((token_id, embedding_key)),
        )
    }

    fn run_inner(
        &self,
        hidden: &mut [f32],
        token_idx: u32,
        project_logits: bool,
        layer_count: u32,
        input_token: Option<(u32, u64)>,
    ) -> Option<u32> {
        let lm_head = project_logits
            .then(|| {
                self.lm_head
                    .as_ref()
                    .and_then(|range| self.mmap.get(range.clone()))
            })
            .flatten();
        crate::inference::cuda_lane::try_cuda_mega_pass_with_token(
            self.n_embd,
            self.n_head,
            self.n_kv,
            self.head_dim,
            layer_count,
            token_idx,
            self.max_context,
            self.layer_stride,
            self.slot_kv_elems,
            self.rope_base,
            self.rope_scale,
            self.rms_eps,
            hidden,
            self,
            project_logits.then_some(self.output_norm_key).flatten(),
            lm_head,
            project_logits.then_some(self.lm_head_key).flatten(),
            self.lm_head_in,
            self.lm_head_out,
            input_token,
        )
    }
}

impl crate::gguf_bridge::QTensorEngine {
    pub fn cuda_prepared_telemetry(&self) -> Option<CudaPreparedTelemetry> {
        let CudaDecodePlanState::Ready(plan) = &self.cuda_decode_plan else {
            return None;
        };
        Some(CudaPreparedTelemetry {
            device_dispatches_per_token: crate::inference::cuda_lane::decode_graph_node_count()?,
            host_to_device_bytes_per_token:
                crate::inference::cuda_lane::decode_graph_h2d_bytes_per_token()?,
            readback_bytes_per_token: 4,
            graph_key: crate::inference::cuda_lane::decode_graph_key(),
            tuning_profile: crate::inference::cuda_lane::cuda_q8_tuning_for_model(
                plan.n_embd,
                plan.n_head,
                plan.n_kv,
                plan.head_dim,
                plan.n_layer,
                plan.lm_head_out,
            )
            .receipt_id(),
            context_window: plan.max_context,
        })
    }

    pub(in crate::gguf_bridge) fn try_prepared_cuda_decode(
        &mut self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        hidden: &mut [f32],
        emb_dim: usize,
        token_idx: u32,
    ) -> Option<u32> {
        let mmap = self.gguf_mmap.as_deref()?;
        let key = (
            mmap.as_ptr() as u64,
            index.tensor_data_start,
            index.hyperparams.n_layer,
        );
        let state = std::mem::replace(&mut self.cuda_decode_plan, CudaDecodePlanState::Unbuilt);
        let plan = match state {
            CudaDecodePlanState::Ready(plan) if plan.key == key => plan,
            CudaDecodePlanState::Ineligible(fingerprint)
                if fingerprint == key.0 ^ key.1 ^ key.2 as u64 =>
            {
                self.cuda_decode_plan = CudaDecodePlanState::Ineligible(fingerprint);
                return None;
            }
            _ => match CudaDecodePlan::build(self, index, emb_dim) {
                Some(plan) => plan,
                None => {
                    self.cuda_decode_plan =
                        CudaDecodePlanState::Ineligible(key.0 ^ key.1 ^ key.2 as u64);
                    return None;
                }
            },
        };
        let result = plan.run(hidden, token_idx);
        self.cuda_decode_plan = CudaDecodePlanState::Ready(plan);
        result
    }

    pub(in crate::gguf_bridge) fn try_prepared_cuda_decode_token(
        &mut self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        token_id: u32,
        hidden: &mut [f32],
        emb_dim: usize,
        token_idx: u32,
    ) -> Option<u32> {
        let mmap = self.gguf_mmap.as_deref()?;
        let key = (
            mmap.as_ptr() as u64,
            index.tensor_data_start,
            index.hyperparams.n_layer,
        );
        let state = std::mem::replace(&mut self.cuda_decode_plan, CudaDecodePlanState::Unbuilt);
        let plan = match state {
            CudaDecodePlanState::Ready(plan) if plan.key == key => plan,
            CudaDecodePlanState::Ineligible(fingerprint)
                if fingerprint == key.0 ^ key.1 ^ key.2 as u64 =>
            {
                self.cuda_decode_plan = CudaDecodePlanState::Ineligible(fingerprint);
                return None;
            }
            _ => match CudaDecodePlan::build(self, index, emb_dim) {
                Some(plan) => plan,
                None => {
                    self.cuda_decode_plan =
                        CudaDecodePlanState::Ineligible(key.0 ^ key.1 ^ key.2 as u64);
                    return None;
                }
            },
        };
        let result = plan.run_token(token_id, hidden, token_idx);
        self.cuda_decode_plan = CudaDecodePlanState::Ready(plan);
        result
    }

    #[cfg(test)]
    pub(in crate::gguf_bridge) fn try_prepared_cuda_hidden_for_test(
        &mut self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        hidden: &mut [f32],
        emb_dim: usize,
        token_idx: u32,
        layer_count: u32,
    ) -> bool {
        let mmap = match self.gguf_mmap.as_deref() {
            Some(mmap) => mmap,
            None => return false,
        };
        let key = (
            mmap.as_ptr() as u64,
            index.tensor_data_start,
            index.hyperparams.n_layer,
        );
        let state = std::mem::replace(&mut self.cuda_decode_plan, CudaDecodePlanState::Unbuilt);
        let plan = match state {
            CudaDecodePlanState::Ready(plan) if plan.key == key => plan,
            _ => match CudaDecodePlan::build(self, index, emb_dim) {
                Some(plan) => plan,
                None => return false,
            },
        };
        let completed =
            plan.run_inner(hidden, token_idx, false, layer_count, None) == Some(u32::MAX);
        self.cuda_decode_plan = CudaDecodePlanState::Ready(plan);
        completed
    }
}
