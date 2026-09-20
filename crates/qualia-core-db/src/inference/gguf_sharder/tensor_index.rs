//! `GgufTensorIndex` — the tensor-name-hash → info lookup table, built by walking
//! the GGUF tensor-info section (and the KV hyperparameter parse that precedes it),
//! plus the per-layer tensor-name helpers and token-embedding dequant access.

use super::*;

/// Upper bound for the cold GGUF header read.  This includes the vocabulary
/// metadata and tensor-info directory, never any tensor payload.  It keeps a
/// native external-trunk activation from reserving/mapping the full model.
pub const MAX_GGUF_HEADER_BYTES: usize = 64 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GgufHeaderError {
    Io,
    Invalid,
    ExceedsLimit,
}

impl core::fmt::Display for GgufHeaderError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Io => write!(f, "could not read GGUF metadata header"),
            Self::Invalid => write!(f, "GGUF metadata header is invalid or truncated"),
            Self::ExceedsLimit => write!(f, "GGUF metadata header exceeds the native 64 MiB limit"),
        }
    }
}

impl std::error::Error for GgufHeaderError {}

/// Lookup table from tensor-name hash → `GgufTensorInfo`, built by walking the
/// GGUF tensor-info section that immediately follows the KV metadata section.
#[derive(Clone)]
pub struct GgufTensorIndex {
    pub(crate) entries: Vec<(u64, GgufTensorInfo)>, // (name_hash, info)
    /// Absolute byte offset in the mmap where tensor payload data begins.
    pub tensor_data_start: u64,
    /// Cached metadata for `token_embd.weight` (embedding lookup target).
    pub(crate) token_embd: Option<GgufTensorInfo>,
    /// Cached `output.weight` for final vocabulary projection.
    pub(crate) output_weight: Option<GgufTensorInfo>,
    /// Cached `output_norm.weight` — final RMSNorm before vocab projection (Llama/SmolLM).
    pub(crate) output_norm: Option<GgufTensorInfo>,
    /// Qwen4Exp PLE n-gram embedding table. This is a direct NVMe row-gather
    /// source, never a staging/GPU-resident weight.
    pub(crate) ple_ngram_embedding: Option<GgufTensorInfo>,
    /// Exact PLE n-gram row-selection contract, when this GGUF supplies one.
    pub ple_config: Option<Qwen4ExpPleConfig>,
    pub hyperparams: GgufHyperparams,
    /// Largest tensor payload in the file (informational).
    pub max_tensor_bytes: usize,
    /// Largest layer matmul tensor (attn/ffn weights) — sizes reusable GPU staging.
    pub max_layer_tensor_bytes: usize,
}

/// True when `name` is a per-layer matmul weight consumed by `dispatch_transformer_layer`.
fn is_layer_matmul_tensor_name(name: &[u8]) -> bool {
    const SUFFIXES: [&[u8]; 16] = [
        b"attn_q.weight",
        b"attn_k.weight",
        b"attn_v.weight",
        b"attn_qkv.weight",
        b"attn_output.weight",
        b"ffn_gate.weight",
        b"ffn_up.weight",
        b"ffn_down.weight",
        b"ffn_gate_inp.weight",
        b"ffn_gate_exps.weight",
        b"ffn_up_exps.weight",
        b"ffn_gate_up_exps.weight",
        b"ffn_down_exps.weight",
        b"ffn_gate_shexp.weight",
        b"ffn_up_shexp.weight",
        b"ffn_down_shexp.weight",
    ];
    if !name.starts_with(b"blk.") {
        return false;
    }
    SUFFIXES.iter().any(|s| name.ends_with(s))
}

/// Write `blk.{layer}.{suffix}` into `out`; returns total bytes written.
pub fn write_blk_tensor_name(layer: u32, suffix: &[u8], out: &mut [u8]) -> usize {
    let prefix = b"blk.";
    let mut n = 0usize;
    if out.len() < prefix.len() + 1 + suffix.len() {
        return 0;
    }
    out[..prefix.len()].copy_from_slice(prefix);
    n += prefix.len();
    let mut v = layer;
    let mut digits = [0u8; 10];
    let mut d = 0usize;
    if v == 0 {
        digits[0] = b'0';
        d = 1;
    } else {
        while v > 0 && d < digits.len() {
            digits[d] = b'0' + (v % 10) as u8;
            v /= 10;
            d += 1;
        }
    }
    for i in (0..d).rev() {
        if n >= out.len() {
            return n;
        }
        out[n] = digits[i];
        n += 1;
    }
    if n >= out.len() {
        return n;
    }
    out[n] = b'.';
    n += 1;
    let copy = suffix.len().min(out.len() - n);
    out[n..n + copy].copy_from_slice(&suffix[..copy]);
    n + copy
}

