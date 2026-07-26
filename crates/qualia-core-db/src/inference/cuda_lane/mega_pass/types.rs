/// Per-layer weight references for the prepared CUDA decode pass.
pub struct MegaPassLayerWeights<'a> {
    pub q_raw: &'a [u8],
    pub k_raw: &'a [u8],
    pub v_raw: &'a [u8],
    pub o_raw: &'a [u8],
    pub gate_raw: &'a [u8],
    pub up_raw: &'a [u8],
    pub down_raw: &'a [u8],
}

/// Per-layer matmul dimensions resolved during cold plan construction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MegaPassLayerDims {
    pub q_in: usize,
    pub q_out: usize,
    pub kv_in: usize,
    pub kv_out: usize,
    pub o_in: usize,
    pub o_out: usize,
    pub gate_in: usize,
    pub gate_out: usize,
    pub up_in: usize,
    pub up_out: usize,
    pub down_in: usize,
    pub down_out: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MegaPassWeightLayout {
    Q4KSoa,
    Q8_0,
}

/// Immutable view of the cold-prepared weights and tensor metadata.
pub trait MegaPassPlanView {
    fn weight_layout(&self) -> MegaPassWeightLayout;
    fn layer_count(&self) -> usize;
    fn layer_weights(&self, layer: usize) -> Option<MegaPassLayerWeights<'_>>;
    fn layer_dims(&self, layer: usize) -> Option<&MegaPassLayerDims>;
    fn layer_weight_keys(&self, layer: usize) -> Option<[u64; 7]>;
    fn layer_norm_keys(&self, layer: usize) -> Option<[u64; 2]>;
}
