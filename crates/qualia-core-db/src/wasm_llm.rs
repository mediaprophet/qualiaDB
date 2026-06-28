//! Browser-native LLM exports — Qualia GGUF + WebGPU path (not third-party llama.cpp bindings).

use js_sys::Function;
use wasm_bindgen::prelude::*;

use crate::gguf_bridge::{PREFILL_CHUNK_SIZE, PREFILL_CHUNK_STACK_FLOATS, VOCAB_CHUNK_ROWS};
use crate::gguf_sharder::GgufTokenizer;

const BROWSER_AGENT_DID: &str = "did:q42:browser-llm-demo";
const BROWSER_MODEL_TAG: &str = "wasm-resident.gguf";

/// Autoregressive decode budget for browser harness (CPU path uses same cap).
const WASM_DECODE_TOKEN_BUDGET: usize = 32;
/// `0` = all transformer layers.
const WASM_LAYER_CAP: u32 = 0;
/// `0` = full vocabulary argmax sweep.
const WASM_VOCAB_CHUNK_CAP: u32 = 0;

/// Crate semver baked in at compile time.
#[wasm_bindgen(js_name = getEngineVersion)]
pub fn get_engine_version() -> String {
    crate::ENGINE_VERSION.to_string()
}

fn engine_ready() -> bool {
    crate::gguf_bridge::WASM_ENGINE_INSTANCE.with(|g| g.borrow().is_some())
}

fn restore_engine(engine: crate::gguf_bridge::QTensorEngine) {
    crate::gguf_bridge::WASM_ENGINE_INSTANCE.with(|g| *g.borrow_mut() = Some(engine));
}

fn take_engine() -> Result<crate::gguf_bridge::QTensorEngine, String> {
    crate::gguf_bridge::WASM_ENGINE_INSTANCE
        .with(|g| g.borrow_mut().take())
        .ok_or_else(|| "WebGPU engine not initialized. Call initialize_webgpu_engine first.".into())
}

fn run_inference(prompt: &str, graph_context: &str) -> Result<String, String> {
    if !engine_ready() {
        return Err("WebGPU engine not initialized. Call initialize_webgpu_engine first.".into());
    }
    let agent = crate::llm_agent::LocalLlmAgent::new(BROWSER_AGENT_DID, BROWSER_MODEL_TAG);
    let (text, _, _, _) =
        agent.infer_local_model_streaming(prompt, graph_context, None::<fn(String)>);
    Ok(text)
}

fn run_inference_streaming(
    prompt: &str,
    graph_context: &str,
    on_token: Function,
) -> Result<String, String> {
    if !engine_ready() {
        return Err("WebGPU engine not initialized. Call initialize_webgpu_engine first.".into());
    }
    let agent = crate::llm_agent::LocalLlmAgent::new(BROWSER_AGENT_DID, BROWSER_MODEL_TAG);
    let on_token = move |piece: String| {
        let _ = on_token.call1(&JsValue::UNDEFINED, &JsValue::from_str(&piece));
    };
    let (text, _, _, _) = agent.infer_local_model_streaming(prompt, graph_context, Some(on_token));
    Ok(text)
}

