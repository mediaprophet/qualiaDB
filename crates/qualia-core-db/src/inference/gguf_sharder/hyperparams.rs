//! Qualia architecture admission policy over canonical numerical metadata.

pub use etalon_model::inference::{
    parse_architecture_id, ARCH_DEEPSEEK_MOE, ARCH_FLAG_HAS_PLE, ARCH_FLAG_HAS_QK_NORM,
    ARCH_FLAG_HAS_SHARED_KV, ARCH_FLAG_HAS_SOFTCAP, ARCH_FLAG_HAS_SWA, ARCH_GEMMA, ARCH_GEMMA2,
    ARCH_GEMMA3, ARCH_GEMMA4, ARCH_GLM4, ARCH_GRANITE_HYBRID, ARCH_LLAMA, ARCH_OTHER, ARCH_PHI4,
    ARCH_QWEN2, ARCH_QWEN35_MOE, ARCH_QWEN4EXP, ARCH_UNKNOWN, DEFAULT_ROPE_FREQ_BASE,
};

/// Architecture hyper-parameters parsed from the GGUF KV section.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct GgufHyperparams {
    pub n_layer: u32,
    pub n_embd: u32,
    pub n_head: u32,
    /// Grouped-query KV heads; `0` means MHA (`n_kv_head == n_head`).
    pub n_kv_head: u32,
    /// `llama.rope.freq_base` (FLOAT32 in GGUF); `0` → [`DEFAULT_ROPE_FREQ_BASE`].
    pub rope_freq_base: f32,
    /// Linear RoPE scale from `llama.rope.scale_linear` / `llama.rope.scaling.factor`; `0` → `1.0`.
    pub rope_scale: f32,
    /// Explicit head dim (`*.attention.key_length`); `0` → derive `n_embd / n_head`.
    pub head_dim: u32,
    /// SWA / local-attention head dim (`*.attention.key_length_swa`); `0` → same as `head_dim`.
    pub head_dim_swa: u32,
    /// Sliding-window size in tokens; `0` → full context attention only.
    pub sliding_window: u32,
    /// Last N layers share KV from the last non-shared layer of the same type (Gemma 4).
    pub shared_kv_layers: u32,
    /// Final logit softcapping (Gemma 2+); `0` → disabled.
    pub logit_softcap: f32,
    /// Qwen hybrid SSM causal-convolution kernel width.
    pub ssm_conv_kernel: u32,
    /// Qwen GatedDeltaNet recurrent state width.
    pub ssm_state_size: u32,
    /// Qwen SSM groups and timestep rank.
    pub ssm_group_count: u32,
    pub ssm_time_step_rank: u32,
    /// Qwen SSM inner projection width.
    pub ssm_inner_size: u32,
    /// Full-attention cadence among hybrid SSM layers (`0` means unspecified).
    pub full_attention_interval: u32,
    /// `ARCH_*()` id from `general.architecture` (and tensor-feature refinement).
    pub architecture: u32,
    /// `ARCH_FLAG_*()` bitmask.
    pub arch_flags: u32,
    pub rope_dim: u32,
    pub rope_dim_swa: u32,
    pub rope_freq_base_swa: f32,
    pub rms_norm_eps: f32,
    pub ple_embd_per_layer: u32,
}

impl GgufHyperparams {
    pub fn effective_rope_freq_base(&self) -> f32 {
        etalon_model::inference::ModelHyperparams::from(*self).effective_rope_freq_base()
    }
    pub fn effective_rope_scale(&self) -> f32 {
        etalon_model::inference::ModelHyperparams::from(*self).effective_rope_scale()
    }
    pub fn effective_head_dim(&self) -> u32 {
        etalon_model::inference::ModelHyperparams::from(*self).effective_head_dim()
    }
    pub fn head_dim(&self) -> u32 {
        etalon_model::inference::ModelHyperparams::from(*self).head_dim()
    }
    pub fn effective_n_kv_head(&self) -> u32 {
        etalon_model::inference::ModelHyperparams::from(*self).effective_n_kv_head()
    }
    pub fn q_heads_per_kv(&self) -> u32 {
        etalon_model::inference::ModelHyperparams::from(*self).q_heads_per_kv()
    }
    pub fn gqa_ratio(&self) -> u32 {
        etalon_model::inference::ModelHyperparams::from(*self).gqa_ratio()
    }
    pub fn head_dim_swa_or(&self) -> u32 {
        etalon_model::inference::ModelHyperparams::from(*self).head_dim_swa_or()
    }
    pub fn architecture_name(&self) -> &'static str {
        etalon_model::inference::ModelHyperparams::from(*self).architecture_name()
    }
    pub fn decode_supported(&self) -> Result<(), String> {
        etalon_model::inference::ModelHyperparams::from(*self).decode_supported_with_override(
            std::env::var_os("QUALIA_LLM_FORCE_UNSUPPORTED_ARCH").is_some(),
        )
    }
}

impl From<etalon_model::inference::ModelHyperparams> for GgufHyperparams {
    fn from(value: etalon_model::inference::ModelHyperparams) -> Self {
        Self {
            n_layer: value.n_layer,
            n_embd: value.n_embd,
            n_head: value.n_head,
            n_kv_head: value.n_kv_head,
            rope_freq_base: value.rope_freq_base,
            rope_scale: value.rope_scale,
            head_dim: value.head_dim,
            head_dim_swa: value.head_dim_swa,
            sliding_window: value.sliding_window,
            shared_kv_layers: value.shared_kv_layers,
            logit_softcap: value.logit_softcap,
            ssm_conv_kernel: value.ssm_conv_kernel,
            ssm_state_size: value.ssm_state_size,
            ssm_group_count: value.ssm_group_count,
            ssm_time_step_rank: value.ssm_time_step_rank,
            ssm_inner_size: value.ssm_inner_size,
            full_attention_interval: value.full_attention_interval,
            architecture: value.architecture,
            arch_flags: value.arch_flags,
            rope_dim: value.rope_dim,
            rope_dim_swa: value.rope_dim_swa,
            rope_freq_base_swa: value.rope_freq_base_swa,
            rms_norm_eps: value.rms_norm_eps,
            ple_embd_per_layer: value.ple_embd_per_layer,
        }
    }
}

impl From<GgufHyperparams> for etalon_model::inference::ModelHyperparams {
    fn from(value: GgufHyperparams) -> Self {
        Self {
            n_layer: value.n_layer,
            n_embd: value.n_embd,
            n_head: value.n_head,
            n_kv_head: value.n_kv_head,
            rope_freq_base: value.rope_freq_base,
            rope_scale: value.rope_scale,
            head_dim: value.head_dim,
            head_dim_swa: value.head_dim_swa,
            sliding_window: value.sliding_window,
            shared_kv_layers: value.shared_kv_layers,
            logit_softcap: value.logit_softcap,
            ssm_conv_kernel: value.ssm_conv_kernel,
            ssm_state_size: value.ssm_state_size,
            ssm_group_count: value.ssm_group_count,
            ssm_time_step_rank: value.ssm_time_step_rank,
            ssm_inner_size: value.ssm_inner_size,
            full_attention_interval: value.full_attention_interval,
            architecture: value.architecture,
            arch_flags: value.arch_flags,
            rope_dim: value.rope_dim,
            rope_dim_swa: value.rope_dim_swa,
            rope_freq_base_swa: value.rope_freq_base_swa,
            rms_norm_eps: value.rms_norm_eps,
            ple_embd_per_layer: value.ple_embd_per_layer,
        }
    }
}