/// Normalize a model-family tensor name into the canonical `blk.N.*` name
/// consumed by `get_layer_tensors` and `dispatch_prefill_layer_batch`.
///
/// This runs once while building an index, never in a decode or prefill hot
/// loop.  It makes the prefill path source-format independent for standard
/// Q/K/V models (Granite/Gemma/Qwen2-style Safetensors and GGUF names).
fn canonical_component_name(name: &[u8], out: &mut [u8; 128]) -> usize {
    let Ok(name) = core::str::from_utf8(name) else {
        return 0;
    };
    if name.contains("per_layer") || name.contains("ple.") {
        return 0;
    }
    // Qwen hybrid checkpoints use one fused `[Q;K;V]` projection.  It has no P64 role
    // equivalent to a single projection, so normalise it before the ordinary role mapper.
    if name.contains("qkv_proj") || name.contains("attn_qkv") {
        if let Some(layer) = tensor_layer_number(name) {
            return write_blk_tensor_name(layer, b"attn_qkv.weight", out);
        }
    }
    if name.contains("gate_up_exps") || name.contains("experts.gate_up_proj") {
        if let Some(layer) = tensor_layer_number(name) {
            return write_blk_tensor_name(layer, b"ffn_gate_up_exps.weight", out);
        }
    }
    let Some(role) = crate::inference::tensor_roles::name_to_role(name) else {
        return 0;
    };
    if role.layer == crate::p64_weight::P64_LAYER_GLOBAL {
        let global = match role.role {
            crate::p64_weight::P64_ROLE_TOKEN_EMBD => b"token_embd.weight".as_slice(),
            crate::p64_weight::P64_ROLE_OUTPUT => b"output.weight".as_slice(),
            crate::p64_weight::P64_ROLE_OUTPUT_NORM => b"output_norm.weight".as_slice(),
            _ => return 0,
        };
        if global.len() > out.len() {
            return 0;
        }
        out[..global.len()].copy_from_slice(global);
        return global.len();
    }
    let Some(suffix) = crate::inference::tensor_roles::canonical_suffix(role.role) else {
        return 0;
    };
    write_blk_tensor_name(role.layer as u32, suffix, out)
}

/// Extract a layer number from the two source naming families without allocating.
fn tensor_layer_number(name: &str) -> Option<u32> {
    for marker in ["blk.", "layers."] {
        let Some(pos) = name.find(marker) else {
            continue;
        };
        let mut value = 0u32;
        let mut seen = false;
        for byte in name.as_bytes()[pos + marker.len()..].iter().copied() {
            if !byte.is_ascii_digit() {
                break;
            }
            seen = true;
            value = value.checked_mul(10)?.checked_add((byte - b'0') as u32)?;
        }
        if seen {
            return Some(value);
        }
    }
    None
}

impl GgufTensorIndex {
    /// Read only the bounded GGUF metadata prefix needed to construct an
    /// index.  This is the native external-trunk activation route: it does
    /// not mmap the source model and does not touch its tensor payloads.
    pub fn from_gguf_header_file(path: &std::path::Path) -> Result<Self, GgufHeaderError> {
        use std::io::Read;

        let file_len = std::fs::metadata(path)
            .map_err(|_| GgufHeaderError::Io)?
            .len();
        let mut request = 8 * 1024 * 1024usize;
        loop {
            let bytes_to_read = (file_len as usize).min(request);
            if bytes_to_read < 24 {
                return Err(GgufHeaderError::Invalid);
            }
            let mut bytes = vec![0u8; bytes_to_read];
            let mut file = std::fs::File::open(path).map_err(|_| GgufHeaderError::Io)?;
            file.read_exact(&mut bytes)
                .map_err(|_| GgufHeaderError::Io)?;
            if let Some(index) = Self::try_build(&bytes) {
                if index.tensor_data_start as usize <= bytes.len() {
                    return Ok(index);
                }
            }
            if bytes_to_read as u64 == file_len {
                return Err(GgufHeaderError::Invalid);
            }
            if request >= MAX_GGUF_HEADER_BYTES {
                return Err(GgufHeaderError::ExceedsLimit);
            }
            request = (request.saturating_mul(2)).min(MAX_GGUF_HEADER_BYTES);
        }
    }

    pub fn from_gguf(mmap: &[u8]) -> Self {
        Self::try_build(mmap).unwrap_or_else(|| Self {
            entries: vec![],
            tensor_data_start: 0,
            token_embd: None,
            output_weight: None,
            output_norm: None,
            ple_ngram_embedding: None,
            ple_config: None,
            hyperparams: GgufHyperparams::default(),
            max_tensor_bytes: 0,
            max_layer_tensor_bytes: 0,
        })
    }

