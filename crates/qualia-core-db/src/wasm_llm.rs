//! Browser-native LLM exports — Qualia GGUF + WebGPU path (not third-party llama.cpp bindings).

use js_sys::Function;
use wasm_bindgen::prelude::*;

use crate::gguf_bridge::{
    StreamingArgmaxResult, PREFILL_CHUNK_SIZE, PREFILL_CHUNK_STACK_FLOATS, VOCAB_CHUNK_ROWS,
};
use crate::gguf_sharder::GgufTokenizer;

/// Autoregressive decode budget for browser harness.
/// 32 was too short for chat replies and made truncated junk look like "garbage".
const WASM_DECODE_TOKEN_BUDGET: usize = 128;
const WASM_MAX_CONFIGURED_DECODE_TOKENS: usize = 512;
/// `0` = all transformer layers.
const WASM_LAYER_CAP: u32 = 0;
/// `0` = full vocabulary argmax sweep.
const WASM_VOCAB_CHUNK_CAP: u32 = 0;
/// Real LLM vocabs are thousands of tokens; the 256-entry byte fallback is never valid for chat.
const MIN_REAL_VOCAB: usize = 1000;

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

/// Reject the 256-byte fallback tokenizer — it "runs" but emits pure garbage on real models.
fn require_real_tokenizer(tok: &GgufTokenizer) -> Result<(), String> {
    let n = tok.vocab_len() as usize;
    if n < MIN_REAL_VOCAB {
        return Err(format!(
            "tokenizer missing or fallback-only (vocab={n}). Re-download a release P64 with Q42T section, or load GGUF so the real SentencePiece/BPE vocab is parsed. Silent byte-level decode produces garbage text."
        ));
    }
    Ok(())
}

/// Phase 2B: fully async prefill + decode via `_async` dispatchers (`map_async` + `.await`).
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct WasmAsyncInferenceResult {
    text: String,
    generated_tokens: u32,
}

