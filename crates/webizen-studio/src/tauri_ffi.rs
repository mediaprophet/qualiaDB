//! Single-owner Tauri wasm-bindgen imports for the studio crate.
//!
//! `dx build --web` links `libwebizen_studio.rlib` into the bin. Every
//! `#[wasm_bindgen] extern` for the same JS `invoke` / `listen` path emits the
//! same `__wbindgen_describe___wbg_*` symbol. Compiling those externs in both
//! the rlib (spatial_bridge) and the bin (main/components) makes rust-lld fail
//! with duplicate symbols even when `cargo check` is green.
//!
//! Own the imports exactly once here. Call sites use `invoke` / `listen`
//! (re-exported) and must not declare their own extern blocks.

#![cfg(target_arch = "wasm32")]

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke, catch)]
    pub async fn invoke(
        cmd: &str,
        args: JsValue,
    ) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "event"], js_name = listen, catch)]
    pub async fn listen(
        event: &str,
        handler: &js_sys::Function,
    ) -> Result<js_sys::Function, JsValue>;
}
