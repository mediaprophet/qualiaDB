use crate::inference_bench::*;
use crate::inference_modes::{atomic_inference_mode, set_inference_mode, InferenceMode};
static TRIAL_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
pub(super) struct TrialState {
    _lock: std::sync::MutexGuard<'static, ()>,
    gpu_topk: bool,
    ternary_ffn: bool,
    kv_dict: bool,
    attention_preproject: bool,
    attention_o_fuse: bool,
    spec_decode: bool,
    ffn_fusion: bool,
    coop_gemv: bool,
    kv_int8: bool,
    resident_decode: bool,
    resident_prefill: bool,
    resident_weights: bool,
    prefix_cache: bool,
    budget: u32,
    pool: u32,
    chunk: u32,
    precision: String,
    mode: InferenceMode,
    env: Vec<(&'static str, Option<std::ffi::OsString>)>,
}
impl TrialState {
    pub(super) fn capture() -> Self {
        let lock = TRIAL_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let env = [
            "QUALIA_INFERENCE_MODE",
            "QUALIA_WGPU_BACKEND",
            "QUALIA_FORGE_BACKEND",
        ]
        .into_iter()
        .map(|k| (k, std::env::var_os(k)))
        .collect();
        Self {
            _lock: lock,
            gpu_topk: gpu_topk_enabled(),
            ternary_ffn: ternary_ffn_enabled(),
            kv_dict: kv_dict_enabled(),
            attention_preproject: attention_preproject_enabled(),
            attention_o_fuse: attention_o_fuse_enabled(),
            spec_decode: spec_decode_enabled(),
            ffn_fusion: ffn_fusion_enabled(),
            coop_gemv: coop_gemv_enabled(),
            kv_int8: kv_int8_enabled(),
            resident_decode: resident_decode_enabled(),
            resident_prefill: resident_prefill_enabled(),
            resident_weights: resident_weights_enabled(),
            prefix_cache: prefix_cache_enabled(),
            budget: decode_budget_override(),
            pool: kv_pool_budget_mb(),
            chunk: prefill_chunk_size_override(),
            precision: prompt_precision_mode(),
            mode: atomic_inference_mode(),
            env,
        }
    }
}
impl Drop for TrialState {
    fn drop(&mut self) {
        set_gpu_topk(self.gpu_topk);
        set_ternary_ffn(self.ternary_ffn);
        set_kv_dict(self.kv_dict);
        set_attention_preproject(self.attention_preproject);
        set_attention_o_fuse(self.attention_o_fuse);
        set_spec_decode(self.spec_decode);
        set_ffn_fusion(self.ffn_fusion);
        set_coop_gemv(self.coop_gemv);
        set_kv_int8(self.kv_int8);
        set_resident_decode(self.resident_decode);
        set_resident_prefill(self.resident_prefill);
        set_resident_weights(self.resident_weights);
        set_prefix_cache_enabled(self.prefix_cache);
        set_decode_budget_override(self.budget);
        set_kv_pool_budget_mb(self.pool);
        set_prefill_chunk_size_override(self.chunk);
        set_prompt_precision_mode(&self.precision);
        set_inference_mode(self.mode);
        for (name, value) in &self.env {
            if let Some(value) = value {
                std::env::set_var(name, value);
            } else {
                std::env::remove_var(name);
            }
        }
    }
}