async fn run_inference_async(
    prompt: &str,
    graph_context: &str,
    on_token: Option<Function>,
    decode_token_budget: usize,
) -> Result<WasmAsyncInferenceResult, String> {
    let mut engine = take_engine()?;
    let prompt_owned = prompt.to_string();
    if !graph_context.is_empty() {
        crate::gguf_bridge::wlog(&format!(
            "[inference] graph-context={:016x}",
            crate::q_hash(graph_context)
        ));
    }

    let result: Result<WasmAsyncInferenceResult, String> = async {
        // Boot purely from P64 when present: synthetic tensor index +
        // tokenizer section, no GGUF parse. (gguf_mmap holds the P64 bytes; the synthetic index
        // uses tensor_data_start=0 + absolute offsets, so the rest of the path is unchanged.)
        //
        // Fail closed: never fall back to GgufTokenizer::default() (256-byte) — that path
        // "succeeds" while streaming unreadable junk, which is what the online demo shows as garbage.
        let (tok, tensor_idx) = if engine.p64_resident.is_some() {
            let data = engine.p64_resident.clone().unwrap();
            match crate::p64_weight::P64TensorIndex::from_p64(&data) {
                Ok(qi) => {
                    let section = qi.tokenizer_bytes(&data);
                    let tok = GgufTokenizer::from_p64_section(section).ok_or_else(|| {
                        format!(
                            "P64 has no usable Q42T tokenizer section ({} bytes). Recompile GGUF→P64 with a current engine, or load the original GGUF.",
                            section.len()
                        )
                    })?;
                    require_real_tokenizer(&tok)?;
                    (tok, Some(qi.to_gguf_index()))
                }
                Err(e) => {
                    crate::gguf_bridge::wlog(&format!("[P64] container parse failed: {e}"));
                    return Err(format!(
                        "P64 container parse failed: {e}. Model bytes are not a valid P64 (or are truncated)."
                    ));
                }
            }
        } else {
            let mmap = engine
                .gguf_mmap
                .as_ref()
                .ok_or_else(|| "no resident model bytes (neither P64 nor GGUF)".to_string())?;
            // from_gguf falls back to Default on parse failure — require_real_tokenizer rejects it.
            let tok = GgufTokenizer::from_gguf(mmap);
            require_real_tokenizer(&tok)?;
            let idx = crate::gguf_sharder::GgufTensorIndex::from_gguf(mmap);
            (tok, Some(idx))
        };

        let mut ctx = tok.encode_prompt(&prompt_owned);
        if ctx.is_empty() {
            return Err("prompt tokenized to empty context".into());
        }
        let eos = tok.eos_token_id;
        let vlen = tok.vocab_len().max(1);

        let emb_dim = tensor_idx
            .as_ref()
            .map(|idx| idx.emb_dim())
            .filter(|&d| d > 0)
            .ok_or_else(|| {
                "tensor index missing embedding dim — weights did not load (cannot decode)".to_string()
            })?
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

        for step in 0..decode_token_budget {
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

            // Fail closed: never treat the hidden state as a vocab distribution.
            // That fold-over-emb_buf path emitted token ids in 0..emb_dim — pure garbage text.
            let idx = tensor_idx
                .as_ref()
                .ok_or_else(|| "tensor index missing mid-decode".to_string())?;
            let token_idx = ctx.len().saturating_sub(1) as u32;
            // Fused path: forward + output norm + argmax in one GPU submit/readback
            let argmax: Option<StreamingArgmaxResult> = engine
                .dispatch_forward_and_argmax_fused_async(
                    idx,
                    &mut emb_buf[..emb_dim],
                    emb_dim,
                    token_idx,
                    WASM_LAYER_CAP,
                    &mut chunk_logits,
                    WASM_VOCAB_CHUNK_CAP,
                )
                .await;
            let argmax = match argmax {
                Some(r) => r,
                None => {
                    // Fallback: separate forward + CPU norm + argmax
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
                    engine
                        .dispatch_output_argmax_chunked_async(
                            idx,
                            &emb_buf[..emb_dim],
                            emb_dim,
                            &mut chunk_logits,
                            WASM_VOCAB_CHUNK_CAP,
                            None,
                        )
                        .await
                        .ok_or_else(|| {
                            format!("output argmax failed at step {step} (WebGPU logits path unavailable)")
                        })?
                }
            };
            if !(argmax.max_logit > f32::NEG_INFINITY) {
                return Err(format!(
                    "output argmax produced no finite logit at step {step} (weights/dequant likely broken)"
                ));
            }
            let top_i = argmax.best_token_id as usize;

            let next = (top_i as u32) % vlen;
            // Stop before appending stop/EOS so the stream does not paint control tokens.
            if next == eos || tok.is_stop_token(next) {
                break;
            }
            out_ids.push(next);
            ctx.push(next);

            let full = tok.decode(&out_ids);
            if full.len() > streamed_len {
                let delta = full[streamed_len..].to_string();
                streamed_len = full.len();
                if let Some(callback) = on_token.as_ref() {
                    let _ = callback.call1(&JsValue::UNDEFINED, &JsValue::from_str(&delta));
                }
            }
            let _ = step;
        }

        Ok(WasmAsyncInferenceResult {
            text: tok.decode(&out_ids),
            generated_tokens: out_ids.len() as u32,
        })
    }
    .await;

    restore_engine(engine);
    result
}

/// Diagnostic: vocab size of the resident model tokenizer (0 if engine empty / parse failed).
/// Used by the online demo to show "garbage risk" before generate.
#[wasm_bindgen(js_name = getResidentTokenizerVocab)]
pub fn get_resident_tokenizer_vocab() -> u32 {
    if !engine_ready() {
        return 0;
    }
    // Peek without taking the engine.
    crate::gguf_bridge::WASM_ENGINE_INSTANCE.with(|g| {
        let borrow = g.borrow();
        let Some(engine) = borrow.as_ref() else {
            return 0;
        };
        if let Some(data) = engine.p64_resident.as_ref() {
            if let Ok(qi) = crate::p64_weight::P64TensorIndex::from_p64(data) {
                if let Some(tok) = GgufTokenizer::from_p64_section(qi.tokenizer_bytes(data)) {
                    return tok.vocab_len();
                }
            }
            return 0;
        }
        if let Some(m) = engine.gguf_mmap.as_ref() {
            return GgufTokenizer::from_gguf(m).vocab_len();
        }
        0
    })
}

