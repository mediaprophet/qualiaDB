use serde::de::DeserializeOwned;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke, catch)]
    async fn tauri_invoke(
        command: &str,
        args: js_sys::Object,
    ) -> Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue>;
}

#[cfg(target_arch = "wasm32")]
pub async fn invoke_json<T: DeserializeOwned>(
    command: &str,
    args: serde_json::Value,
) -> Result<T, String> {
    if !crate::endpoints::is_native_host() {
        return Err("held / not yet — desktop host not on this surface".to_string());
    }
    // JSON.parse → plain Object. serde_wasm_bindgen Maps drop `path` and
    // Catalog Open pack then false-holds a live bind.
    let json = serde_json::to_string(&args).map_err(|error| error.to_string())?;
    let parsed = js_sys::JSON::parse(&json).map_err(|error| format!("{error:?}"))?;
    let obj = parsed
        .dyn_into::<js_sys::Object>()
        .map_err(|error| format!("{error:?}"))?;
    let result = tauri_invoke(command, obj)
        .await
        .map_err(|error| format!("{error:?}"))?;
    serde_wasm_bindgen::from_value(result).map_err(|error| error.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn invoke_json<T: DeserializeOwned>(
    _command: &str,
    _args: serde_json::Value,
) -> Result<T, String> {
    Err(webizen_studio::lexicon_catalog::HELD_WHY.to_string())
}
