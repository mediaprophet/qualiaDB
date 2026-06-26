//! Task #12 / STELLAR §A — **tensor-name → engine GEMM-role** mapping + the ternary policy.
//!
//! The transcoder (`q42_weight`) needs to know *what* each tensor is to (a) populate the Q42W
//! manifest with real engine roles (so a container boots without a GGUF re-parse) and (b) apply the
//! §A compression *policy*: **ternary the FFN projections, keep attention + norms + embeddings at
//! higher fidelity** (ternary norms/attention would wreck coherence).
//!
//! Two naming conventions are recognised:
//! * **GGUF / llama.cpp** — `blk.{L}.attn_q.weight`, `blk.{L}.ffn_gate.weight`, `token_embd.weight`, …
//! * **Hugging Face safetensor** — `model.layers.{L}.self_attn.q_proj.weight`,
//!   `model.layers.{L}.mlp.gate_proj.weight`, `model.embed_tokens.weight`, `lm_head.weight`, …

use crate::q42_weight::{
    Q42_LAYER_GLOBAL, Q42_ROLE_ATTN_K, Q42_ROLE_ATTN_NORM, Q42_ROLE_ATTN_OUTPUT, Q42_ROLE_ATTN_Q,
    Q42_ROLE_ATTN_V, Q42_ROLE_FFN_DOWN, Q42_ROLE_FFN_GATE, Q42_ROLE_FFN_NORM, Q42_ROLE_FFN_UP,
    Q42_ROLE_OUTPUT, Q42_ROLE_OUTPUT_NORM, Q42_ROLE_TOKEN_EMBD,
};

/// A resolved tensor identity: an engine role + its layer (`Q42_LAYER_GLOBAL` for non-layer tensors).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TensorRole {
    pub role: u16,
    pub layer: u16,
}

/// Extract the layer index from a `blk.{N}.` (GGUF) or `…layers.{N}.` (HF) tensor name.
fn extract_layer(name: &str) -> Option<u16> {
    for marker in ["blk.", "layers."] {
        if let Some(pos) = name.find(marker) {
            let rest = &name[pos + marker.len()..];
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(l) = digits.parse::<u16>() {
                return Some(l);
            }
        }
    }
    None
}

/// Map a tensor name (GGUF or HF convention) to its engine role + layer, or `None` if unrecognised.
pub fn name_to_role(name: &str) -> Option<TensorRole> {
    // Global (non-layer) tensors first — and `output_norm` BEFORE the bare `output` (it contains it).
    if name.contains("output_norm") || name == "model.norm.weight" {
        return Some(TensorRole { role: Q42_ROLE_OUTPUT_NORM, layer: Q42_LAYER_GLOBAL });
    }
    if name.contains("token_embd") || name.contains("embed_tokens") {
        return Some(TensorRole { role: Q42_ROLE_TOKEN_EMBD, layer: Q42_LAYER_GLOBAL });
    }
    if name.contains("lm_head") || name == "output.weight" {
        return Some(TensorRole { role: Q42_ROLE_OUTPUT, layer: Q42_LAYER_GLOBAL });
    }

    // Per-layer tensors require a layer index.
    let layer = extract_layer(name)?;
    let role = if name.contains("attn_q") || name.contains("q_proj") {
        Q42_ROLE_ATTN_Q
    } else if name.contains("attn_k") || name.contains("k_proj") {
        Q42_ROLE_ATTN_K
    } else if name.contains("attn_v") || name.contains("v_proj") {
        Q42_ROLE_ATTN_V
    } else if name.contains("attn_output") || name.contains("o_proj") {
        Q42_ROLE_ATTN_OUTPUT
    } else if name.contains("ffn_gate") || name.contains("gate_proj") {
        Q42_ROLE_FFN_GATE
    } else if name.contains("ffn_up") || name.contains("up_proj") {
        Q42_ROLE_FFN_UP
    } else if name.contains("ffn_down") || name.contains("down_proj") {
        Q42_ROLE_FFN_DOWN
    } else if name.contains("attn_norm") || name.contains("input_layernorm") {
        Q42_ROLE_ATTN_NORM
    } else if name.contains("ffn_norm") || name.contains("post_attention_layernorm") {
        Q42_ROLE_FFN_NORM
    } else {
        return None;
    };
    Some(TensorRole { role, layer })
}

/// The §A ternary **policy**: only the FFN projection weights (`gate` / `up` / `down`) are
/// eligible for BitNet-1.58b ternary packing. Attention projections, norms, and embeddings stay at
/// higher fidelity — ternarising them destroys coherence.
pub fn ternary_eligible(role: u16) -> bool {
    matches!(role, Q42_ROLE_FFN_GATE | Q42_ROLE_FFN_UP | Q42_ROLE_FFN_DOWN)
}

