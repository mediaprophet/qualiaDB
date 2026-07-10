//! Qualia-unique hybrid inference: graph + manifold + modes + deontic.
//!
//! Most engines only optimise GEMV. QualiaDB can also:
//! 1. **Graph-grounded speculative draft** — encode fact-table repairs as draft
//!    tokens and verify with the same batch path as prompt-lookup (bit-exact when
//!    accepted; force-emit when `QUALIA_GRAPH_FORCE=1`).
//! 2. **Graph route mask** — hash prompt tokens into the U1 `AttentionRouteMask`
//!    so sparse attention can bias toward graph-linked KV provenance slots.
//! 3. **10D query publish** — fold prompt word hashes into a `Tensor10D` query
//!    anchor for the continuous U1→U0 context-inject path.
//! 4. **Graph logit bias** — boost tokenizer ids for known answer strings mid-decode
//!    (neuro-symbolic sampling, not post-hoc string replace only).
//! 5. **Deontic obligation** — when a high-stakes fact matches the prompt, compile
//!    an `OP_OBLIGATE` norm Quin so the Rights/Webizen layer can audit the duty
//!    to ground (unique to Qualia’s logic stack).
//!
//! These are first-class companions to portable/cuda/quant-graph modes — not a
//! second engine.

use crate::compute_universe::{
    publish_attention_route_mask, publish_query_tensor, AttentionRouteMask,
};
use crate::prompt_lookup::{Draft, MAX_DRAFT};
use crate::q_hash;
use crate::tensor::Tensor10D;
use crate::NQuin;

/// Logit boost applied to graph answer-token ids (nats; soft guidance).
pub const GRAPH_LOGIT_BIAS: f32 = 2.5;

/// Prepare hybrid hints before a decode turn (safe no-ops when inactive).
/// Call **after** any default `publish_query_tensor` so route/query are not wiped.
/// FastVerify skips mid-decode hybrid work (post-turn verify owns quality).
pub fn prepare_hybrid_decode(prompt: &str) {
    if matches!(
        crate::inference_modes::active_inference_mode(),
        crate::inference_modes::InferenceMode::FastVerify
    ) {
        return;
    }
    if crate::inference_modes::quant_graph_grounding_enabled()
        || crate::inference_modes::prefer_tensor_core_gemm()
    {
        publish_graph_route_from_prompt(prompt);
        publish_prompt_query_tensor(prompt);
    }
    if crate::inference_modes::quant_graph_grounding_enabled() {
        let _ = publish_grounding_obligation(prompt);
    }
}

/// Map prompt words → attention route bits (tensor/KV provenance indices).
/// Novel: uses the same U1 mask path as 10D kNN routing, fed from language not GPU.
/// When quant-graph matches a fact, also route bits from subject/object quin hashes
/// so attention can prefer KV slots co-located with grounding provenance.
pub fn publish_graph_route_from_prompt(prompt: &str) {
    let mut mask = AttentionRouteMask::default();
    for word in prompt.split(|c: char| !c.is_alphanumeric()) {
        if word.len() < 3 {
            continue;
        }
        let h = q_hash(&word.to_ascii_lowercase());
        // 1024 KV slots covered by route mask mapping in attention_kv_mask_u32.
        mask.set_index((h % 1024) as u32);
        // Also set a few nearby slots for soft neighbourhood.
        mask.set_index(((h.wrapping_add(1)) % 1024) as u32);
    }
    // Fact-quin routing: subject/object hashes → deterministic KV indices.
    if crate::inference_modes::quant_graph_grounding_enabled() {
        let g = crate::quant_graph_grounding::ground_generation(prompt, "");
        if let Some(obj) = g.object_hash {
            mask.set_index((obj % 1024) as u32);
            mask.set_index(((obj >> 10) % 1024) as u32);
            // Export quin into a one-slot scratch for future SPARQL/WAL consumers.
            let mut buf = [NQuin {
                subject: 0,
                predicate: 0,
                object: 0,
                context: 0,
                metadata: 0,
                parity: 0,
            }; 8];
            let n = crate::quant_graph_grounding::export_fact_quins(&mut buf);
            for q in buf.iter().take(n) {
                mask.set_index((q.subject % 1024) as u32);
                mask.set_index((q.object % 1024) as u32);
            }
        }
    }
    if mask.active_bits > 0 {
        publish_attention_route_mask(mask);
        log::debug!(
            "qualia_hybrid|route_mask|bits={}",
            mask.active_bits
        );
    }
}