    fn parse_kv_hyperparams(
        key: &str,
        vtype: u32,
        mmap: &[u8],
        pos: &mut usize,
    ) -> GgufHyperparams {
        let mut patch = GgufHyperparams::default();
        // general.architecture = STRING
        if key == "general.architecture" && vtype == 8 {
            if *pos + 8 <= mmap.len() {
                let n =
                    u64::from_le_bytes(mmap[*pos..*pos + 8].try_into().unwrap_or([0; 8])) as usize;
                *pos += 8;
                if *pos + n <= mmap.len() {
                    let s = std::str::from_utf8(&mmap[*pos..*pos + n]).unwrap_or("");
                    patch.architecture = parse_architecture_id(s);
                    *pos += n;
                }
            }
            return patch;
        }
        if key.ends_with("rope.freq_base") && !key.contains("swa") {
            match vtype {
                6 if *pos + 4 <= mmap.len() => {
                    let bits =
                        u32::from_le_bytes(mmap[*pos..*pos + 4].try_into().unwrap_or([0; 4]));
                    *pos += 4;
                    patch.rope_freq_base = f32::from_bits(bits);
                }
                12 if *pos + 8 <= mmap.len() => {
                    let bits =
                        u64::from_le_bytes(mmap[*pos..*pos + 8].try_into().unwrap_or([0; 8]));
                    *pos += 8;
                    patch.rope_freq_base = f64::from_bits(bits) as f32;
                }
                _ => {
                    let _ = gguf_skip_value(mmap, pos, vtype);
                }
            }
            return patch;
        }
        if key.ends_with("final_logit_softcapping") || key.ends_with("attention.logit_softcapping")
        {
            match vtype {
                6 if *pos + 4 <= mmap.len() => {
                    let bits =
                        u32::from_le_bytes(mmap[*pos..*pos + 4].try_into().unwrap_or([0; 4]));
                    *pos += 4;
                    patch.logit_softcap = f32::from_bits(bits);
                    if patch.logit_softcap > 0.0 {
                        patch.arch_flags |= ARCH_FLAG_HAS_SOFTCAP;
                    }
                }
                12 if *pos + 8 <= mmap.len() => {
                    let bits =
                        u64::from_le_bytes(mmap[*pos..*pos + 8].try_into().unwrap_or([0; 8]));
                    *pos += 8;
                    patch.logit_softcap = f64::from_bits(bits) as f32;
                    if patch.logit_softcap > 0.0 {
                        patch.arch_flags |= ARCH_FLAG_HAS_SOFTCAP;
                    }
                }
                _ => {
                    let _ = gguf_skip_value(mmap, pos, vtype);
                }
            }
            return patch;
        }
        if key.ends_with("rope.scale_linear") || key.ends_with("rope.scaling.factor") {
            match vtype {
                6 if *pos + 4 <= mmap.len() => {
                    let bits =
                        u32::from_le_bytes(mmap[*pos..*pos + 4].try_into().unwrap_or([0; 4]));
                    *pos += 4;
                    patch.rope_scale = f32::from_bits(bits);
                }
                12 if *pos + 8 <= mmap.len() => {
                    let bits =
                        u64::from_le_bytes(mmap[*pos..*pos + 8].try_into().unwrap_or([0; 8]));
                    *pos += 8;
                    patch.rope_scale = f64::from_bits(bits) as f32;
                }
                4 if *pos + 4 <= mmap.len() => {
                    patch.rope_scale =
                        u32::from_le_bytes(mmap[*pos..*pos + 4].try_into().unwrap_or([0; 4]))
                            as f32;
                    *pos += 4;
                }
                _ => {
                    let _ = gguf_skip_value(mmap, pos, vtype);
                }
            }
            return patch;
        }
        if vtype != 4 {
            let _ = gguf_skip_value(mmap, pos, vtype);
            return patch;
        }
        if *pos + 4 > mmap.len() {
            return patch;
        }
        let v = u32::from_le_bytes(mmap[*pos..*pos + 4].try_into().unwrap_or([0; 4]));
        *pos += 4;
        if key.ends_with("block_count") {
            patch.n_layer = v;
        } else if key.ends_with("embedding_length") && !key.contains("per_layer") {
            patch.n_embd = v;
        } else if key.ends_with("attention.head_count") && !key.contains("kv") {
            patch.n_head = v;
        } else if key.contains("head_count_kv") || key.contains("n_kv_head") {
            patch.n_kv_head = v;
        } else if key.ends_with("attention.key_length_swa")
            || key.ends_with("attention.value_length_swa")
        {
            // Prefer key_length_swa; value_length_swa is the same for Gemma 4.
            if patch.head_dim_swa == 0 {
                patch.head_dim_swa = v;
            }
            if v > 0 {
                patch.arch_flags |= ARCH_FLAG_HAS_SWA;
            }
        } else if key.ends_with("attention.key_length") || key.ends_with("attention.value_length") {
            if !key.contains("swa") && patch.head_dim == 0 {
                patch.head_dim = v;
            }
        } else if key.ends_with("attention.sliding_window") {
            patch.sliding_window = v;
            if v > 0 {
                patch.arch_flags |= ARCH_FLAG_HAS_SWA;
            }
        } else if key.ends_with("attention.shared_kv_layers") {
            patch.shared_kv_layers = v;
            if v > 0 {
                patch.arch_flags |= ARCH_FLAG_HAS_SHARED_KV;
            }
        } else if key.ends_with("ssm.conv_kernel") {
            patch.ssm_conv_kernel = v;
        } else if key.ends_with("ssm.state_size") {
            patch.ssm_state_size = v;
        } else if key.ends_with("ssm.group_count") {
            patch.ssm_group_count = v;
        } else if key.ends_with("ssm.time_step_rank") {
            patch.ssm_time_step_rank = v;
        } else if key.ends_with("ssm.inner_size") {
            patch.ssm_inner_size = v;
        } else if key.ends_with("full_attention_interval") {
            patch.full_attention_interval = v;
        }
        patch
    }

