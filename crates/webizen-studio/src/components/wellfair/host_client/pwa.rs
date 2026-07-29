//! PWA package & publish

use super::*;

#[cfg(target_arch = "wasm32")]
use crate::components::qapp_engine::tauri_invoke;
#[cfg(target_arch = "wasm32")]
use js_sys;

#[cfg(target_arch = "wasm32")]
#[allow(clippy::too_many_arguments)]
pub async fn publish_qapp_pwa(
    target_dir: &str,
    id: &str,
    name: &str,
    kind: &str,
    description: &str,
    capabilities: &str,
    wasm_filename: &str,
) -> Result<Vec<String>, String> {
    let args = js_sys::Object::new();
    for (k, v) in [
        ("targetDir", target_dir),
        ("id", id),
        ("name", name),
        ("kind", kind),
        ("description", description),
        ("capabilities", capabilities),
        ("wasmFilename", wasm_filename),
    ] {
        js_sys::Reflect::set(&args, &k.into(), &wasm_bindgen::JsValue::from_str(v))
            .map_err(|_| "failed to build invoke args".to_string())?;
    }
    let js = tauri_invoke("wellfair_publish_qapp_pwa", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "response not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
#[allow(clippy::too_many_arguments)]
pub async fn publish_qapp_pwa(
    _target_dir: &str,
    _id: &str,
    _name: &str,
    _kind: &str,
    _description: &str,
    _capabilities: &str,
    _wasm_filename: &str,
) -> Result<Vec<String>, String> {
    Err("Package & Publish requires the Tauri desktop host".into())
}