/// Returns true when a GGUF or P64 model has been loaded via `initialize_webgpu_engine`.
#[wasm_bindgen(js_name = isWebgpuEngineReady)]
pub fn is_webgpu_engine_ready() -> bool {
    engine_ready()
}

/// Differential WebGPU/CPU probe for the first layer's Q projection.
///
/// This is intentionally exposed to the browser debug surface so automated
/// agents can distinguish model/package failures from quantized-kernel
/// failures without asking a user to inspect opaque generated text.
#[wasm_bindgen(js_name = verifyFirstLayerQuant)]
pub async fn verify_first_layer_quant() -> Result<JsValue, JsValue> {
    #[derive(serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    struct QuantProbe {
        role: &'static str,
        ggml_type: u32,
        rows_checked: usize,
        max_abs_error: f32,
        max_rel_error: f32,
        passed: bool,
    }

    let engine = take_engine().map_err(|e| JsValue::from_str(&e))?;
    let result: Result<QuantProbe, String> = async {
        let index = if let Some(data) = engine.p64_resident.as_ref() {
            crate::p64_weight::P64TensorIndex::from_p64(data)?.to_gguf_index()
        } else {
            let bytes = engine
                .gguf_mmap
                .as_deref()
                .ok_or_else(|| "quant probe: no resident model".to_string())?;
            crate::gguf_sharder::GgufTensorIndex::from_gguf(bytes)
        };
        let info = index
            .get_layer_tensors(0)
            .attn_q
            .ok_or_else(|| "quant probe: layer 0 has no Q projection".to_string())?;
        let (n_in, n_out) = crate::gguf_bridge::QTensorEngine::matmul_dims(&info);
        let rows = n_out.min(8);
        if n_in == 0 || n_in > 8192 || rows == 0 {
            return Err("quant probe: unsupported projection dimensions".to_string());
        }
        let bytes = engine
            .gguf_mmap
            .as_deref()
            .ok_or_else(|| "quant probe: model bytes unavailable".to_string())?;
        let raw = crate::ggml_quants::fetch_tensor_bytes(bytes, index.tensor_data_start, &info)
            .map_err(|e| format!("quant probe: tensor bytes unavailable: {e:?}"))?;
        let mut input = vec![0.0f32; n_in];
        for (i, value) in input.iter_mut().enumerate() {
            *value = ((i as f32 * 0.017).sin() + (i as f32 * 0.013).cos()) * 0.25;
        }
        let mut gpu = vec![0.0f32; rows];
        if !engine
            .dispatch_gemm_into_async(&index, &info, &input, &mut gpu, n_in, rows)
            .await
        {
            return Err("quant probe: WebGPU GEMM dispatch failed".to_string());
        }
        let mut row = vec![0.0f32; n_in];
        let mut max_abs = 0.0f32;
        let mut max_rel = 0.0f32;
        for r in 0..rows {
            crate::ggml_quants::dequant_matrix_row_into(raw, &info, r, &mut row)
                .map_err(|e| format!("quant probe: CPU dequant failed: {e:?}"))?;
            let cpu = row
                .iter()
                .zip(input.iter())
                .map(|(w, x)| w * x)
                .sum::<f32>();
            let abs = (gpu[r] - cpu).abs();
            let rel = abs / cpu.abs().max(1.0e-5);
            max_abs = max_abs.max(abs);
            max_rel = max_rel.max(rel);
        }
        Ok(QuantProbe {
            role: "blk.0.attn_q.weight",
            ggml_type: info.ggml_type,
            rows_checked: rows,
            max_abs_error: max_abs,
            max_rel_error: max_rel,
            passed: max_abs <= 0.05 || max_rel <= 0.02,
        })
    }
    .await;
    restore_engine(engine);
    let probe = result.map_err(|e| JsValue::from_str(&e))?;
    serde_wasm_bindgen::to_value(&probe).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Load a GGUF or P64 model into the resident browser WebGPU engine.
