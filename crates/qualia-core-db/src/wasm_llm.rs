//! Browser-native LLM exports — Qualia GGUF + WebGPU path (not third-party llama.cpp bindings).

use js_sys::Function;
use wasm_bindgen::prelude::*;

use crate::llm_agent::LocalLlmAgent;

const BROWSER_AGENT_DID: &str = "did:q42:browser-llm-demo";
const BROWSER_MODEL_TAG: &str = "wasm-resident.gguf";

/// Crate semver baked in at compile time.
#[wasm_bindgen(js_name = getEngineVersion)]
pub fn get_engine_version() -> String {
    crate::ENGINE_VERSION.to_string()
}

fn engine_ready() -> bool {
    crate::gguf_bridge::WASM_ENGINE_INSTANCE.with(|g| g.borrow().is_some())
}

fn run_inference(
    prompt: &str,
    graph_context: &str,
) -> Result<String, String> {
    if !engine_ready() {
        return Err(
            "WebGPU engine not initialized. Call initialize_webgpu_engine first.".into(),
        );
    }
    let agent = LocalLlmAgent::new(BROWSER_AGENT_DID, BROWSER_MODEL_TAG);
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
        return Err(
            "WebGPU engine not initialized. Call initialize_webgpu_engine first.".into(),
        );
    }
    let agent = LocalLlmAgent::new(BROWSER_AGENT_DID, BROWSER_MODEL_TAG);
    let on_token = move |piece: String| {
        let _ = on_token.call1(&JsValue::UNDEFINED, &JsValue::from_str(&piece));
    };
    let (text, _, _, _) =
        agent.infer_local_model_streaming(prompt, graph_context, Some(on_token));
    Ok(text)
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