// Embedding dispatch, topology-draft acceptance, sieve construction and the
// prefix KV cache — the free-function helpers used by the Phase-8 decode loop.
//
// NOTE ON VISIBILITY: several helpers are `pub(super)` widenings (they were
// private module-level fns in the monolith). This lets `decode.rs` call them
// across the new submodule boundary while keeping them crate-internal — the
// external public API is unchanged.

use crate::NQuin;

use super::config::{TEST_TRANSFORMER_LAYER_CAP, TEST_VOCAB_CHUNK_CAP};

// ─── Embedding dispatch helpers (native) ─────────────────────────────────────

fn pseudo_embedding_forward(
    token_id: u32,
    emb_dim: usize,
    emb_buf: &mut [f32],
    engine: &crate::gguf_bridge::QTensorEngine,
    wt: &crate::gguf_bridge::QTensor,
) -> Vec<f32> {
    for i in 0..emb_dim {
        emb_buf[i] = (token_id as f32 * (i as f32 + 1.0) * 0.001_f32).sin()
            * (1.0_f32 / (emb_dim as f32).sqrt());
    }
    engine.dispatch_fused_transformer_block(wt, &emb_buf[..emb_dim])
}

#[cfg(not(target_arch = "wasm32"))]
fn cpu_embedding_forward(
    engine: &crate::gguf_bridge::QTensorEngine,
    idx: &crate::gguf_sharder::GgufTensorIndex,
    mmap: &[u8],
    token_id: u32,
    emb_dim: usize,
    emb_buf: &mut [f32],
    wt: &crate::gguf_bridge::QTensor,
) -> Vec<f32> {
    let n = idx.dequantize_token_embedding_into(mmap, token_id, &mut emb_buf[..emb_dim]);
    if n > 0 {
        engine.dispatch_fused_transformer_block(wt, &emb_buf[..n])
    } else {
        pseudo_embedding_forward(token_id, emb_dim, emb_buf, engine, wt)
    }
}

/// Like `cpu_embedding_forward` but applies a LoRA delta to the embedding
/// vector before dispatching it through the transformer block.
///
/// The delta is computed as `B @ (A @ emb) * scaling` on the CPU.
/// If the adapter dimensions do not match `emb_dim` the call silently falls
/// back to the unmodified embedding (the base model is still correct).
fn lora_embedding_forward(
    engine: &crate::gguf_bridge::QTensorEngine,
    idx: &crate::gguf_sharder::GgufTensorIndex,
    mmap: &[u8],
    token_id: u32,
    emb_dim: usize,
    emb_buf: &mut [f32],
    wt: &crate::gguf_bridge::QTensor,
    adapter: &crate::lora::LoRAAdapter,
) -> Vec<f32> {
    let n = idx.dequantize_token_embedding_into(mmap, token_id, &mut emb_buf[..emb_dim]);
    let actual_n = if n > 0 { n } else { emb_dim };

    if n == 0 {
        // Populate emb_buf with pseudo embeddings
        for i in 0..emb_dim {
            emb_buf[i] = (token_id as f32 * (i as f32 + 1.0) * 0.001_f32).sin()
                * (1.0_f32 / (emb_dim as f32).sqrt());
        }
    }

    // Apply LoRA delta if dimensions match — silent no-op otherwise
    if adapter.meta.n_in == actual_n && adapter.meta.n_out == actual_n {
        let input_snap: Vec<f32> = emb_buf[..actual_n].to_vec();
        let _ = adapter.apply_cpu(&input_snap, &mut emb_buf[..actual_n]);
    }

    engine.dispatch_fused_transformer_block(wt, &emb_buf[..actual_n])
}