    /// Consume Qwen4Exp PLE metadata into its fixed-capacity runtime contract.
    /// Returns `true` only when this function consumed the value.
    fn parse_ple_config_kv(
        key: &str,
        vtype: u32,
        mmap: &[u8],
        pos: &mut usize,
        config: &mut Qwen4ExpPleConfig,
    ) -> bool {
        fn scalar_u32(vtype: u32, mmap: &[u8], pos: &mut usize, field: &mut u32) -> bool {
            if vtype != 4 || *pos + 4 > mmap.len() {
                let _ = gguf_skip_value(mmap, pos, vtype);
                return false;
            }
            *field = u32::from_le_bytes(mmap[*pos..*pos + 4].try_into().unwrap_or([0; 4]));
            *pos += 4;
            true
        }

        if key.ends_with("ple.ngram_size") {
            return scalar_u32(vtype, mmap, pos, &mut config.ngram_size);
        }
        if key.ends_with("ple.heads_per_ngram") {
            return scalar_u32(vtype, mmap, pos, &mut config.heads_per_ngram);
        }
        if key.ends_with("ple.eos_token_id") {
            return scalar_u32(vtype, mmap, pos, &mut config.eos_token_id);
        }
        if key.ends_with("embedding_length_per_layer_input") {
            return scalar_u32(vtype, mmap, pos, &mut config.embedding_row_width);
        }

        let target: Option<(&mut [i64], &mut u32)> = if key.ends_with("ple.layer_multipliers") {
            Some((&mut config.layer_multipliers, &mut config.multiplier_count))
        } else if key.ends_with("ple.head_offsets") {
            Some((&mut config.head_offsets, &mut config.head_count))
        } else if key.ends_with("ple.head_vocab_sizes") {
            Some((&mut config.head_vocab_sizes, &mut config.head_count))
        } else {
            None
        };
        if let Some((target, count)) = target {
            if vtype != 9 || *pos + 12 > mmap.len() {
                let _ = gguf_skip_value(mmap, pos, vtype);
                return false;
            }
            let element_type =
                u32::from_le_bytes(mmap[*pos..*pos + 4].try_into().unwrap_or([0; 4]));
            *pos += 4;
            let items = u64::from_le_bytes(mmap[*pos..*pos + 8].try_into().unwrap_or([0; 8]));
            *pos += 8;
            if element_type != 10
                || items > target.len() as u64
                || *pos + items as usize * 8 > mmap.len()
            {
                for _ in 0..items {
                    if gguf_skip_value(mmap, pos, element_type).is_none() {
                        break;
                    }
                }
                return true;
            }
            for slot in target.iter_mut().take(items as usize) {
                *slot = i64::from_le_bytes(mmap[*pos..*pos + 8].try_into().unwrap_or([0; 8]));
                *pos += 8;
            }
            *count = items as u32;
            return true;
        }

        if key.ends_with("ple.layers") {
            if vtype != 9 || *pos + 12 > mmap.len() {
                let _ = gguf_skip_value(mmap, pos, vtype);
                return false;
            }
            let element_type =
                u32::from_le_bytes(mmap[*pos..*pos + 4].try_into().unwrap_or([0; 4]));
            *pos += 4;
            let items = u64::from_le_bytes(mmap[*pos..*pos + 8].try_into().unwrap_or([0; 8]));
            *pos += 8;
            if element_type != 5
                || items > config.layers.len() as u64
                || *pos + items as usize * 4 > mmap.len()
            {
                for _ in 0..items {
                    if gguf_skip_value(mmap, pos, element_type).is_none() {
                        break;
                    }
                }
                return true;
            }
            for slot in config.layers.iter_mut().take(items as usize) {
                *slot = u32::from_le_bytes(mmap[*pos..*pos + 4].try_into().unwrap_or([0; 4]));
                *pos += 4;
            }
            config.layer_count = items as u32;
            return true;
        }
        false
    }