/// Phase 2B: fully async prefill + decode via `_async` dispatchers (`map_async` + `.await`).
async fn run_inference_async(prompt: &str, on_token: Function) -> Result<String, String> {
    let mut engine = take_engine()?;
    let prompt_owned = prompt.to_string();

    let result: Result<String, String> = async {
        // Phase 4: boot purely from the `.q42` container when present — synthetic tensor index +
        // tokenizer section, no GGUF parse. (gguf_mmap holds the .q42 bytes; the synthetic index
        // uses tensor_data_start=0 + absolute offsets, so the rest of the path is unchanged.)
        let (tok, tensor_idx) = if engine.q42_resident.is_some() {
            let data = engine.q42_resident.clone().unwrap();
            match crate::p64_weight::P64TensorIndex::from_p64(&data) {
                Ok(qi) => {
                    let tok = GgufTokenizer::from_p64_section(qi.tokenizer_bytes(&data))
                        .unwrap_or_default();
                    (tok, Some(qi.to_gguf_index()))
                }
                Err(e) => {
                    crate::gguf_bridge::wlog(&format!("[Q42] container parse failed: {e}"));
                    (GgufTokenizer::default(), None)
                }
            }
        } else {
            let tok = engine
                .gguf_mmap
                .as_ref()
                .map(|m| GgufTokenizer::from_gguf(m))
                .unwrap_or_default();
            let idx = engine
                .gguf_mmap
                .as_ref()
                .map(|m| crate::gguf_sharder::GgufTensorIndex::from_gguf(m));
            (tok, idx)
        };

        let mut ctx = tok.encode_prompt(&prompt_owned);
        let eos = tok.eos_token_id;
        let vlen = tok.vocab_len().max(1);

        let emb_dim = tensor_idx
            .as_ref()
            .map(|idx| idx.emb_dim())
            .filter(|&d| d > 0)
            .unwrap_or(4096)
            .min(8192);

        const MAX_FFN_DIM: usize = 10240;
        let mut emb_buf = [0f32; 8192];
        let mut scratch_a = [0f32; MAX_FFN_DIM];
        let mut scratch_b = [0f32; MAX_FFN_DIM];
        let mut prefill_chunk = [0f32; PREFILL_CHUNK_STACK_FLOATS];
        let mut chunk_logits = [0f32; VOCAB_CHUNK_ROWS];

        engine.reset_kv_cache();

        let prompt_len = ctx.len();
        let prefill_tokens = prompt_len.saturating_sub(1);
        if prefill_tokens > 0 {
            if let Some(idx) = tensor_idx.as_ref() {
                let chunk_cap = (PREFILL_CHUNK_STACK_FLOATS / emb_dim)
                    .min(PREFILL_CHUNK_SIZE)
                    .max(1);
                let mut pos = 0usize;
                while pos < prefill_tokens {
                    let n = (prefill_tokens - pos).min(chunk_cap);
                    let batch_elems = n * emb_dim;
                    let mmap = match engine.gguf_mmap.as_deref() {
                        Some(m) => m,
                        None => break,
                    };
                    for t in 0..n {
                        let _ = idx.dequantize_token_embedding_into(
                            mmap,
                            ctx[pos + t],
                            &mut prefill_chunk[t * emb_dim..(t + 1) * emb_dim],
                        );
                    }
                    if !engine
                        .dispatch_prefill_chunk_async(
                            idx,
                            &mut prefill_chunk[..batch_elems],
                            emb_dim,
                            n as u32,
                            pos as u32,
                            &mut scratch_a,
                            &mut scratch_b,
                            WASM_LAYER_CAP,
                            None,
                        )
                        .await
                    {
                        crate::gguf_bridge::wlog(&format!(
                            "[llm_async] PREFILL chunk FAILED pos={pos} n={n}"
                        ));
                        return Err("prefill chunk failed".into());
                    }
                    pos += n;
                }
            }
        }

        let mut out_ids: Vec<u32> = Vec::new();
        let mut streamed_len = 0usize;

        for step in 0..WASM_DECODE_TOKEN_BUDGET {
            let cur = *ctx.last().unwrap_or(&tok.bos_token_id);

            let hidden_ok = tensor_idx
                .as_ref()
                .and_then(|idx| {
                    engine.gguf_mmap.as_deref().map(|m| {
                        idx.dequantize_token_embedding_into(m, cur, &mut emb_buf[..emb_dim])
                    })
                })
                .unwrap_or(0);

            if hidden_ok == 0 {
                return Err("embedding dequant failed".into());
            }

            let (top_i, _top_v) = if let Some(idx) = tensor_idx.as_ref() {
                let token_idx = ctx.len().saturating_sub(1) as u32;
                let _layers = engine
                    .dispatch_transformer_forward_async(
                        idx,
                        &mut emb_buf[..emb_dim],
                        emb_dim,
                        &mut scratch_a,
                        &mut scratch_b,
                        token_idx,
                        WASM_LAYER_CAP,
                    )
                    .await;
                let _ = engine.apply_output_norm_inplace(idx, &mut emb_buf[..emb_dim], emb_dim);
                if let Some(argmax) = engine
                    .dispatch_output_argmax_chunked_async(
                        idx,
                        &emb_buf[..emb_dim],
                        emb_dim,
                        &mut chunk_logits,
                        WASM_VOCAB_CHUNK_CAP,
                        None,
                    )
                    .await
                {
                    if argmax.max_logit > f32::NEG_INFINITY {
                        (argmax.best_token_id as usize, argmax.max_logit)
                    } else {
                        (0usize, f32::NEG_INFINITY)
                    }
                } else {
                    emb_buf[..emb_dim].iter().enumerate().fold(
                        (0usize, f32::NEG_INFINITY),
                        |(bi, bv), (i, &v)| {
                            if v > bv {
                                (i, v)
                            } else {
                                (bi, bv)
                            }
                        },
                    )
                }
            } else {
                (0usize, 0.0)
            };

            let next = (top_i as u32) % vlen;
            out_ids.push(next);
            ctx.push(next);

            let full = tok.decode(&out_ids);
            if full.len() > streamed_len {
                let delta = full[streamed_len..].to_string();
                streamed_len = full.len();
                let _ = on_token.call1(&JsValue::UNDEFINED, &JsValue::from_str(&delta));
            }

            if next == eos {
                break;
            }
            let _ = step;
        }

        Ok(tok.decode(&out_ids))
    }
    .await;

    restore_engine(engine);
    result
}

