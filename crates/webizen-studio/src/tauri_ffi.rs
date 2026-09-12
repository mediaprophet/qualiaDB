//! Single-owner Tauri wasm-bindgen FFI.
//!
//! `dx build --web` links `libwebizen_studio.rlib` into the studio bin. wasm-bindgen
//! describe symbols (`__wbindgen_describe___wbg_invoke_*`, `___wbg_listen_*`) are
//! global per JS import path (`window.__TAURI__.core.invoke` /
//! `window.__TAURI__.event.listen`). Declaring those imports in more than one
//! compilation unit (lib + bin, or many component modules that survive into both)
//! makes rust-lld fail with duplicate symbols.
//!
//! This module is the only place that may emit those imports. Call sites use
//! [`invoke`] / [`listen`] (or the `tauri_invoke` / `tauri_listen` aliases).
//! Do not add `#[wasm_bindgen] extern` Tauri bindings elsewhere.

/// Raw `window.__TAURI__.core.invoke` import. Exists only in this module.
#[cfg(target_arch = "wasm32")]
mod bindings {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke, catch)]
        pub async fn invoke_raw(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;

        #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "event"], js_name = listen, catch)]
        pub async fn listen_raw(event: &str, handler: &JsValue) -> Result<JsValue, JsValue>;
    }
}

/// Invoke a Tauri command. `args` accepts `JsValue` or `js_sys::Object`.
#[cfg(target_arch = "wasm32")]
pub async fn invoke(
    cmd: &str,
    args: impl Into<wasm_bindgen::JsValue>,
) -> Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue> {
    bindings::invoke_raw(cmd, args.into()).await
}

/// Subscribe to a Tauri event. `handler` accepts `JsValue`, `js_sys::Function`,
/// or `Closure<T>` (all `AsRef<JsValue>`).
#[cfg(target_arch = "wasm32")]
pub async fn listen(
    event: &str,
    handler: impl AsRef<wasm_bindgen::JsValue>,
) -> Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue> {
    bindings::listen_raw(event, handler.as_ref()).await
}

#[cfg(target_arch = "wasm32")]
pub use invoke as tauri_invoke;
#[cfg(target_arch = "wasm32")]
pub use listen as tauri_listen;

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    fn studio_src() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
    }

    fn collect_rs(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
        let entries = fs::read_dir(dir).unwrap_or_else(|err| {
            panic!("read {}: {err}", dir.display());
        });
        for entry in entries {
            let entry = entry.expect("dir entry");
            let path = entry.path();
            if path.is_dir() {
                collect_rs(&path, out);
            } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                out.push(path);
            }
        }
    }

    /// Regression: a second `#[wasm_bindgen]` import of Tauri invoke/listen
    /// re-breaks `dx build --web` with rust-lld duplicate describe symbols.
    #[test]
    fn only_this_module_declares_tauri_wbindgen_imports() {
        let mut files = Vec::new();
        collect_rs(&studio_src(), &mut files);
        let mut offenders = Vec::new();
        for path in files {
            if path.file_name().and_then(|n| n.to_str()) == Some("tauri_ffi.rs") {
                continue;
            }
            let text = fs::read_to_string(&path).unwrap_or_else(|err| {
                panic!("read {}: {err}", path.display());
            });
            if text.contains("js_namespace = [\"window\", \"__TAURI__\"") {
                offenders.push(path.display().to_string());
            }
        }
        assert!(
            offenders.is_empty(),
            "Tauri wasm-bindgen invoke/listen must live only in tauri_ffi.rs; found in:\n{}",
            offenders.join("\n")
        );
    }
}
