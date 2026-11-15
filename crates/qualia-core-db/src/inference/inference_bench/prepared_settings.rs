//! Immutable numerical settings for a single native execution scope.
//! Cold capture; scalar getters borrow TLS without allocation. Worker threads
//! must explicitly enter a clone before numerical work. Provider identity and
//! live semantic inputs are separate requirements, never inferred from settings.
use sha2::{Digest, Sha256};
use std::{cell::RefCell, marker::PhantomData, rc::Rc};
use std::{env::VarError, ffi::OsString};

const ENV_KEYS: &[&str] = &[
    "QUALIA_HYBRID_ROUTE",
    "QUALIA_GRAPH_FORCE",
    "QUALIA_LLM_CUDA_TC_REDUCED",
    "QUALIA_LLM_PROFILE_DECODE",
    "QUALIA_CUDA_CONTEXT",
    "QUALIA_DIRECTML",
    "QUALIA_LLM_CUDA_DECODE",
    "QUALIA_LLM_CUDA_TC_DECODE",
    "QUALIA_LLM_CUDA_TC_PREWARM",
    "QUALIA_LLM_DEBUG_DECODE",
    "QUALIA_LLM_DIRECTML",
    "QUALIA_LLM_DUAL_MR",
    "QUALIA_LLM_FFN_MR",
    "QUALIA_LLM_FFN_WARP",
    "QUALIA_LLM_MULTIROW",
    "QUALIA_LLM_TRIPLE_QKV",
    "QUALIA_LLM_WARP_GEMV",
];
const MAX_ENV_VALUE_BYTES: usize = 32768;
const MAX_SETTINGS_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone, Copy)]
pub struct SettingsCapacity;