/// Decode fallback when full hidden-state projection is unavailable: real GGUF
/// embedding lookup when mmap/index exist, otherwise deterministic pseudo logits.
pub(super) fn embedding_fallback_logits(
    engine: &crate::gguf_bridge::QTensorEngine,
    tensor_idx: Option<&crate::gguf_sharder::GgufTensorIndex>,
    lora_adapter: Option<&crate::lora::LoRAAdapter>,
    token_id: u32,
    emb_dim: usize,
    emb_buf: &mut [f32],
    wt: &crate::gguf_bridge::QTensor,
) -> Vec<f32> {
    if let (Some(idx), Some(mmap)) = (tensor_idx, engine.gguf_mmap.as_deref()) {
        if let Some(adapter) = lora_adapter {
            lora_embedding_forward(engine, idx, mmap, token_id, emb_dim, emb_buf, wt, adapter)
        } else {
            #[cfg(not(target_arch = "wasm32"))]
            {
                cpu_embedding_forward(engine, idx, mmap, token_id, emb_dim, emb_buf, wt)
            }
            #[cfg(target_arch = "wasm32")]
            {
                let n =
                    idx.dequantize_token_embedding_into(mmap, token_id, &mut emb_buf[..emb_dim]);
                if n > 0 {
                    engine.dispatch_fused_transformer_block(wt, &emb_buf[..n])
                } else {
                    pseudo_embedding_forward(token_id, emb_dim, emb_buf, engine, wt)
                }
            }
        }
    } else {
        pseudo_embedding_forward(token_id, emb_dim, emb_buf, engine, wt)
    }
}

fn push_decode_stream_delta(
    tok: &crate::gguf_sharder::GgufTokenizer,
    out_ids: &[u32],
    streamed_len: &mut usize,
    stream_tx: Option<&std::sync::mpsc::SyncSender<String>>,
) {
    let Some(tx) = stream_tx else {
        return;
    };
    let full = tok.decode(out_ids);
    if full.len() <= *streamed_len {
        return;
    }
    let delta = full[*streamed_len..].to_string();
    *streamed_len = full.len();
    let _ = tx.send(delta);
}

/// Load sibling canonical `.q42` metadata (if present) and merge its stop-token set into `tok`.
/// Also tries preferring a sibling `.p64` path's helper when `model_path` is a GGUF
/// that has already been converted beside it.
pub(super) fn apply_model_helper_stops(
    model_path: &str,
    tok: &mut crate::gguf_sharder::GgufTokenizer,
) {
    let path = std::path::Path::new(model_path);
    // Direct: path is already .p64 (or any path with a sibling helper).
    if let Ok(Some(h)) = crate::model_helper::ModelHelper::load_beside_p64(path) {
        h.apply_stops_to_tokenizer(tok);
        return;
    }
    // Prefer converted sibling: foo.gguf → foo.p64 + foo.q42
    if path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("gguf"))
        .unwrap_or(false)
    {
        let p64 = path.with_extension("p64");
        if let Ok(Some(h)) = crate::model_helper::ModelHelper::load_beside_p64(&p64) {
            h.apply_stops_to_tokenizer(tok);
        }
    }
}

/// Outcome of one topology-draft accept attempt (B3.1d).
pub(super) enum TopologyDraftStep {
    NoDraft,
    Denied,
    AcceptedFull,
    Fallthrough,
    Stop {
        sieve_failed: bool,
        semantic_quin: Option<NQuin>,
    },
}

pub(super) fn drain_tensor_context_inject() {
    while let Some(inject) = crate::compute_universe::pop_tensor_context() {
        if let Some(tensor) = crate::tensor::resident_substrate::global_resident_substrate()
            .tensor_at(inject.tensor_index)
        {
            crate::compute_universe::publish_query_tensor(tensor, inject.subject_hash);
        }
    }
}