    fn try_build(mmap: &[u8]) -> Option<Self> {
        if mmap.len() < 24 || &mmap[0..4] != b"GGUF" {
            return None;
        }
        let version = u32::from_le_bytes(mmap[4..8].try_into().ok()?);
        if version < 2 {
            return None;
        }
        let tensor_count = u64::from_le_bytes(mmap[8..16].try_into().ok()?);
        let kv_count = u64::from_le_bytes(mmap[16..24].try_into().ok()?);

        let mut hyperparams = GgufHyperparams::default();
        let mut ple_config = Qwen4ExpPleConfig::default();
        let mut pos = 24usize;
        for _ in 0..kv_count {
            if pos + 8 > mmap.len() {
                return None;
            }
            let klen = u64::from_le_bytes(mmap[pos..pos + 8].try_into().ok()?) as usize;
            pos += 8;
            if pos + klen + 4 > mmap.len() {
                return None;
            }
            let key = std::str::from_utf8(&mmap[pos..pos + klen]).unwrap_or("");
            pos += klen;
            let vtype = u32::from_le_bytes(mmap[pos..pos + 4].try_into().ok()?);
            pos += 4;
            if Self::parse_ple_config_kv(key, vtype, mmap, &mut pos, &mut ple_config) {
                continue;
            }
            let patch = Self::parse_kv_hyperparams(key, vtype, mmap, &mut pos);
            if patch.n_layer != 0 {
                hyperparams.n_layer = patch.n_layer;
            }
            if patch.n_embd != 0 {
                hyperparams.n_embd = patch.n_embd;
            }
            if patch.n_head != 0 {
                hyperparams.n_head = patch.n_head;
            }
            if patch.n_kv_head != 0 {
                hyperparams.n_kv_head = patch.n_kv_head;
            }
            if patch.rope_freq_base > 0.0 {
                hyperparams.rope_freq_base = patch.rope_freq_base;
            }
            if patch.rope_scale > 0.0 {
                hyperparams.rope_scale = patch.rope_scale;
            }
            if patch.head_dim != 0 {
                hyperparams.head_dim = patch.head_dim;
            }
            if patch.head_dim_swa != 0 {
                hyperparams.head_dim_swa = patch.head_dim_swa;
            }
            if patch.sliding_window != 0 {
                hyperparams.sliding_window = patch.sliding_window;
            }
            if patch.shared_kv_layers != 0 {
                hyperparams.shared_kv_layers = patch.shared_kv_layers;
            }
            if patch.logit_softcap > 0.0 {
                hyperparams.logit_softcap = patch.logit_softcap;
            }
            if patch.ssm_conv_kernel != 0 {
                hyperparams.ssm_conv_kernel = patch.ssm_conv_kernel;
            }
            if patch.ssm_state_size != 0 {
                hyperparams.ssm_state_size = patch.ssm_state_size;
            }
            if patch.ssm_group_count != 0 {
                hyperparams.ssm_group_count = patch.ssm_group_count;
            }
            if patch.ssm_time_step_rank != 0 {
                hyperparams.ssm_time_step_rank = patch.ssm_time_step_rank;
            }
            if patch.ssm_inner_size != 0 {
                hyperparams.ssm_inner_size = patch.ssm_inner_size;
            }
            if patch.full_attention_interval != 0 {
                hyperparams.full_attention_interval = patch.full_attention_interval;
            }
            if patch.architecture != 0 {
                hyperparams.architecture = patch.architecture;
            }
            hyperparams.arch_flags |= patch.arch_flags;
        }

        let mut entries = Vec::with_capacity(tensor_count.min(4096) as usize);
        let mut max_tensor_bytes = 0usize;
        let mut max_layer_tensor_bytes = 0usize;
        for _ in 0..tensor_count {
            if pos + 8 > mmap.len() {
                return None;
            }
            let nlen = u64::from_le_bytes(mmap[pos..pos + 8].try_into().ok()?) as usize;
            pos += 8;
            if pos + nlen > mmap.len() {
                return None;
            }
            let name = &mmap[pos..pos + nlen];
            let name_hash = gguf_name_hash(name);
            pos += nlen;

            // n_dims
            if pos + 4 > mmap.len() {
                return None;
            }
            let n_dims_raw = u32::from_le_bytes(mmap[pos..pos + 4].try_into().ok()?) as usize;
            pos += 4;

            // Shape (up to 4 dims stored; rest skipped)
            let mut dims = [0u64; 4];
            for d in 0..n_dims_raw {
                if pos + 8 > mmap.len() {
                    return None;
                }
                let v = u64::from_le_bytes(mmap[pos..pos + 8].try_into().ok()?);
                pos += 8;
                if d < 4 {
                    dims[d] = v;
                }
            }

            // ggml_type + offset
            if pos + 12 > mmap.len() {
                return None;
            }
            let ggml_type = u32::from_le_bytes(mmap[pos..pos + 4].try_into().ok()?);
            pos += 4;
            let byte_offset = u64::from_le_bytes(mmap[pos..pos + 8].try_into().ok()?);
            pos += 8;

            let info = GgufTensorInfo {
                dims,
                n_dims: n_dims_raw.min(4) as u32,
                ggml_type,
                byte_offset,
            };
            if let Some(tb) = crate::ggml_quants::tensor_byte_len(&info) {
                max_tensor_bytes = max_tensor_bytes.max(tb);
                if is_layer_matmul_tensor_name(name) {
                    max_layer_tensor_bytes = max_layer_tensor_bytes.max(tb);
                }
            }
            entries.push((name_hash, info));
        }

        let tensor_data_start = ((pos as u64 + 31) & !31) as u64;
        let emb_hash = gguf_name_hash(b"token_embd.weight");
        let out_hash = gguf_name_hash(b"output.weight");
        let out_norm_hash = gguf_name_hash(b"output_norm.weight");
        let token_embd = entries
            .iter()
            .find(|(h, _)| *h == emb_hash)
            .map(|(_, i)| *i);
        let output_weight = entries
            .iter()
            .find(|(h, _)| *h == out_hash)
            .map(|(_, i)| *i);
        let output_norm = entries
            .iter()
            .find(|(h, _)| *h == out_norm_hash)
            .map(|(_, i)| *i);
        if hyperparams.n_embd == 0 {
            hyperparams.n_embd = token_embd.map(|t| t.dims[0] as u32).unwrap_or(0);
        }
        // Tensor-feature refinement. Qwen4Exp's PLE tensor is hash-gathered and must be
        // handled as an NVMe row source, never as an ordinary embedding or GEMM matrix.
        let ple_hash = gguf_name_hash(b"per_layer_token_embd.weight");
        let ple_alt_hash = gguf_name_hash(b"ple.ple_embedding.ngram_embedding.weight");
        let ple_ngram_embedding = entries
            .iter()
            .find(|(h, _)| *h == ple_hash || *h == ple_alt_hash)
            .map(|(_, i)| *i);
        if ple_ngram_embedding.is_some() {
            hyperparams.arch_flags |= ARCH_FLAG_HAS_PLE;
            if hyperparams.architecture == ARCH_UNKNOWN || hyperparams.architecture == ARCH_OTHER {
                hyperparams.architecture = ARCH_QWEN4EXP;
            }
        }
        let qk_norm_hash = gguf_name_hash(b"blk.0.attn_q_norm.weight");
        if entries.iter().any(|(h, _)| *h == qk_norm_hash) {
            hyperparams.arch_flags |= ARCH_FLAG_HAS_QK_NORM;
        }
        // Granite-H uses an SSM input/convolution block rather than normal Q/K/V.  Treat this
        // concrete tensor signature as authoritative when older converters omit or rename the
        // architecture metadata, so the attention-only runtime fails closed instead of skipping
        // recurrent state updates.
        let granite_ssm_in = gguf_name_hash(b"blk.0.ssm_in.weight");
        let granite_ssm_conv = gguf_name_hash(b"blk.0.ssm_conv1d.weight");
        if entries.iter().any(|(h, _)| *h == granite_ssm_in)
            && entries.iter().any(|(h, _)| *h == granite_ssm_conv)
            && (hyperparams.architecture == ARCH_UNKNOWN || hyperparams.architecture == ARCH_OTHER)
        {
            hyperparams.architecture = ARCH_GRANITE_HYBRID;
        }
        Some(Self {
            entries,
            tensor_data_start,
            token_embd,
            output_weight,
            output_norm,
            ple_ngram_embedding,
            ple_config: ple_config.is_complete().then_some(ple_config),
            hyperparams,
            max_tensor_bytes,
            max_layer_tensor_bytes,
        })
    }

