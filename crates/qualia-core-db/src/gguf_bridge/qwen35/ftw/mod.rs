//! Architecture-owned, explicitly admitted FreeToken Qwen3.5 text execution.
//! The generic GGUF/Llama admission remains unchanged.
pub mod admission;
pub mod attention;
mod bridge;
pub mod dense_host;
pub(crate) mod nvfp4_rows;
pub mod session;
pub(crate) mod session_ffn;
pub mod weights;

use crate::inference::moe::ftw_loader::FtwModelPackage;
use etalon_model::inference::{
    qwen_attention::{QkNorm, Qwen35AttentionGeometry},
    qwen_roles::{Qwen35Geometry, Qwen35Packing},
    TensorInfo,
};
use std::sync::Arc;
use weights::Nvfp4Projection;

#[derive(Clone, Debug)]
pub struct Profile {
    pub(crate) hidden: usize,
    pub(crate) layers: usize,
    pub(crate) vocab: usize,
    pub(crate) intermediate: usize,
    pub(crate) experts: usize,
    pub(crate) topk: usize,
    pub(crate) max_context: usize,
    pub(crate) epsilon: f32,
    pub(crate) linear_geometry: Qwen35Geometry,
    pub(crate) attention_geometry: Qwen35AttentionGeometry,
    pub(crate) rotary_inv_freq: Vec<f32>,
    pub(crate) packing: Qwen35Packing,
    pub(crate) ordinary_norm: QkNorm,
}

pub struct LayerPlan {
    pub(crate) attention: attention::AttentionPlan,
    pub(crate) input_norm: TensorInfo,
    pub(crate) post_norm: TensorInfo,
    pub(crate) router: TensorInfo,
    pub(crate) shared_gate: TensorInfo,
    pub(crate) shared_up: Nvfp4Projection,
    pub(crate) shared_down: Nvfp4Projection,
}

pub struct Plan {
    pub(crate) pkg: Arc<FtwModelPackage>,
    pub(crate) profile: Profile,
    pub(crate) layers: Vec<LayerPlan>,
    pub(crate) output: Nvfp4Projection,
    pub(crate) final_norm: TensorInfo,
    pub(crate) embedding: TensorInfo,
    pub(crate) reserved_bytes: usize,
}
impl Plan {
    pub(crate) fn reserved_bytes(&self) -> usize {
        self.reserved_bytes
    }
}
