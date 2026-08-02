//! Browser binding for the independent Qualia CPU-WASM decode plan.

use js_sys::Function;
use wasm_bindgen::prelude::*;

use crate::gguf_bridge::wasm_cpu::CpuWasmEngine;

use super::WasmAsyncInferenceResult;

thread_local! {
    static CPU_ENGINE: std::cell::RefCell<Option<CpuWasmEngine>> = const { std::cell::RefCell::new(None) };
}

pub(super) fn ready() -> bool {
    CPU_ENGINE.with(|engine| engine.borrow().is_some())
}

pub(super) fn vocab_len() -> u32 {
    CPU_ENGINE.with(|engine| {
        engine
            .borrow()
            .as_ref()
            .map(CpuWasmEngine::vocab_len)
            .unwrap_or(0)
    })
}

pub(super) fn release() {
    CPU_ENGINE.with(|engine| *engine.borrow_mut() = None);
}

pub(super) async fn initialize(model: std::sync::Arc<[u8]>) -> Result<(), String> {
    initialize_with_context(
        model,
        crate::gguf_bridge::wasm_cpu::CPU_WASM_DEFAULT_CONTEXT,
    )
    .await
}

pub(super) async fn initialize_with_context(
    model: std::sync::Arc<[u8]>,
    max_context: usize,
) -> Result<(), String> {
    crate::gguf_bridge::wasm_yield::phase("Preparing Qualia CPU-WASM model…").await;
    let engine =
        CpuWasmEngine::new_with_context(model, max_context).map_err(|error| error.to_string())?;
    crate::gguf_bridge::wlog(&format!(
        "[cpu-wasm] ready: vocab={} context={} working-set={:.1} MiB",
        engine.vocab_len(),
        engine.max_context(),
        engine.working_set_bytes() as f64 / (1024.0 * 1024.0)
    ));
    CPU_ENGINE.with(|slot| *slot.borrow_mut() = Some(engine));
    crate::gguf_bridge::wasm_yield::clear_init_status();
    Ok(())
}

fn take() -> Result<CpuWasmEngine, String> {
    CPU_ENGINE
        .with(|engine| engine.borrow_mut().take())
        .ok_or_else(|| "CPU-WASM engine is not initialized".to_string())
}

fn restore(engine: CpuWasmEngine) {
    CPU_ENGINE.with(|slot| *slot.borrow_mut() = Some(engine));
}

pub(super) async fn infer(
    prompt: &str,
    graph_context: &str,
    on_token: Option<Function>,
    decode_budget: usize,
) -> Result<WasmAsyncInferenceResult, String> {
    let mut engine = take()?;
    let result = async {
        if !graph_context.is_empty() {
            crate::gguf_bridge::wlog(&format!(
                "[cpu-wasm] graph-context={:016x}",
                crate::q_hash(graph_context)
            ));
        }
        let mut context = engine.tokenizer().encode_prompt(prompt);
        if context.is_empty() {
            return Err("prompt tokenized to empty context".to_string());
        }
        if context.len() >= engine.max_context() {
            return Err(format!(
                "prompt has {} tokens; CPU-WASM context is {}",
                context.len(),
                engine.max_context()
            ));
        }
        engine.reset();
        // REVIEW(wasm-mobile-2026-08-02 F4): this correctness floor serially
        // evaluates prompt tokens on the browser thread. Move the prepared plan
        // into a Worker and add deterministic WASM-SIMD/threaded prefill without
        // coupling inference memory to the 42 MiB semantic Sentinel arena.
        for (position, &token) in context
            .iter()
            .take(context.len().saturating_sub(1))
            .enumerate()
        {
            engine
                .ingest_token(token, position)
                .map_err(|error| error.to_string())?;
            crate::gguf_bridge::wasm_yield::yield_to_browser().await;
        }

        let eos = engine.tokenizer().eos_token_id;
        let mut decoded = Vec::with_capacity(decode_budget * 8);
        let mut streamed = 0usize;
        let mut generated = 0u32;
        let mut piece = [0u8; 1024];
        let available = engine.max_context().saturating_sub(context.len());
        let budget = decode_budget.min(available);

        for step in 0..budget {
            let current = *context.last().unwrap_or(&engine.tokenizer().bos_token_id);
            let position = context.len().saturating_sub(1);
            let next = engine
                .run_token(current, position)
                .map_err(|error| error.to_string())?
                .token_id;
            if next == eos || engine.tokenizer().is_stop_token(next) {
                break;
            }
            context.push(next);
            generated += 1;
            let piece_len = engine
                .tokenizer()
                .decode_token_bytes_into(next, &mut piece)
                .ok_or_else(|| format!("token {next} exceeds decode-piece limit"))?;
            decoded.extend_from_slice(&piece[..piece_len]);
            let pending = &decoded[streamed..];
            let valid = match std::str::from_utf8(pending) {
                Ok(_) => pending.len(),
                Err(error) => error.valid_up_to(),
            };
            if valid > 0 {
                let delta = std::str::from_utf8(&pending[..valid]).expect("validated UTF-8 prefix");
                streamed += valid;
                if let Some(callback) = on_token.as_ref() {
                    let _ = callback.call1(&JsValue::UNDEFINED, &JsValue::from_str(delta));
                }
            }
            let _ = step;
            crate::gguf_bridge::wasm_yield::yield_to_browser().await;
        }

        Ok(WasmAsyncInferenceResult {
            text: String::from_utf8_lossy(&decoded).into_owned(),
            generated_tokens: generated,
        })
    }
    .await;
    restore(engine);
    result
}