    /// Build a synthetic index from an explicit `(name, info)` list — used to boot from a P64
    /// weight container so the *entire* GGUF-based hot path (get_layer_tensors / fetch_tensor_bytes /
    /// resident upload) works unchanged. The caller passes absolute blob offsets in each
    /// `GgufTensorInfo.byte_offset` and `tensor_data_start = 0`, pointing the byte source at the
    /// P64 bytes. Format-agnostic: the hot path never learns it is reading P64.
    pub fn from_components(
        named_tensors: &[(&[u8], GgufTensorInfo)],
        hyperparams: GgufHyperparams,
        tensor_data_start: u64,
    ) -> Self {
        let mut entries = Vec::with_capacity(named_tensors.len());
        let mut max_tensor_bytes = 0usize;
        let mut max_layer_tensor_bytes = 0usize;
        for (name, info) in named_tensors {
            let mut canonical = [0u8; 128];
            let canonical_len = canonical_component_name(name, &mut canonical);
            let lookup_name = if canonical_len > 0 {
                &canonical[..canonical_len]
            } else {
                name
            };
            if let Some(tb) = crate::ggml_quants::tensor_byte_len(info) {
                max_tensor_bytes = max_tensor_bytes.max(tb);
                if is_layer_matmul_tensor_name(lookup_name) {
                    max_layer_tensor_bytes = max_layer_tensor_bytes.max(tb);
                }
            }
            entries.push((gguf_name_hash(lookup_name), *info));
        }
        let find_h = |h: u64| entries.iter().find(|(eh, _)| *eh == h).map(|(_, i)| *i);
        let token_embd = find_h(gguf_name_hash(b"token_embd.weight"));
        let output_weight = find_h(gguf_name_hash(b"output.weight"));
        let output_norm = find_h(gguf_name_hash(b"output_norm.weight"));
        let ple_ngram_embedding = find_h(gguf_name_hash(b"per_layer_token_embd.weight"))
            .or_else(|| find_h(gguf_name_hash(b"ple.ple_embedding.ngram_embedding.weight")));
        Self {
            entries,
            tensor_data_start,
            token_embd,
            output_weight,
            output_norm,
            ple_ngram_embedding,
            ple_config: None,
            hyperparams,
            max_tensor_bytes,
            max_layer_tensor_bytes,
        }
    }

    fn find(&self, name: &[u8]) -> Option<GgufTensorInfo> {
        let h = gguf_name_hash(name);
        self.entries
            .iter()
            .find(|(eh, _)| *eh == h)
            .map(|(_, i)| *i)
    }

    /// Resolve a named global tensor from the bounded metadata index.  This
    /// performs no source-payload I/O and is intended for architecture-owned
    /// global paths such as Qwen4Exp's final Hyper-Connection mixer.
    pub fn tensor_info(&self, name: &[u8]) -> Option<GgufTensorInfo> {
        self.find(name)
    }

