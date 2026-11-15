//! Engine compatibility entry points for an already admitted Qwen owner.
//! Unknown Qwen artifacts cannot fall through to the generic Llama kernels.
use super::{dense_host::DenseHost, session::Session, weights::DenseSource, Plan};
use crate::gguf_bridge::{QTensorEngine, StreamingArgmaxResult};
use crate::gguf_sharder::{GgufTensorIndex, LayerTensors};
use etalon_inference::ordinary::qwen35::{ordinary_rms_norm, Host};
use etalon_model::inference::qwen_attention::QkNorm;
use std::sync::Arc;

impl QTensorEngine {
    /// The execution owner is bound to the same immutable package and tensor
    /// directory. Public legacy engine fields cannot redirect its descriptors.
    pub(crate) fn qwen35_plan(&self, index: &GgufTensorIndex) -> Option<&Arc<Plan>> {
        let session = self.qwen35_ftw.as_ref()?;
        if !session.is_valid() {
            return None;
        }
        let plan = session.plan();
        let package = self.ftw_package.as_ref()?;
        let current = self.tensor_index_cache.as_ref()?;
        if !Arc::ptr_eq(package, &plan.pkg)
            || self.hyperparams != package.tensor_index.hyperparams
            || self.tensor_data_offset != package.tensor_index.tensor_data_start
            || current.hyperparams != index.hyperparams
            || current.entries != index.entries
            || current.moe_routing != index.moe_routing
            || current.tensor_data_start != index.tensor_data_start
            || index.hyperparams != package.tensor_index.hyperparams
            || index.entries != package.tensor_index.entries
            || index.moe_routing != package.tensor_index.moe_routing
            || index.tensor_data_start != package.tensor_index.tensor_data_start
        {
            return None;
        }
        Some(plan)
    }

    pub(crate) fn take_qwen35_session(&mut self, index: &GgufTensorIndex) -> Option<Session> {
        self.qwen35_plan(index)?;
        let mut session = self.qwen35_ftw.take()?;
        session.reset();
        Some(session)
    }

    pub(crate) fn qwen35_attention(
        &mut self,
        index: &GgufTensorIndex,
        layer: u32,
        hidden: &mut [f32],
        width: usize,
        tensors: &LayerTensors,
    ) -> bool {
        let Some(plan) = self.qwen35_plan(index) else {
            return false;
        };
        let Some(admitted) = plan.layers.get(layer as usize) else {
            return false;
        };
        if width != plan.profile.hidden
            || hidden.len() != width
            || tensors.attn_norm != Some(admitted.input_norm)
            || !matches!(
                admitted.attention,
                super::attention::AttentionPlan::Linear(_)
            )
        {
            return false;
        }
        self.qwen35_ftw
            .as_mut()
            .is_some_and(|s| s.attention(layer as usize, hidden).is_ok())
    }

    pub(crate) fn qwen35_ffn(
        &mut self,
        index: &GgufTensorIndex,
        hidden: &mut [f32],
        width: usize,
        tensors: &LayerTensors,
    ) -> bool {
        let Some(plan) = self.qwen35_plan(index) else {
            return false;
        };
        if width != plan.profile.hidden || hidden.len() != width {
            return false;
        }
        // This legacy API has no layer argument. Match both retained norms and
        // router, never guess a layer from an expert id or tensor dimensions.
        let mut matches = plan.layers.iter().enumerate().filter(|(_, p)| {
            tensors.ffn_norm == Some(p.post_norm) && tensors.moe_router == Some(p.router)
        });
        let Some((layer, _)) = matches.next() else {
            return false;
        };
        if matches.next().is_some() {
            return false;
        }
        self.qwen35_ftw
            .as_mut()
            .is_some_and(|s| s.ffn(layer, hidden).is_ok())
    }

    pub(crate) fn qwen35_forward(
        &mut self,
        index: &GgufTensorIndex,
        hidden: &mut [f32],
        width: usize,
        position: u32,
        max_layers: u32,
    ) -> u32 {
        let Some(plan) = self.qwen35_plan(index) else {
            return 0;
        };
        if width != plan.profile.hidden || hidden.len() != width {
            return 0;
        }
        let limit = if max_layers == 0 {
            plan.layers.len()
        } else {
            (max_layers as usize).min(plan.layers.len())
        };
        let Some(session) = &mut self.qwen35_ftw else {
            return 0;
        };
        if session.position() != Ok(position as usize) {
            return 0;
        }
        session.forward(0..limit, hidden).map_or(0, |n| n as u32)
    }

    pub(crate) fn qwen35_output_norm(
        &self,
        index: &GgufTensorIndex,
        hidden: &mut [f32],
        width: usize,
    ) -> bool {
        let Some(plan) = self.qwen35_plan(index) else {
            return false;
        };
        if width != plan.profile.hidden || hidden.len() != width || width > 16384 {
            return false;
        }
        let mut weights = [0.; 16384];
        let mut result = [0.; 16384];
        let source = DenseSource(&plan.pkg);
        let host = DenseHost { source: &source };
        if host.read(&plan.final_norm, &mut weights[..width]).is_err()
            || ordinary_rms_norm(
                hidden,
                &weights[..width],
                plan.profile.ordinary_norm == QkNorm::RawZeroCentered,
                plan.profile.epsilon,
                &mut result[..width],
            )
            .is_err()
        {
            return false;
        }
        hidden.copy_from_slice(&result[..width]);
        true
    }

    pub(crate) fn qwen35_argmax(
        &self,
        index: &GgufTensorIndex,
        hidden: &[f32],
        width: usize,
        max_chunks: u32,
        mask: Option<&crate::neuro_symbolic_sieve::SieveStateMask>,
    ) -> Option<StreamingArgmaxResult> {
        let plan = self.qwen35_plan(index)?;
        if width != plan.profile.hidden
            || hidden.len() != width
            || width > 16384
            || hidden.iter().any(|v| !v.is_finite())
        {
            return None;
        }
        let rows = if max_chunks == 0 {
            plan.output.rows
        } else {
            (max_chunks as usize)
                .checked_mul(etalon_inference::projections::VOCAB_CHUNK_ROWS)?
                .min(plan.output.rows)
        };
        let mut weights = [0.; 16384];
        let mut best = StreamingArgmaxResult {
            best_token_id: 0,
            max_logit: f32::NEG_INFINITY,
        };
        for row in 0..rows {
            if mask.is_some_and(|mask| !mask.allows(row as u32)) {
                continue;
            }
            plan.output
                .row(&plan.pkg, row, &mut weights[..width])
                .ok()?;
            let logit: f32 = hidden
                .iter()
                .zip(&weights[..width])
                .map(|(x, w)| x * w)
                .sum();
            if !logit.is_finite() {
                return None;
            }
            if logit > best.max_logit {
                best = StreamingArgmaxResult {
                    best_token_id: row as u32,
                    max_logit: logit,
                };
            }
        }
        best.max_logit.is_finite().then_some(best)
    }
}
