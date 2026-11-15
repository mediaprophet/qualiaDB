//! `TensorIndex` — the tensor-name-hash → info lookup table, built by walking
//! the GGUF tensor-info section (and the KV hyperparameter parse that precedes it),
//! plus the per-layer tensor-name helpers and token-embedding dequant access.
pub mod moe_routing;
pub mod projection;

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

/// Lookup table from tensor-name hash → `TensorInfo`, built by walking the
/// GGUF tensor-info section that immediately follows the KV metadata section.
#[derive(Clone, Debug)]
pub struct TensorIndex<H = ModelHyperparams> {
    pub entries: Vec<(u64, TensorInfo)>, // (name_hash, info)
    /// Absolute byte offset in the mmap where tensor payload data begins.
    pub tensor_data_start: u64,
    /// Cached metadata for `token_embd.weight` (embedding lookup target).
    pub token_embd: Option<TensorInfo>,
    /// Cached `output.weight` for final vocabulary projection.
    pub output_weight: Option<TensorInfo>,
    /// Cached `output_norm.weight` — final RMSNorm before vocab projection (Llama/SmolLM).
    pub output_norm: Option<TensorInfo>,
    /// Qwen4Exp PLE n-gram embedding table. This is a direct NVMe row-gather
    /// source, never a staging/GPU-resident weight.
    pub ple_ngram_embedding: Option<TensorInfo>,
    /// Exact PLE n-gram row-selection contract, when this GGUF supplies one.
    pub ple_config: Option<Qwen4ExpPleConfig>,
    pub moe_routing: moe_routing::MoeRoutingSettings,
    pub hyperparams: H,
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

/// Write `model.layers.{layer}.{suffix}` into `out`; returns total bytes written.
pub fn write_hf_layer_tensor_name(layer: u32, suffix: &[u8], out: &mut [u8]) -> usize {
    let prefix = b"model.layers.";
    if out.len() < prefix.len() + 1 + suffix.len() {
        return 0;
    }
    out[..prefix.len()].copy_from_slice(prefix);
    let mut n = prefix.len();
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
    if !name.ends_with(".weight") {
        return 0;
    }
    let component = |value: &str| name.split('.').any(|part| part == value);
    if name.contains("per_layer") || name.contains("ple.") {
        return 0;
    }
    // Qwen hybrid checkpoints use one fused `[Q;K;V]` projection.  It has no P64 role
    // equivalent to a single projection, so normalise it before the ordinary role mapper.
    if component("qkv_proj") || component("attn_qkv") {
        if let Some(layer) = tensor_layer_number(name) {
            return write_blk_tensor_name(layer, b"attn_qkv.weight", out);
        }
    }
    if component("ffn_gate_up_exps") || (component("experts") && component("gate_up_proj")) {
        if let Some(layer) = tensor_layer_number(name) {
            return write_blk_tensor_name(layer, b"ffn_gate_up_exps.weight", out);
        }
    }
    let Some(role) = crate::tensor_roles::name_to_role(name) else {
        return 0;
    };
    if role.layer == crate::p64::P64_LAYER_GLOBAL {
        let global = match role.role {
            crate::p64::P64_ROLE_TOKEN_EMBD => b"token_embd.weight".as_slice(),
            crate::p64::P64_ROLE_OUTPUT => b"output.weight".as_slice(),
            crate::p64::P64_ROLE_OUTPUT_NORM => b"output_norm.weight".as_slice(),
            _ => return 0,
        };
        if global.len() > out.len() {
            return 0;
        }
        out[..global.len()].copy_from_slice(global);
        return global.len();
    }
    let Some(suffix) = crate::tensor_roles::canonical_suffix(role.role) else {
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

mod file;
pub mod ftw_config;
mod hyperparams;
mod lookup;
mod metadata;
mod parse;
pub mod qwen_attention;
pub mod qwen_roles;
mod types;
pub use hyperparams::*;
pub use types::*;

mod compat_cursor;
use compat_cursor::gguf_name_hash as tensor_name_hash;
use compat_cursor::gguf_skip_value;
use tensor_name_hash as gguf_name_hash;

/// Metadata projection accepted by numerical consumers without cloning the directory.
pub trait HyperparamsSource: Copy + Into<ModelHyperparams> + From<ModelHyperparams> {}
impl<T: Copy + Into<ModelHyperparams> + From<ModelHyperparams>> HyperparamsSource for T {}

mod prepared;
pub use prepared::{ModelMetadata, ModelPreparationBudget, PrepareModelError, PreparedP64};

#[cfg(test)]
mod tests;

mod extension;
pub use extension::{metadata_extension_locator, MODEL_METADATA_EXTENSION_BYTES};

fn synthetic_tensor_hash(name: &[u8]) -> Option<u64> {
    if name.len() != 23 || !name.starts_with(b"tensor.") {
        return None;
    }
    let mut hash = 0u64;
    for &byte in &name[7..] {
        let digit = match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'f' => byte - b'a' + 10,
            _ => return None,
        };
        hash = (hash << 4) | u64::from(digit);
    }
    Some(hash)
}

mod encoding;
pub use encoding::WeightEncoding;

mod moe_metadata;
pub use moe_metadata::MODEL_ROUTING_EXTENSION_BYTES;

#[cfg(test)]
mod tensor_alias_tests;
