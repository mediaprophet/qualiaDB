//! Architecture hyper-parameters parsed from the GGUF KV section, plus the
//! architecture id / feature-flag constants and the `general.architecture` mapper.

/// Default RoPE base for Llama 3 / SmolLM2 when GGUF omits `llama.rope.freq_base`.
pub const DEFAULT_ROPE_FREQ_BASE: f32 = 100_000.0;

/// Architecture id (stored in P64 hparams + used for support gating).
pub const ARCH_UNKNOWN: u32 = 0;
pub const ARCH_LLAMA: u32 = 1;
pub const ARCH_GEMMA: u32 = 2;
pub const ARCH_GEMMA2: u32 = 3;
pub const ARCH_GEMMA3: u32 = 4;
/// Gemma 4 (E2B/E4B/…): dual head-dim SWA+global, PLE, shared KV — **not** standard Llama-shape.
pub const ARCH_GEMMA4: u32 = 5;
pub const ARCH_QWEN2: u32 = 6;
pub const ARCH_GLM4: u32 = 7;
pub const ARCH_PHI4: u32 = 8;
pub const ARCH_DEEPSEEK_MOE: u32 = 9;
pub const ARCH_OTHER: u32 = 255;

/// Feature flags on [`GgufHyperparams::arch_flags`].
pub const ARCH_FLAG_HAS_PLE: u32 = 1 << 0;
pub const ARCH_FLAG_HAS_SWA: u32 = 1 << 1;
pub const ARCH_FLAG_HAS_SHARED_KV: u32 = 1 << 2;
pub const ARCH_FLAG_HAS_QK_NORM: u32 = 1 << 3;
pub const ARCH_FLAG_HAS_SOFTCAP: u32 = 1 << 4;

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
    /// [`ARCH_*`] id from `general.architecture` (and tensor-feature refinement).
    pub architecture: u32,
    /// [`ARCH_FLAG_*`] bitmask.
    pub arch_flags: u32,
}

impl GgufHyperparams {
    pub fn effective_rope_freq_base(&self) -> f32 {
        if self.rope_freq_base > 0.0 && self.rope_freq_base.is_finite() {
            self.rope_freq_base
        } else {
            DEFAULT_ROPE_FREQ_BASE
        }
    }

    /// Effective position divisor for RoPE (`scaled_pos = pos / scale`).
    pub fn effective_rope_scale(&self) -> f32 {
        if self.rope_scale > 0.0 && self.rope_scale.is_finite() {
            self.rope_scale
        } else {
            1.0
        }
    }

    /// Nominal `head_dim` (`n_embd / n_head` if `head_dim == 0`).
    pub fn effective_head_dim(&self) -> u32 {
        if self.head_dim > 0 {
            self.head_dim
        } else if self.n_head > 0 {
            self.n_embd / self.n_head
        } else {
            128
        }
    }

    pub fn head_dim(&self) -> u32 {
        self.effective_head_dim()
    }

    pub fn effective_n_kv_head(&self) -> u32 {
        if self.n_kv_head > 0 {
            self.n_kv_head
        } else {
            self.n_head.max(1)
        }
    }

    pub fn q_heads_per_kv(&self) -> u32 {
        let kv = self.effective_n_kv_head();
        if kv == 0 {
            1
        } else {
            (self.n_head / kv).max(1)
        }
    }

    pub fn gqa_ratio(&self) -> u32 {
        self.q_heads_per_kv()
    }

    pub fn head_dim_swa_or(&self) -> u32 {
        if self.head_dim_swa > 0 {
            self.head_dim_swa
        } else {
            self.effective_head_dim()
        }
    }

    /// Human-readable architecture name for logs / errors.
    pub fn architecture_name(&self) -> &'static str {
        match self.architecture {
            ARCH_LLAMA => "llama",
            ARCH_GEMMA => "gemma",
            ARCH_GEMMA2 => "gemma2",
            ARCH_GEMMA3 => "gemma3",
            ARCH_GEMMA4 => "gemma4",
            ARCH_QWEN2 => "qwen2",
            ARCH_GLM4 => "glm4",
            ARCH_PHI4 => "phi4",
            ARCH_DEEPSEEK_MOE => "deepseek_moe",
            ARCH_OTHER => "other",
            _ => "unknown",
        }
    }

    /// Whether the native decode path can run this architecture coherently.
    ///
    /// Gemma 4 (E2B/E4B) requires PLE, dual-RoPE SWA/global head dims, QK-norm, post-norms,
    /// variable FFN width, and shared KV — none of which the Llama-shaped decode path implements.
    /// Running it produces multilingual garbage (measured 2026-07-09 on gemma-4-E2B-it-Q4_K_M).
    /// Override with `QUALIA_LLM_FORCE_UNSUPPORTED_ARCH=1` only for bring-up.
    pub fn decode_supported(&self) -> Result<(), String> {
        if std::env::var_os("QUALIA_LLM_FORCE_UNSUPPORTED_ARCH").is_some() {
            return Ok(());
        }
        if self.architecture == ARCH_GEMMA4
            || (self.arch_flags & ARCH_FLAG_HAS_PLE) != 0
            || (self.arch_flags & ARCH_FLAG_HAS_SHARED_KV) != 0
        {
            let mut missing = Vec::new();
            if (self.arch_flags & ARCH_FLAG_HAS_PLE) != 0 {
                missing.push("per-layer embeddings (PLE)");
            }
            if (self.arch_flags & ARCH_FLAG_HAS_SWA) != 0 {
                missing.push("sliding-window + dual head_dim");
            }
            if (self.arch_flags & ARCH_FLAG_HAS_SHARED_KV) != 0 {
                missing.push("shared KV layers");
            }
            if (self.arch_flags & ARCH_FLAG_HAS_QK_NORM) != 0 {
                missing.push("QK-norm");
            }
            if missing.is_empty() {
                missing.push("gemma4 decoder graph");
            }
            return Err(format!(
                "architecture '{}' is not supported by the native Llama-shaped decode path yet \
                 (missing: {}). Convert/activate to p64 still works; coherent inference needs the \
                 gemma4 graph. Set QUALIA_LLM_FORCE_UNSUPPORTED_ARCH=1 to force (will be garbage).",
                self.architecture_name(),
                missing.join(", ")
            ));
        }
        Ok(())
    }
}

/// Map `general.architecture` GGUF string → [`ARCH_*`].
pub fn parse_architecture_id(name: &str) -> u32 {
    let n = name.trim().to_ascii_lowercase();
    match n.as_str() {
        "llama" | "llama2" | "llama3" => ARCH_LLAMA,
        "gemma" => ARCH_GEMMA,
        "gemma2" => ARCH_GEMMA2,
        "gemma3" => ARCH_GEMMA3,
        "gemma4" => ARCH_GEMMA4,
        "qwen2" | "qwen2vl" | "qwen3" | "qwen3.5" | "qwen3.6" => ARCH_QWEN2,
        "glm" | "glm4" | "glm4.7" | "chatglm" => ARCH_GLM4,
        "phi" | "phi3" | "phi4" => ARCH_PHI4,
        "deepseek" | "deepseek2" | "deepseek3" | "deepseek_moe" => ARCH_DEEPSEEK_MOE,
        "" => ARCH_UNKNOWN,
        _ => ARCH_OTHER,
    }
}
