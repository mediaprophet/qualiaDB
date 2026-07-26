use std::ops::Range;
use std::sync::Arc;

use crate::inference::cuda_lane::{
    MegaPassLayerDims, MegaPassLayerWeights, MegaPassPlanView, MegaPassWeightLayout,
};

pub(crate) struct CudaLayerPlan {
    pub dims: MegaPassLayerDims,
    pub weights: [Range<usize>; 7],
    pub weight_keys: [u64; 7],
    pub norm_keys: [u64; 2],
}

pub(crate) struct CudaDecodePlan {
    pub key: (u64, u64, u32),
    pub weight_layout: MegaPassWeightLayout,
    pub mmap: Arc<memmap2::Mmap>,
    pub layers: Vec<CudaLayerPlan>,
    pub output_norm_key: Option<u64>,
    pub token_embedding_key: Option<u64>,
    pub token_embedding_vocab: usize,
    pub lm_head: Option<Range<usize>>,
    pub lm_head_key: Option<u64>,
    pub lm_head_in: usize,
    pub lm_head_out: usize,
    pub n_embd: usize,
    pub n_head: usize,
    pub n_kv: usize,
    pub head_dim: usize,
    pub n_layer: u32,
    pub max_context: u32,
    pub layer_stride: u32,
    pub slot_kv_elems: u32,
    pub rope_base: f32,
    pub rope_scale: f32,
    pub rms_eps: f32,
}

pub(crate) enum CudaDecodePlanState {
    Unbuilt,
    Ineligible(u64),
    Ready(Box<CudaDecodePlan>),
}

impl MegaPassPlanView for CudaDecodePlan {
    fn weight_layout(&self) -> MegaPassWeightLayout {
        self.weight_layout
    }

    fn layer_count(&self) -> usize {
        self.layers.len()
    }

    fn layer_weights(&self, layer: usize) -> Option<MegaPassLayerWeights<'_>> {
        let layer = self.layers.get(layer)?;
        let [q, k, v, o, gate, up, down] = &layer.weights;
        Some(MegaPassLayerWeights {
            q_raw: self.mmap.get(q.clone())?,
            k_raw: self.mmap.get(k.clone())?,
            v_raw: self.mmap.get(v.clone())?,
            o_raw: self.mmap.get(o.clone())?,
            gate_raw: self.mmap.get(gate.clone())?,
            up_raw: self.mmap.get(up.clone())?,
            down_raw: self.mmap.get(down.clone())?,
        })
    }

    fn layer_dims(&self, layer: usize) -> Option<&MegaPassLayerDims> {
        self.layers.get(layer).map(|layer| &layer.dims)
    }

    fn layer_weight_keys(&self, layer: usize) -> Option<[u64; 7]> {
        self.layers.get(layer).map(|layer| layer.weight_keys)
    }

    fn layer_norm_keys(&self, layer: usize) -> Option<[u64; 2]> {
        self.layers.get(layer).map(|layer| layer.norm_keys)
    }
}
