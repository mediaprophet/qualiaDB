//! Single-owner Tauri FFI for the studio crate.
//!
//! Call sites use `invoke` / `listen` (re-exported) and must not declare
//! their own extern blocks. Uses dynamic dispatch via `window.__TAURI__`
//! to ensure zero duplicate wasm-bindgen describe symbols and 100% reliable
//! adapter-free linking across dual lib/bin compilation under Dioxus CLI.

#![cfg(target_arch = "wasm32")]

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

pub async fn invoke(
    cmd: &str,
    args: JsValue,
) -> Result<JsValue, JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("window unavailable"))?;
    let tauri = js_sys::Reflect::get(&window, &JsValue::from_str("__TAURI__"))?;
    let core = js_sys::Reflect::get(&tauri, &JsValue::from_str("core"))?;
    let invoke_fn = js_sys::Reflect::get(&core, &JsValue::from_str("invoke"))?;
    let func = invoke_fn.dyn_into::<js_sys::Function>()?;
    let cmd_val = JsValue::from_str(cmd);
    let promise_val = func.call2(&core, &cmd_val, &args)?;
    let promise = promise_val.dyn_into::<js_sys::Promise>()?;
    wasm_bindgen_futures::JsFuture::from(promise).await
}

pub async fn listen(
    event: &str,
    handler: &js_sys::Function,
) -> Result<js_sys::Function, JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("window unavailable"))?;
    let tauri = js_sys::Reflect::get(&window, &JsValue::from_str("__TAURI__"))?;
    let event_mod = js_sys::Reflect::get(&tauri, &JsValue::from_str("event"))?;
    let listen_fn = js_sys::Reflect::get(&event_mod, &JsValue::from_str("listen"))?;
    let func = listen_fn.dyn_into::<js_sys::Function>()?;
    let event_val = JsValue::from_str(event);
    let promise_val = func.call2(&event_mod, &event_val, handler)?;
    let promise = promise_val.dyn_into::<js_sys::Promise>()?;
    let unlisten = wasm_bindgen_futures::JsFuture::from(promise).await?;
    unlisten.dyn_into::<js_sys::Function>()
}