/// True iff this tensor name resolves to an FFN projection (so it is ternary-eligible). A name we
/// cannot classify is **not** ternarised (fail safe to high fidelity).
pub fn name_is_ternary_eligible(name: &str) -> bool {
    name_to_role(name).map(|r| ternary_eligible(r.role)).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gguf_names_map_to_roles() {
        assert_eq!(name_to_role("blk.3.attn_q.weight"), Some(TensorRole { role: Q42_ROLE_ATTN_Q, layer: 3 }));
        assert_eq!(name_to_role("blk.0.ffn_gate.weight"), Some(TensorRole { role: Q42_ROLE_FFN_GATE, layer: 0 }));
        assert_eq!(name_to_role("blk.11.ffn_down.weight"), Some(TensorRole { role: Q42_ROLE_FFN_DOWN, layer: 11 }));
        assert_eq!(name_to_role("blk.2.attn_norm.weight"), Some(TensorRole { role: Q42_ROLE_ATTN_NORM, layer: 2 }));
        assert_eq!(name_to_role("token_embd.weight"), Some(TensorRole { role: Q42_ROLE_TOKEN_EMBD, layer: Q42_LAYER_GLOBAL }));
        assert_eq!(name_to_role("output.weight"), Some(TensorRole { role: Q42_ROLE_OUTPUT, layer: Q42_LAYER_GLOBAL }));
        assert_eq!(name_to_role("output_norm.weight"), Some(TensorRole { role: Q42_ROLE_OUTPUT_NORM, layer: Q42_LAYER_GLOBAL }));
    }

    #[test]
    fn hf_names_map_to_roles() {
        assert_eq!(name_to_role("model.layers.5.self_attn.q_proj.weight"), Some(TensorRole { role: Q42_ROLE_ATTN_Q, layer: 5 }));
        assert_eq!(name_to_role("model.layers.5.self_attn.o_proj.weight"), Some(TensorRole { role: Q42_ROLE_ATTN_OUTPUT, layer: 5 }));
        assert_eq!(name_to_role("model.layers.7.mlp.gate_proj.weight"), Some(TensorRole { role: Q42_ROLE_FFN_GATE, layer: 7 }));
        assert_eq!(name_to_role("model.layers.7.mlp.down_proj.weight"), Some(TensorRole { role: Q42_ROLE_FFN_DOWN, layer: 7 }));
        assert_eq!(name_to_role("model.layers.7.input_layernorm.weight"), Some(TensorRole { role: Q42_ROLE_ATTN_NORM, layer: 7 }));
        assert_eq!(name_to_role("model.layers.7.post_attention_layernorm.weight"), Some(TensorRole { role: Q42_ROLE_FFN_NORM, layer: 7 }));
        assert_eq!(name_to_role("model.embed_tokens.weight"), Some(TensorRole { role: Q42_ROLE_TOKEN_EMBD, layer: Q42_LAYER_GLOBAL }));
        assert_eq!(name_to_role("lm_head.weight"), Some(TensorRole { role: Q42_ROLE_OUTPUT, layer: Q42_LAYER_GLOBAL }));
        assert_eq!(name_to_role("model.norm.weight"), Some(TensorRole { role: Q42_ROLE_OUTPUT_NORM, layer: Q42_LAYER_GLOBAL }));
    }

    #[test]
    fn unknown_names_are_none() {
        assert_eq!(name_to_role("some.random.tensor"), None);
        assert_eq!(name_to_role("blk.0.rotary_emb.inv_freq"), None);
    }

    #[test]
    fn ternary_policy_is_ffn_only() {
        // FFN → eligible
        assert!(name_is_ternary_eligible("blk.0.ffn_gate.weight"));
        assert!(name_is_ternary_eligible("model.layers.3.mlp.up_proj.weight"));
        assert!(name_is_ternary_eligible("blk.9.ffn_down.weight"));
        // attention / norms / embeddings → NOT eligible (kept high-fidelity)
        assert!(!name_is_ternary_eligible("blk.0.attn_q.weight"));
        assert!(!name_is_ternary_eligible("model.layers.3.self_attn.o_proj.weight"));
        assert!(!name_is_ternary_eligible("blk.0.attn_norm.weight"));
        assert!(!name_is_ternary_eligible("token_embd.weight"));
        assert!(!name_is_ternary_eligible("output.weight"));
        // unknown → fail safe to high fidelity
        assert!(!name_is_ternary_eligible("mystery.weight"));
    }
}