#[derive(Clone, Debug)]
pub struct PreparedInferenceSettings {
    #[cfg(feature = "cuda")]
    cached_cuda_tuning: [u8; 32],
    pub(crate) inference_mode: crate::inference_modes::InferenceMode,
    environment: Vec<(&'static str, Option<OsString>)>,
    pub(crate) decode_budget_override: u32,
    pub(crate) inference_timeout_ms: u64,
    pub(crate) gpu_topk_enabled: bool,
    pub(crate) ternary_ffn_enabled: bool,
    pub(crate) attention_preproject_enabled: bool,
    pub(crate) attention_o_fuse_enabled: bool,
    pub(crate) forge_attention_enabled: bool,
    pub(crate) resident_weights_enabled: bool,
    pub(crate) resident_decode_enabled: bool,
    pub(crate) resident_prefill_enabled: bool,
    pub(crate) spec_decode_enabled: bool,
    pub(crate) kv_int8_enabled: bool,
    pub(crate) kv_dict_enabled: bool,
    pub(crate) sampler_config: Option<crate::sampler::SamplerConfig>,
    pub(crate) ffn_fusion_enabled: bool,
    pub(crate) ffn_f16_enabled: bool,
    pub(crate) coop_gemv_enabled: bool,
    pub(crate) coopmat_gemm_enabled: bool,
    pub(crate) cpu_attention_enabled: bool,
    pub(crate) prefix_cache_enabled: bool,
    pub(crate) kv_pool_budget_mb: u32,
    pub(crate) prefill_chunk_size_override: u32,
    pub(crate) prompt_precision_mode: String,
    pub(crate) stage_telemetry_enabled: bool,
}

thread_local! {
    static PREPARED: RefCell<Option<PreparedInferenceSettings>> = const { RefCell::new(None) };
}

pub(crate) fn read<T>(get: impl FnOnce(&PreparedInferenceSettings) -> T) -> Option<T> {
    PREPARED.with(|slot| slot.borrow().as_ref().map(get))
}

impl PreparedInferenceSettings {
    /// Capture actual effective getter values. Existing scopes stay immutable.
    pub fn capture() -> Result<Self, SettingsCapacity> {
        let mut environment = Vec::with_capacity(ENV_KEYS.len());
        let mut charged = 0usize;
        for &key in ENV_KEYS {
            let value = env_var_os(key);
            let bytes = value
                .as_ref()
                .map_or(0, |value| value.as_encoded_bytes().len());
            charged = charged.checked_add(bytes).ok_or(SettingsCapacity)?;
            if bytes > MAX_ENV_VALUE_BYTES || charged > MAX_SETTINGS_BYTES {
                return Err(SettingsCapacity);
            }
            environment.push((key, value));
        }
        let prompt_precision_mode = super::toggles::prompt_precision_mode();
        if prompt_precision_mode.len() > MAX_ENV_VALUE_BYTES {
            return Err(SettingsCapacity);
        }
        Ok(Self {
            #[cfg(feature = "cuda")]
            cached_cuda_tuning: crate::inference::cuda_lane::cached_tuning_digest(),
            inference_mode: crate::inference_modes::atomic_inference_mode(),
            environment,
            decode_budget_override: super::toggles::decode_budget_override(),
            inference_timeout_ms: super::toggles::inference_timeout_ms(),
            gpu_topk_enabled: super::toggles::gpu_topk_enabled(),
            ternary_ffn_enabled: super::toggles::ternary_ffn_enabled(),
            attention_preproject_enabled: super::toggles::attention_preproject_enabled(),
            attention_o_fuse_enabled: super::toggles::attention_o_fuse_enabled(),
            forge_attention_enabled: super::toggles::forge_attention_enabled(),
            resident_weights_enabled: super::toggles::resident_weights_enabled(),
            resident_decode_enabled: super::toggles::resident_decode_enabled(),
            resident_prefill_enabled: super::toggles::resident_prefill_enabled(),
            spec_decode_enabled: super::toggles::spec_decode_enabled(),
            kv_int8_enabled: super::toggles::kv_int8_enabled(),
            kv_dict_enabled: super::toggles::kv_dict_enabled(),
            sampler_config: super::toggles::sampler_config(),
            ffn_fusion_enabled: super::toggles::ffn_fusion_enabled(),
            ffn_f16_enabled: super::toggles::ffn_f16_enabled(),
            coop_gemv_enabled: super::toggles::coop_gemv_enabled(),
            coopmat_gemm_enabled: super::toggles::coopmat_gemm_enabled(),
            cpu_attention_enabled: super::toggles::cpu_attention_enabled(),
            prefix_cache_enabled: super::toggles::prefix_cache_enabled(),
            kv_pool_budget_mb: super::toggles::kv_pool_budget_mb(),
            prefill_chunk_size_override: super::toggles::prefill_chunk_size_override(),
            prompt_precision_mode,
            stage_telemetry_enabled: super::toggles::stage_telemetry_enabled(),
        })
    }