    fn find_layer_tensor(&self, layer: u32, suffix: &[u8]) -> Option<GgufTensorInfo> {
        let mut name = [0u8; 96];
        let n = write_blk_tensor_name(layer, suffix, &mut name);
        if n == 0 {
            return None;
        }
        self.find(&name[..n])
    }

    /// Retrieve attention + FFN tensor metadata for one transformer block.
    pub fn get_layer_tensors(&self, layer_idx: u32) -> LayerTensors {
        LayerTensors {
            layer_idx,
            attn_norm: self.find_layer_tensor(layer_idx, b"attn_norm.weight"),
            attn_q: self.find_layer_tensor(layer_idx, b"attn_q.weight"),
            attn_k: self.find_layer_tensor(layer_idx, b"attn_k.weight"),
            attn_v: self.find_layer_tensor(layer_idx, b"attn_v.weight"),
            attn_q_norm: self.find_layer_tensor(layer_idx, b"attn_q_norm.weight"),
            attn_k_norm: self.find_layer_tensor(layer_idx, b"attn_k_norm.weight"),
            attn_qkv: self.find_layer_tensor(layer_idx, b"attn_qkv.weight"),
            attn_gate: self.find_layer_tensor(layer_idx, b"attn_gate.weight"),
            attn_output: self.find_layer_tensor(layer_idx, b"attn_output.weight"),
            ffn_norm: self.find_layer_tensor(layer_idx, b"ffn_norm.weight"),
            ffn_gate: self.find_layer_tensor(layer_idx, b"ffn_gate.weight"),
            ffn_up: self.find_layer_tensor(layer_idx, b"ffn_up.weight"),
            ffn_down: self.find_layer_tensor(layer_idx, b"ffn_down.weight"),
            moe_router: self.find_layer_tensor(layer_idx, b"ffn_gate_inp.weight"),
            moe_gate_exps: self.find_layer_tensor(layer_idx, b"ffn_gate_exps.weight"),
            moe_up_exps: self.find_layer_tensor(layer_idx, b"ffn_up_exps.weight"),
            moe_gate_up_exps: self.find_layer_tensor(layer_idx, b"ffn_gate_up_exps.weight"),
            moe_down_exps: self.find_layer_tensor(layer_idx, b"ffn_down_exps.weight"),
            moe_shared_gate: self.find_layer_tensor(layer_idx, b"ffn_gate_shexp.weight"),
            moe_shared_up: self.find_layer_tensor(layer_idx, b"ffn_up_shexp.weight"),
            moe_shared_down: self.find_layer_tensor(layer_idx, b"ffn_down_shexp.weight"),
            moe_shared_gate_input: self.find_layer_tensor(layer_idx, b"ffn_gate_inp_shexp.weight"),
            ssm_alpha: self.find_layer_tensor(layer_idx, b"ssm_alpha.weight"),
            ssm_beta: self.find_layer_tensor(layer_idx, b"ssm_beta.weight"),
            ssm_in: self.find_layer_tensor(layer_idx, b"ssm_in.weight"),
            ssm_conv1d: self.find_layer_tensor(layer_idx, b"ssm_conv1d.weight"),
            ssm_conv1d_bias: self.find_layer_tensor(layer_idx, b"ssm_conv1d.bias"),
            ssm_dt_bias: self.find_layer_tensor(layer_idx, b"ssm_dt.bias"),
            ssm_a: self.find_layer_tensor(layer_idx, b"ssm_a"),
            ssm_norm: self.find_layer_tensor(layer_idx, b"ssm_norm.weight"),
            ssm_out: self.find_layer_tensor(layer_idx, b"ssm_out.weight"),
            hc_attn_norm: self.find_layer_tensor(layer_idx, b"hc_attn_norm.weight"),
            hc_attn_down: self.find_layer_tensor(layer_idx, b"hc_attn_down.weight"),
            hc_attn_up: self.find_layer_tensor(layer_idx, b"hc_attn_up.weight"),
            hc_attn_inject: self.find_layer_tensor(layer_idx, b"hc_attn_inject.weight"),
            hc_ffn_norm: self.find_layer_tensor(layer_idx, b"hc_ffn_norm.weight"),
            hc_ffn_down: self.find_layer_tensor(layer_idx, b"hc_ffn_down.weight"),
            hc_ffn_up: self.find_layer_tensor(layer_idx, b"hc_ffn_up.weight"),
            hc_ffn_inject: self.find_layer_tensor(layer_idx, b"hc_ffn_inject.weight"),
            ple_key: self.find_layer_tensor(layer_idx, b"ple_key.weight"),
            ple_value: self.find_layer_tensor(layer_idx, b"ple_value.weight"),
            ple_norm_key: self.find_layer_tensor(layer_idx, b"ple_norm_key.weight"),
            ple_norm_query: self.find_layer_tensor(layer_idx, b"ple_norm_query.weight"),
            ple_norm_conv: self.find_layer_tensor(layer_idx, b"ple_norm_conv.weight"),
            ple_conv1d: self.find_layer_tensor(layer_idx, b"ple_conv1d.weight"),
            indexer_q: self.find_layer_tensor(layer_idx, b"indexer.q_proj.weight"),
            indexer_k: self.find_layer_tensor(layer_idx, b"indexer.k_proj.weight"),
            indexer_q_norm: self.find_layer_tensor(layer_idx, b"indexer.q_norm.weight"),
            indexer_k_norm: self.find_layer_tensor(layer_idx, b"indexer.k_norm.weight"),
        }
    }

