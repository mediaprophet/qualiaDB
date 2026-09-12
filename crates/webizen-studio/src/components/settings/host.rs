use serde::de::DeserializeOwned;

#[cfg(target_arch = "wasm32")]
use crate::tauri_ffi::invoke as tauri_invoke;

#[cfg(target_arch = "wasm32")]
pub async fn invoke_json<T: DeserializeOwned>(
    command: &str,
    args: serde_json::Value,
) -> Result<T, String> {
    if !crate::endpoints::is_native_host() {
        return Err("held / not yet — desktop host not on this surface".to_string());
    }
    let args = serde_wasm_bindgen::to_value(&args).map_err(|error| error.to_string())?;
    let result = tauri_invoke(command, args.into())
        .await
        .map_err(|error| format!("{error:?}"))?;
    serde_wasm_bindgen::from_value(result).map_err(|error| error.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn invoke_json<T: DeserializeOwned>(
    _command: &str,
    _args: serde_json::Value,
) -> Result<T, String> {
    Err("Desktop commands are available in the Webizen host".to_string())
}