    /// Canonical numeric-bit digest, including the consumed sampling seed.
    pub fn digest(&self) -> [u8; 32] {
        let mut hash = Sha256::new();
        hash.update(b"qualia:prepared-inference-settings:v1\0");
        #[cfg(feature = "cuda")]
        hash.update(self.cached_cuda_tuning);
        hash.update([self.inference_mode as u8]);
        for (key, value) in &self.environment {
            hash.update((key.len() as u64).to_le_bytes());
            hash.update(key.as_bytes());
            if let Some(value) = value {
                hash.update([1]);
                hash.update((value.as_encoded_bytes().len() as u64).to_le_bytes());
                hash.update(value.as_encoded_bytes());
            } else {
                hash.update([0]);
            }
        }
        hash.update(b"decode_budget_override\0");
        hash.update(self.decode_budget_override.to_le_bytes());
        hash.update(b"inference_timeout_ms\0");
        hash.update(self.inference_timeout_ms.to_le_bytes());
        hash.update(b"gpu_topk_enabled\0");
        hash.update([u8::from(self.gpu_topk_enabled)]);
        hash.update(b"ternary_ffn_enabled\0");
        hash.update([u8::from(self.ternary_ffn_enabled)]);
        hash.update(b"attention_preproject_enabled\0");
        hash.update([u8::from(self.attention_preproject_enabled)]);
        hash.update(b"attention_o_fuse_enabled\0");
        hash.update([u8::from(self.attention_o_fuse_enabled)]);
        hash.update(b"forge_attention_enabled\0");
        hash.update([u8::from(self.forge_attention_enabled)]);
        hash.update(b"resident_weights_enabled\0");
        hash.update([u8::from(self.resident_weights_enabled)]);
        hash.update(b"resident_decode_enabled\0");
        hash.update([u8::from(self.resident_decode_enabled)]);
        hash.update(b"resident_prefill_enabled\0");
        hash.update([u8::from(self.resident_prefill_enabled)]);
        hash.update(b"spec_decode_enabled\0");
        hash.update([u8::from(self.spec_decode_enabled)]);
        hash.update(b"kv_int8_enabled\0");
        hash.update([u8::from(self.kv_int8_enabled)]);
        hash.update(b"kv_dict_enabled\0");
        hash.update([u8::from(self.kv_dict_enabled)]);
        hash.update(b"sampler_config\0");
        if let Some(config) = self.sampler_config {
            hash.update([1]);
            for value in [
                config.temperature,
                config.top_p,
                config.repeat_penalty,
                config.freq_penalty,
                config.presence_penalty,
            ] {
                hash.update(value.to_bits().to_le_bytes());
            }
            hash.update(config.top_k.to_le_bytes());
            hash.update(config.penalty_window.to_le_bytes());
            hash.update(config.seed.to_le_bytes());
        } else {
            hash.update([0]);
        }
        hash.update(b"ffn_fusion_enabled\0");
        hash.update([u8::from(self.ffn_fusion_enabled)]);
        hash.update(b"ffn_f16_enabled\0");
        hash.update([u8::from(self.ffn_f16_enabled)]);
        hash.update(b"coop_gemv_enabled\0");
        hash.update([u8::from(self.coop_gemv_enabled)]);
        hash.update(b"coopmat_gemm_enabled\0");
        hash.update([u8::from(self.coopmat_gemm_enabled)]);
        hash.update(b"cpu_attention_enabled\0");
        hash.update([u8::from(self.cpu_attention_enabled)]);
        hash.update(b"prefix_cache_enabled\0");
        hash.update([u8::from(self.prefix_cache_enabled)]);
        hash.update(b"kv_pool_budget_mb\0");
        hash.update(self.kv_pool_budget_mb.to_le_bytes());
        hash.update(b"prefill_chunk_size_override\0");
        hash.update(self.prefill_chunk_size_override.to_le_bytes());
        hash.update(b"prompt_precision_mode\0");
        hash.update((self.prompt_precision_mode.len() as u64).to_le_bytes());
        hash.update(self.prompt_precision_mode.as_bytes());
        hash.update(b"stage_telemetry_enabled\0");
        hash.update([u8::from(self.stage_telemetry_enabled)]);
        hash.finalize().into()
    }

    /// Runs only on the preparing thread. Nested scopes restore exact prior state.
    pub fn enter(self) -> PreparedSettingsGuard {
        let previous = PREPARED.with(|slot| slot.replace(Some(self)));
        PreparedSettingsGuard {
            previous,
            _thread: PhantomData,
        }
    }
}

/// Thread-affine RAII scope; unwind restores the previous settings.
pub struct PreparedSettingsGuard {
    previous: Option<PreparedInferenceSettings>,
    _thread: PhantomData<Rc<()>>,
}
impl Drop for PreparedSettingsGuard {
    fn drop(&mut self) {
        PREPARED.with(|slot| {
            slot.replace(self.previous.take());
        });
    }
}

/// Frozen environment for enrolled numerical settings; unrelated keys preserve
/// normal process behavior. Values retain exact native OS encoding.
pub fn env_var_os(key: &str) -> Option<OsString> {
    if let Some(value) = read(|settings| {
        settings
            .environment
            .iter()
            .find(|(name, _)| *name == key)
            .map(|(_, value)| value.clone())
    }) {
        if let Some(value) = value {
            return value;
        }
    }
    std::env::var_os(key)
}
pub fn env_var(key: &str) -> Result<String, VarError> {
    env_var_os(key)
        .ok_or(VarError::NotPresent)?
        .into_string()
        .map_err(VarError::NotUnicode)
}