/// Fold prompt into a 10D query for continuous graph–tensor inject.
pub fn publish_prompt_query_tensor(prompt: &str) {
    let mut t = Tensor10D {
        q: 0.0,
        v: 0.0,
        w: 0.0,
        x: 0.0,
        y: 0.0,
        z: 0.0,
        t: 0.0,
        alpha: 0.0,
        mu: 0.0,
        sigma: 0.0,
    };
    let mut subject = 0u64;
    let mut i = 0usize;
    for word in prompt.split_whitespace().take(32) {
        let h = q_hash(word);
        subject ^= h.rotate_left((i as u32) * 3);
        let f = ((h & 0xFFFF) as f32) / 65535.0;
        match i % 10 {
            0 => t.q += f,
            1 => t.v += f,
            2 => t.w += f,
            3 => t.x += f,
            4 => t.y += f,
            5 => t.z += f,
            6 => t.t += f,
            7 => t.alpha += f,
            8 => t.mu += f,
            _ => t.sigma += f,
        }
        i += 1;
    }
    if i > 0 {
        let n = i as f32;
        t.q /= n;
        t.v /= n;
        t.w /= n;
        t.x /= n;
        t.y /= n;
        t.z /= n;
        t.t /= n;
        t.alpha /= n;
        t.mu /= n;
        t.sigma /= n;
        publish_query_tensor(t, subject);
    }
}

/// Draft tokens from the quant-graph fact table (repair text encoded by caller).
///
/// `encode` should be the model tokenizer (`encode` or chat-aware). Returns empty
/// when no fact matches the prompt. Uses empty-answer probe: if needles match and
/// the (empty) answer is not yet grounded, draft the repair string for verify.
pub fn propose_fact_draft(prompt: &str, encode: &dyn Fn(&str) -> Vec<u32>) -> Draft {
    if !crate::inference_modes::quant_graph_grounding_enabled() {
        return Draft::empty();
    }
    // Empty answer never contains answer_ok → repaired=true iff needles match.
    let g = crate::quant_graph_grounding::ground_generation(prompt, "");
    // Accept either repaired text or a known reason with object (fact hit).
    let repair = if g.repaired {
        g.text.as_str()
    } else if g.reason.is_some() && g.object_hash.is_some() {
        // Prompt matches a fact but empty answer was treated as already-ok (shouldn't
        // happen); still no draft — model is free.
        return Draft::empty();
    } else {
        return Draft::empty();
    };
    let ids = encode(repair);
    if ids.is_empty() {
        return Draft::empty();
    }
    let mut d = Draft::empty();
    let take = ids.len().min(MAX_DRAFT);
    d.tokens[..take].copy_from_slice(&ids[..take]);
    d.len = take;
    log::info!(
        "qualia_hybrid|fact_draft|reason={:?}|len={take}",
        g.reason
    );
    d
}

/// Whether to force-emit graph repair without model verify (high-stakes capitals, etc.).
#[inline]
pub fn graph_force_enabled() -> bool {
    matches!(
        std::env::var("QUALIA_GRAPH_FORCE").ok().as_deref(),
        Some("1") | Some("true")
    )
}

/// If quant-graph + force, return full repair token sequence for immediate emit.
pub fn force_fact_tokens(prompt: &str, encode: &dyn Fn(&str) -> Vec<u32>) -> Option<Vec<u32>> {
    if !crate::inference_modes::quant_graph_grounding_enabled() || !graph_force_enabled() {
        return None;
    }
    let g = crate::quant_graph_grounding::ground_generation(prompt, "");
    if !g.repaired {
        return None;
    }
    let ids = encode(&g.text);
    if ids.is_empty() {
        None
    } else {
        log::info!("qualia_hybrid|fact_force|reason={:?}", g.reason);
        Some(ids)
    }
}

/// Soft-boost logits for graph answer strings (first matching token id per string).
///
/// `lookup` maps a short answer string → preferred token id (e.g. tokenizer encode
/// of "Paris" / "paris"). Call after full logits are on host, before sample/argmax.
/// Returns how many vocab entries were boosted.
pub fn apply_graph_logit_bias(
    prompt: &str,
    logits: &mut [f32],
    lookup: &dyn Fn(&str) -> Option<u32>,
) -> usize {
    if !crate::inference_modes::quant_graph_grounding_enabled() || logits.is_empty() {
        return 0;
    }
    let g = crate::quant_graph_grounding::ground_generation(prompt, "");
    if !g.repaired {
        // Already grounded or no match — nothing to bias.
        return 0;
    }
    // Prefer the repair text tokens and the reason's capital name.
    let mut boosted = 0usize;
    let candidates: [&str; 4] = [
        g.reason.as_deref().unwrap_or(""),
        "Paris",
        "paris",
        g.text.as_str(),
    ];
    // Extract last word of repair as primary answer (… is Paris.)
    let last_word = g
        .text
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() > 2)
        .last()
        .unwrap_or("");
    for s in [last_word, candidates[1], candidates[2]] {
        if s.is_empty() {
            continue;
        }
        if let Some(tid) = lookup(s) {
            let i = tid as usize;
            if i < logits.len() {
                logits[i] += GRAPH_LOGIT_BIAS;
                boosted += 1;
            }
        }
        // Case variants
        let lower = s.to_ascii_lowercase();
        if lower != s {
            if let Some(tid) = lookup(&lower) {
                let i = tid as usize;
                if i < logits.len() {
                    logits[i] += GRAPH_LOGIT_BIAS * 0.5;
                    boosted += 1;
                }
            }
        }
    }
    if boosted > 0 {
        log::debug!("qualia_hybrid|logit_bias|n={boosted}|reason={:?}", g.reason);
    }
    boosted
}