#[wasm_bindgen]
pub async fn initialize_webgpu_engine(model_data: js_sys::Uint8Array) -> Result<(), js_sys::Error> {
    let vec = model_data.to_vec();
    let arc: std::sync::Arc<[u8]> = vec.into();
    crate::gguf_bridge::initialize_webgpu_engine(arc)
        .await
        .map_err(|e| js_sys::Error::new(&e))
}

/// Release resident model weights and tear down the WebGPU engine instance.
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
    run_inference_async(&prompt, &graph_context, None, WASM_DECODE_TOKEN_BUDGET)
        .await
        .map(|result| result.text)
        .map_err(|e| JsValue::from_str(&e))
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
    run_inference_async(
        &prompt,
        &graph_context,
        Some(on_token),
        WASM_DECODE_TOKEN_BUDGET,
    )
    .await
    .map(|result| result.text)
    .map_err(|e| JsValue::from_str(&e))
}

/// Phase 2B: async WebGPU decode — yields to the browser event loop on every `map_async`.
/// Returns a JS `Promise`; use `await inferWasmAsync(...)` from module code.
#[wasm_bindgen(js_name = inferWasmAsync)]
pub async fn infer_wasm_async(prompt: String, on_token: Function) -> Result<String, JsValue> {
    run_inference_async(&prompt, "", Some(on_token), WASM_DECODE_TOKEN_BUDGET)
        .await
        .map(|result| result.text)
        .map_err(|e| JsValue::from_str(&e))
}

/// Async WebGPU decode with an explicit, bounded token budget and exact token count.
///
/// This is the benchmark-safe API: callers can compare engines at the same decode
/// budget without estimating model tokens from whitespace.
#[wasm_bindgen(js_name = inferWasmAsyncMeasured)]
pub async fn infer_wasm_async_measured(
    prompt: String,
    max_tokens: u32,
    on_token: Function,
) -> Result<JsValue, JsValue> {
    let budget = (max_tokens as usize).clamp(1, WASM_MAX_CONFIGURED_DECODE_TOKENS);
    let result = run_inference_async(&prompt, "", Some(on_token), budget)
        .await
        .map_err(|e| JsValue::from_str(&e))?;
    serde_wasm_bindgen::to_value(&result).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Compile a flat GGUF byte image into a canonical P64 LLM-weight container.
/// Run once at ingest and cache the result in OPFS.
#[wasm_bindgen(js_name = compileGgufToP64)]
pub fn compile_gguf_to_p64(
    gguf: js_sys::Uint8Array,
    page_log2: u16,
) -> Result<js_sys::Uint8Array, JsValue> {
    let bytes = gguf.to_vec();
    let out = crate::p64_weight::compile_gguf_to_p64(&bytes, page_log2)
        .map_err(|e| JsValue::from_str(&e))?;
    Ok(js_sys::Uint8Array::from(out.as_slice()))
}

/// Historical export retained for browser compatibility. Emits P64 bytes.
#[wasm_bindgen(js_name = compileGgufToQ42)]
pub fn compile_gguf_to_q42(
    gguf: js_sys::Uint8Array,
    page_log2: u16,
) -> Result<js_sys::Uint8Array, JsValue> {
    compile_gguf_to_p64(gguf, page_log2)
}

/// Current P64 container version for OPFS cache invalidation.
#[wasm_bindgen(js_name = p64FormatVersion)]
pub fn p64_format_version() -> u16 {
    crate::p64_weight::P64_VERSION
}

/// Historical export retained for browser compatibility. Returns P64_VERSION.
#[wasm_bindgen(js_name = q42FormatVersion)]
pub fn q42_format_version() -> u16 {
    p64_format_version()
}
