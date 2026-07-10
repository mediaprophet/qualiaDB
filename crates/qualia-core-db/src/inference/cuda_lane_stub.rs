//! Stub when CUDA feature is off — CUDA lane APIs return false / no-op.

#![cfg(any(target_arch = "wasm32", not(feature = "cuda")))]

pub const MAX_DENSE_ELEMS: usize = 0;

pub fn weight_fingerprint(_raw: &[u8], _n_in: usize, _n_out: usize) -> u64 {
    0
}

pub fn cache_dense_weight(_key: u64, _n_in: usize, _n_out: usize, _data: Vec<f32>) {}

pub fn dense_weight_cached(_key: u64) -> bool {
    false
}

pub fn preload_q4k_soa_weights(_weights: &[(&[u8], usize, usize)]) -> usize {
    0
}

pub fn try_cuda_batch_gemv(
    _hidden: &[f32],
    _batch: usize,
    _n_in: usize,
    _n_out: usize,
    _weight: &[f32],
    _out: &mut [f32],
) -> bool {
    false
}

pub fn try_cuda_batch_gemv_cached_only(
    _key: u64,
    _hidden: &[f32],
    _batch: usize,
    _out: &mut [f32],
) -> bool {
    false
}

pub fn try_cuda_batch_gemv_cached(
    _key: u64,
    _hidden: &[f32],
    _batch: usize,
    _n_in: usize,
    _n_out: usize,
    _weight: &[f32],
    _out: &mut [f32],
) -> bool {
    false
}

pub fn weight_cache_len() -> usize {
    0
}

pub fn clear_weight_cache() {}

pub fn try_q4k_soa_gemv(
    _n_in: usize,
    _n_out: usize,
    _x: &[f32],
    _weight_raw: &[u8],
    _out: &mut [f32],
) -> bool {
    false
}

pub fn try_q4k_soa_qkv(
    _n_in: usize,
    _n_q: usize,
    _n_kv: usize,
    _x: &[f32],
    _q_raw: &[u8],
    _k_raw: &[u8],
    _v_raw: &[u8],
    _q_out: &mut [f32],
    _k_out: &mut [f32],
    _v_out: &mut [f32],
) -> bool {
    false
}

pub fn try_q4k_soa_fused_swiglu(
    _n_in: usize,
    _n_out: usize,
    _x: &[f32],
    _gate_raw: &[u8],
    _up_raw: &[u8],
    _out: &mut [f32],
) -> bool {
    false
}

pub fn try_q4k_soa_ffn_block_residual(
    _n_embd: usize,
    _n_ffn: usize,
    _x_normed: &[f32],
    _residual: &[f32],
    _gate_raw: &[u8],
    _up_raw: &[u8],
    _down_raw: &[u8],
    _out: &mut [f32],
) -> bool {
    false
}

pub fn try_q4k_soa_ffn_block(
    _n_embd: usize,
    _n_ffn: usize,
    _x: &[f32],
    _gate_raw: &[u8],
    _up_raw: &[u8],
    _down_raw: &[u8],
    _out: &mut [f32],
) -> bool {
    false
}

pub fn q4k_device_weight_count() -> usize {
    0
}

pub fn warm_cuda_context() -> bool {
    false
}

pub fn ensure_device_kv_cache(
    _max_context: u32,
    _n_layer: u32,
    _n_kv_head: u32,
    _head_dim: u32,
    _slot_kv_elems: u32,
    _layer_stride: u32,
    _total_f32_elems: usize,
) -> bool {
    false
}

pub fn device_kv_ready() -> bool {
    false
}

pub fn try_q4k_soa_attention_device(
    _n_embd: usize,
    _n_head: usize,
    _n_kv: usize,
    _head_dim: usize,
    _layer: u32,
    _token_idx: u32,
    _max_context: u32,
    _layer_stride: u32,
    _slot_kv_elems: u32,
    _rope_base: f32,
    _rope_scale: f32,
    _x_normed: &[f32],
    _q_raw: &[u8],
    _k_raw: &[u8],
    _v_raw: &[u8],
    _o_raw: &[u8],
    _host_kv: Option<&mut [f32]>,
    _out_delta: &mut [f32],
) -> bool {
    false
}