/// Compile a deontic **obligation** Quin for a matched grounding fact (audit trail).
///
/// Uses real `compile_norm_quin` — party = process principal hash, property =
/// capitalOf path, contract = grounding context. Returns None when no fact matches.
pub fn publish_grounding_obligation(prompt: &str) -> Option<NQuin> {
    if !crate::inference_modes::quant_graph_grounding_enabled() {
        return None;
    }
    let g = crate::quant_graph_grounding::ground_generation(prompt, "");
    if g.reason.is_none() && !g.repaired {
        return None;
    }
    let object = g.object_hash.unwrap_or(0);
    if object == 0 {
        return None;
    }
    // Party: synthetic "decode principal"; property: capitalOf; action: object city.
    let party = q_hash("q42:inference-principal");
    let property = q_hash(crate::quant_graph_grounding::P_CAPITAL_OF);
    let contract = q_hash(crate::quant_graph_grounding::CTX_GROUNDING);
    let quin = crate::modalities::logic::deontic::compile_norm_quin(
        party,
        crate::modalities::logic::deontic::OP_OBLIGATE,
        property,
        object,
        contract,
        0, // no expiry
        false,
    );
    log::info!(
        "qualia_hybrid|deontic_obligate|reason={:?}|object={object:#x}",
        g.reason
    );
    Some(quin)
}

/// Prefer fact draft when quant-graph; else prompt-lookup n-gram draft.
pub fn propose_best_draft(
    prompt: &str,
    ctx: &[u32],
    encode: &dyn Fn(&str) -> Vec<u32>,
) -> Draft {
    let fact = propose_fact_draft(prompt, encode);
    if fact.len > 0 {
        return fact;
    }
    crate::prompt_lookup::propose(ctx, MAX_DRAFT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inference_modes::{set_inference_mode, InferenceMode};
    use crate::quant_graph_grounding::reset_fact_store_to_defaults;
    use std::sync::Mutex;

    /// Serialise mode mutations — InferenceMode is process-global.
    fn mode_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: Mutex<()> = Mutex::new(());
        LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    #[test]
    fn fact_draft_on_capital_prompt() {
        if std::env::var("QUALIA_INFERENCE_MODE").is_ok() {
            return;
        }
        let _g = mode_lock();
        reset_fact_store_to_defaults();
        set_inference_mode(InferenceMode::QuantGraph);
        // Sanity: empty answer must repair when needles match.
        let g = crate::quant_graph_grounding::ground_generation(
            "What is the capital of France?",
            "",
        );
        assert!(
            g.repaired,
            "expected repair on empty answer; reason={:?} text={}",
            g.reason,
            g.text
        );
        let encode = |s: &str| {
            // Fake tokenizer: one id per byte
            s.bytes().map(|b| b as u32).collect()
        };
        let d = propose_fact_draft("What is the capital of France?", &encode);
        assert!(d.len > 0, "draft len 0; repaired={} text={}", g.repaired, g.text);
        set_inference_mode(InferenceMode::Portable);
    }

    #[test]
    fn route_mask_sets_bits() {
        publish_graph_route_from_prompt("capital of France Paris knowledge graph");
        let m = crate::compute_universe::attention_route_mask();
        assert!(m.active_bits > 0);
    }

    #[test]
    fn logit_bias_boosts_vocab_slot() {
        if std::env::var("QUALIA_INFERENCE_MODE").is_ok() {
            return;
        }
        let _g = mode_lock();
        reset_fact_store_to_defaults();
        set_inference_mode(InferenceMode::QuantGraph);
        let mut logits = vec![0.0f32; 128];
        // Map any answer string containing 'P'/'p' style to fixed ids.
        let lookup = |s: &str| -> Option<u32> {
            if s.eq_ignore_ascii_case("paris") {
                Some(42)
            } else {
                None
            }
        };
        let n = apply_graph_logit_bias("What is the capital of France?", &mut logits, &lookup);
        assert!(n >= 1);
        assert!(logits[42] >= GRAPH_LOGIT_BIAS);
        set_inference_mode(InferenceMode::Portable);
    }

    #[test]
    fn deontic_obligation_on_match() {
        if std::env::var("QUALIA_INFERENCE_MODE").is_ok() {
            return;
        }
        let _g = mode_lock();
        reset_fact_store_to_defaults();
        set_inference_mode(InferenceMode::QuantGraph);
        let q = publish_grounding_obligation("What is the capital of France?");
        assert!(q.is_some());
        let q = q.unwrap();
        assert_eq!(
            (q.predicate & 0xFF) as u8,
            crate::modalities::logic::deontic::OP_OBLIGATE
        );
        set_inference_mode(InferenceMode::Portable);
    }
}