    pub fn output_norm_info(&self) -> Option<&GgufTensorInfo> {
        self.output_norm.as_ref()
    }

    /// Required Qwen4Exp hash-gathered PLE n-gram table, if present.
    pub fn ple_ngram_embedding_info(&self) -> Option<&GgufTensorInfo> {
        self.ple_ngram_embedding.as_ref()
    }

    pub fn output_weight_info(&self) -> Option<&GgufTensorInfo> {
        self.output_weight.as_ref()
    }

    /// True when `output.weight` is absent and logits use tied `token_embd.weight`.
    pub fn output_weights_tied(&self) -> bool {
        self.output_weight.is_none() && self.token_embd.is_some()
    }

    /// Output projection weights: `output.weight` when present, else tied `token_embd.weight`.
    pub fn logits_projection_info(&self) -> Option<&GgufTensorInfo> {
        self.output_weight
            .as_ref()
            .or_else(|| self.token_embd.as_ref())
    }

    /// Diagnostic: byte offset + dims for tied-weight validation (MC3g).
    pub fn weight_tie_probe(&self) -> (bool, u64, u64, [u64; 4], [u64; 4]) {
        let tied = self.output_weights_tied();
        let emb = self.token_embd.as_ref();
        let out = self.output_weight.as_ref();
        let emb_off = emb.map(|t| t.byte_offset).unwrap_or(0);
        let out_off = out.map(|t| t.byte_offset).unwrap_or(emb_off);
        let emb_dims = emb.map(|t| t.dims).unwrap_or([0; 4]);
        let out_dims = out.map(|t| t.dims).unwrap_or(emb_dims);
        (tied, emb_off, out_off, emb_dims, out_dims)
    }

    /// Cached `token_embd.weight` tensor metadata.
    pub fn token_embd_info(&self) -> Option<&GgufTensorInfo> {
        self.token_embd.as_ref()
    }

    /// Return the embedding dimension (n_embd) from `token_embd.weight`, or 0 if unknown.
    pub fn emb_dim(&self) -> usize {
        self.token_embd_info()
            .map(|i| i.dims[0] as usize)
            .unwrap_or(0)
    }

    /// Vocabulary size from `token_embd.weight` shape `[n_embd, n_vocab]`.
    pub fn vocab_dim(&self) -> usize {
        self.token_embd_info()
            .map(|i| i.dims[1] as usize)
            .unwrap_or(0)
    }

    /// Dequantize one token embedding into caller-supplied `out` (zero heap in hot path).
    pub fn dequantize_token_embedding_into(
        &self,
        mmap: &[u8],
        token_id: u32,
        out: &mut [f32],
    ) -> usize {
        let info = match self.token_embd_info() {
            Some(i) => i,
            None => return 0,
        };
        let n_embd = info.dims[0] as usize;
        let n_vocab = info.dims[1] as usize;
        if n_embd == 0 || token_id as usize >= n_vocab || out.len() < n_embd {
            return 0;
        }
        let raw = match crate::ggml_quants::fetch_token_embedding(
            mmap,
            self.tensor_data_start,
            info,
            token_id,
        ) {
            Ok(s) => s,
            Err(_) => return 0,
        };
        crate::ggml_quants::dequantize_row_into(raw, info.ggml_type, n_embd, out).unwrap_or(0)
    }

    /// Dequantize one Qwen4Exp PLE n-gram table row from an already-mapped
    /// source into caller storage. This is a parity/reference path for the
    /// direct NVMe reader, not a licence to retain the full PLE table resident.
    pub fn dequantize_ple_row_into(&self, mmap: &[u8], row: u64, out: &mut [f32]) -> usize {
        let info = match self.ple_ngram_embedding_info() {
            Some(info) => info,
            None => return 0,
        };
        let row_width = info.dims[0] as usize;
        if row_width == 0 || row > u32::MAX as u64 || row >= info.dims[1] || out.len() < row_width {
            return 0;
        }
        let raw = match crate::ggml_quants::fetch_token_embedding(
            mmap,
            self.tensor_data_start,
            info,
            row as u32,
        ) {
            Ok(raw) => raw,
            Err(_) => return 0,
        };
        crate::ggml_quants::dequantize_row_into(raw, info.ggml_type, row_width, out).unwrap_or(0)
    }

    /// Slice and dequantize a token embedding (test / legacy path; allocates `Vec`).
    pub fn get_token_embedding(&self, mmap: &[u8], token_id: u32) -> Vec<f32> {
        let n_embd = self.emb_dim();
        if n_embd == 0 {
            return vec![];
        }
        let mut out = vec![0.0f32; n_embd];
        let n = self.dequantize_token_embedding_into(mmap, token_id, &mut out);
        if n == 0 {
            vec![]
        } else {
            out.truncate(n);
            out
        }
    }
}