pub(super) fn try_accept_topology_draft(
    engine: &mut crate::gguf_bridge::QTensorEngine,
    tensor_idx: Option<&crate::gguf_sharder::GgufTensorIndex>,
    draft_mapper: &crate::topology_draft::TopologyDraftMapper<'_>,
    ctx: &mut Vec<u32>,
    emb_dim: usize,
    emb_buf: &mut [f32],
    scratch_a: &mut [f32],
    scratch_b: &mut [f32],
    out_ids: &mut Vec<u32>,
    sieve: &mut Option<crate::neuro_symbolic_sieve::NeuroSymbolicSieve>,
    prov_hash: u64,
    eos: u32,
    tok: &crate::gguf_sharder::GgufTokenizer,
    streamed_len: &mut usize,
    stream_tx: Option<&std::sync::mpsc::SyncSender<String>>,
    mut on_token: Option<&mut dyn FnMut(String)>,
) -> TopologyDraftStep {
    let idx = match tensor_idx {
        Some(i) => i,
        None => return TopologyDraftStep::NoDraft,
    };
    let draft = match crate::compute_universe::pop_topology_draft() {
        Some(d) => d,
        None => return TopologyDraftStep::NoDraft,
    };

    let mut mapped = draft;
    for i in 0..mapped.draft_len as usize {
        mapped.draft_ids[i] = draft_mapper.concept_to_token_id(mapped.concept_hashes[i]);
    }

    if !crate::compute_universe::sentinel_allows_topology_draft(&mapped) {
        return TopologyDraftStep::Denied;
    }

    let accepted = engine.verify_topology_draft_batch(
        idx,
        ctx,
        &mapped,
        emb_dim,
        &mut emb_buf[..emb_dim],
        scratch_a,
        scratch_b,
        TEST_TRANSFORMER_LAYER_CAP,
        TEST_VOCAB_CHUNK_CAP,
    );
    crate::gpu_context::record_draft_acceptance(accepted, mapped.draft_len as u32);

    if accepted == 0 {
        return TopologyDraftStep::Fallthrough;
    }

    let mut sieve_failed = false;
    let mut semantic_quin = None;
    for i in 0..accepted as usize {
        let id = mapped.draft_ids[i];
        out_ids.push(id);
        if let Some(ref mut s) = sieve {
            if s.apply_token(id).is_err() {
                sieve_failed = true;
                break;
            }
            if s.is_complete() {
                semantic_quin = Some(s.assemble_quin(prov_hash));
                break;
            }
        } else if let Some(tx) = stream_tx {
            push_decode_stream_delta(tok, out_ids, streamed_len, Some(tx));
        } else if let Some(ref mut cb) = on_token {
            let full = tok.decode(out_ids);
            if full.len() > *streamed_len {
                let delta = full[*streamed_len..].to_string();
                *streamed_len = full.len();
                cb(delta);
            }
        }
        if tok.is_stop_token(id) || *ctx.last().unwrap_or(&eos) == eos {
            break;
        }
    }

    if sieve_failed || semantic_quin.is_some() {
        return TopologyDraftStep::Stop {
            sieve_failed,
            semantic_quin,
        };
    }
    if accepted == mapped.draft_len as u32 {
        TopologyDraftStep::AcceptedFull
    } else {
        TopologyDraftStep::Fallthrough
    }
}

pub(super) fn build_sieve(
    tok: &crate::gguf_sharder::GgufTokenizer,
    spec: Option<&crate::neuro_symbolic_sieve::SieveLexSpec>,
    lex_path: Option<&str>,
) -> Option<crate::neuro_symbolic_sieve::NeuroSymbolicSieve> {
    let spec = spec?;
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(path) = lex_path {
        let p = std::path::Path::new(path);
        if crate::q42_volume::is_unified_volume(p).unwrap_or(false) {
            if let Ok(vol) = crate::q42_volume::Q42Volume::open(p) {
                if let Ok(view) = vol.lex_view() {
                    let s = crate::neuro_symbolic_sieve::NeuroSymbolicSieve::from_lex_and_tokenizer(
                        &view, tok, spec,
                    );
                    if s.masks_ready() {
                        return Some(s);
                    }
                }
            }
        } else if let Ok(lex_file) = crate::q42_lex::Q42LexFile::open(p) {
            let s = crate::neuro_symbolic_sieve::NeuroSymbolicSieve::from_lex_and_tokenizer(
                &lex_file.view(),
                tok,
                spec,
            );
            if s.masks_ready() {
                return Some(s);
            }
        }
    }
    let s = crate::neuro_symbolic_sieve::NeuroSymbolicSieve::from_gguf_tokenizer(tok);
    if s.masks_ready() {
        Some(s)
    } else {
        None
    }
}

static PREFIX_CACHE: std::sync::OnceLock<
    std::sync::Mutex<std::collections::HashMap<u64, Box<[f32]>>>,
> = std::sync::OnceLock::new();

pub(super) fn get_prefix_cache(
) -> &'static std::sync::Mutex<std::collections::HashMap<u64, Box<[f32]>>> {
    PREFIX_CACHE.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}