/// Returns true when a GGUF model has been loaded via `initialize_webgpu_engine`.
#[wasm_bindgen(js_name = isWebgpuEngineReady)]
pub fn is_webgpu_engine_ready() -> bool {
    engine_ready()
}

/// Load a GGUF model into the resident browser WebGPU engine.
#[wasm_bindgen]
pub async fn initialize_webgpu_engine(gguf_data: js_sys::Uint8Array) -> Result<(), js_sys::Error> {
    let vec = gguf_data.to_vec();
    let arc: std::sync::Arc<[u8]> = vec.into();
    crate::gguf_bridge::initialize_webgpu_engine(arc)
        .await
        .map_err(|e| js_sys::Error::new(&e))
}

/// Release resident GGUF weights and tear down the WebGPU engine instance.
#[wasm_bindgen(js_name = releaseWebgpuEngine)]
pub async fn release_webgpu_engine() -> Result<(), JsValue> {
    crate::gguf_bridge::WASM_ENGINE_INSTANCE.with(|g| {
        *g.borrow_mut() = None;
    });
    Ok(())
}

/// Run autoregressive inference (non-streaming). Prompt must include any chat template tokens.
#[wasm_bindgen]
pub async fn infer_wasm(prompt: String) -> Result<String, JsValue> {
    infer_wasm_with_context(prompt, String::new()).await
}

/// Same as `infer_wasm` but accepts optional graph-context bytes for provenance hashing.
#[wasm_bindgen(js_name = inferWasmWithContext)]
pub async fn infer_wasm_with_context(
    prompt: String,
    graph_context: String,
) -> Result<String, JsValue> {
    run_inference(&prompt, &graph_context).map_err(|e| JsValue::from_str(&e))
}

/// Stream token deltas to `on_token` (UTF-8 string chunks) while decoding.
#[wasm_bindgen(js_name = inferWasmStreaming)]
pub async fn infer_wasm_streaming(prompt: String, on_token: Function) -> Result<String, JsValue> {
    infer_wasm_streaming_with_context(prompt, String::new(), on_token).await
}

/// Streaming inference with optional graph context for provenance hashing.
#[wasm_bindgen(js_name = inferWasmStreamingWithContext)]
pub async fn infer_wasm_streaming_with_context(
    prompt: String,
    graph_context: String,
    on_token: Function,
) -> Result<String, JsValue> {
    run_inference_streaming(&prompt, &graph_context, on_token).map_err(|e| JsValue::from_str(&e))
}

/// Phase 2B: async WebGPU decode — yields to the browser event loop on every `map_async`.
/// Returns a JS `Promise`; use `await inferWasmAsync(...)` from module code.
#[wasm_bindgen(js_name = inferWasmAsync)]
pub async fn infer_wasm_async(prompt: String, on_token: Function) -> Result<String, JsValue> {
    run_inference_async(&prompt, on_token)
        .await
        .map_err(|e| JsValue::from_str(&e))
}

/// Phase 4 (AOT): compile a flat GGUF byte image into a `.q42` LLM-weight container
/// (page-aligned tensor blobs + zero-parse NQuin manifest). Run once at ingest; stream the
/// result to OPFS. `page_log2 == 0` selects the 16 KB default.
#[wasm_bindgen(js_name = compileGgufToQ42)]
pub fn compile_gguf_to_q42(
    gguf: js_sys::Uint8Array,
    page_log2: u16,
) -> Result<js_sys::Uint8Array, JsValue> {
    let bytes = gguf.to_vec();
    let out = crate::p64_weight::compile_gguf_to_q42(&bytes, page_log2)
        .map_err(|e| JsValue::from_str(&e))?;
    Ok(js_sys::Uint8Array::from(out.as_slice()))
}

/// Current `.q42` weight-container format version (single source of truth for the JS cache layer,
/// so a format bump auto-invalidates any stale `.q42` cached in OPFS).
#[wasm_bindgen(js_name = q42FormatVersion)]
pub fn q42_format_version() -> u16 {
    crate::p64_weight::P64_VERSION
}
