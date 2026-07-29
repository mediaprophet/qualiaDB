//! Chora host client — spatio-temporal canvas bridges via Tauri invoke.

#[cfg(target_arch = "wasm32")]
use crate::components::qapp_engine::tauri_invoke;
#[cfg(target_arch = "wasm32")]
use js_sys;

#[cfg(target_arch = "wasm32")]
pub async fn list_canvas_worlds() -> Result<Vec<serde_json::Value>, String> {
    let js = tauri_invoke("chora_list_worlds", wasm_bindgen::JsValue::NULL)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "worlds response not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn list_canvas_worlds() -> Result<Vec<serde_json::Value>, String> {
    Err("Chora requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn get_canvas_world(world_id: &str) -> Result<serde_json::Value, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(
        &args,
        &"worldId".into(),
        &wasm_bindgen::JsValue::from_str(world_id),
    )
    .map_err(|_| "args".to_string())?;
    let js = tauri_invoke("chora_get_world", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "world response not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn get_canvas_world(_world_id: &str) -> Result<serde_json::Value, String> {
    Err("Chora requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn save_canvas_world(config_json: &str) -> Result<(), String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(
        &args,
        &"configJson".into(),
        &wasm_bindgen::JsValue::from_str(config_json),
    )
    .map_err(|_| "args".to_string())?;
    tauri_invoke("chora_save_world", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    Ok(())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn save_canvas_world(_config_json: &str) -> Result<(), String> {
    Err("Chora requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn delete_canvas_world(world_id: &str) -> Result<bool, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(
        &args,
        &"worldId".into(),
        &wasm_bindgen::JsValue::from_str(world_id),
    )
    .map_err(|_| "args".to_string())?;
    let js = tauri_invoke("chora_delete_world", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    js.as_bool()
        .ok_or_else(|| "delete response not bool".to_string())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn delete_canvas_world(_world_id: &str) -> Result<bool, String> {
    Err("Chora requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn seed_canvas_demo() -> Result<bool, String> {
    let js = tauri_invoke("chora_seed_demo", wasm_bindgen::JsValue::NULL)
        .await
        .map_err(|e| format!("{e:?}"))?;
    js.as_bool()
        .ok_or_else(|| "seed response not bool".to_string())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn seed_canvas_demo() -> Result<bool, String> {
    Err("Chora requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn canvas_navigation() -> Result<serde_json::Value, String> {
    let js = tauri_invoke("chora_navigation", wasm_bindgen::JsValue::NULL)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "nav response not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn canvas_navigation() -> Result<serde_json::Value, String> {
    Err("Chora requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn set_canvas_temporal(t_value: f64) -> Result<(), String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(
        &args,
        &"tValue".into(),
        &wasm_bindgen::JsValue::from_f64(t_value),
    )
    .map_err(|_| "args".to_string())?;
    tauri_invoke("chora_set_temporal", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    Ok(())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn set_canvas_temporal(_t: f64) -> Result<(), String> {
    Err("Chora requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn set_active_canvas_world(world_id: &str) -> Result<(), String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(
        &args,
        &"worldId".into(),
        &wasm_bindgen::JsValue::from_str(world_id),
    )
    .map_err(|_| "args".to_string())?;
    tauri_invoke("chora_set_active_world", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    Ok(())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn set_active_canvas_world(_id: &str) -> Result<(), String> {
    Err("Chora requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn query_canvas_region(
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
) -> Result<Vec<serde_json::Value>, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"x1".into(), &wasm_bindgen::JsValue::from_f64(x1))
        .map_err(|_| "args".to_string())?;
    js_sys::Reflect::set(&args, &"y1".into(), &wasm_bindgen::JsValue::from_f64(y1))
        .map_err(|_| "args".to_string())?;
    js_sys::Reflect::set(&args, &"x2".into(), &wasm_bindgen::JsValue::from_f64(x2))
        .map_err(|_| "args".to_string())?;
    js_sys::Reflect::set(&args, &"y2".into(), &wasm_bindgen::JsValue::from_f64(y2))
        .map_err(|_| "args".to_string())?;
    let js = tauri_invoke("chora_query_region", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "query response not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn query_canvas_region(
    _x1: f64,
    _y1: f64,
    _x2: f64,
    _y2: f64,
) -> Result<Vec<serde_json::Value>, String> {
    Err("Chora requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn publish_planted_asset(asset_json: &str) -> Result<(), String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(
        &args,
        &"assetJson".into(),
        &wasm_bindgen::JsValue::from_str(asset_json),
    )
    .map_err(|_| "args".to_string())?;
    tauri_invoke("chora_publish_asset", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    Ok(())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn publish_planted_asset(_asset_json: &str) -> Result<(), String> {
    Err("Chora requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn pull_spatial_assets(cell_id: u64) -> Result<Vec<serde_json::Value>, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(
        &args,
        &"cellId".into(),
        &wasm_bindgen::JsValue::from_f64(cell_id as f64),
    )
    .map_err(|_| "args".to_string())?;
    let js = tauri_invoke("chora_pull_assets", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "pull response not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn pull_spatial_assets(_cell_id: u64) -> Result<Vec<serde_json::Value>, String> {
    Err("Chora requires the Tauri desktop host".into())
}

// ── Layer library + asset download pipeline ─────────────────────────────────

#[cfg(target_arch = "wasm32")]
pub async fn list_layers() -> Result<Vec<serde_json::Value>, String> {
    let js = tauri_invoke("chora_list_layers", wasm_bindgen::JsValue::NULL)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "layers response not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn list_layers() -> Result<Vec<serde_json::Value>, String> {
    Err("Chora requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn download_layer(layer_id: &str, resolution: u32) -> Result<serde_json::Value, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(
        &args,
        &"layerId".into(),
        &wasm_bindgen::JsValue::from_str(layer_id),
    )
    .map_err(|_| "args".to_string())?;
    js_sys::Reflect::set(
        &args,
        &"resolution".into(),
        &wasm_bindgen::JsValue::from_f64(resolution as f64),
    )
    .map_err(|_| "args".to_string())?;
    let js = tauri_invoke("chora_download_layer", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js
        .as_string()
        .ok_or_else(|| "download response not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn download_layer(
    _layer_id: &str,
    _resolution: u32,
) -> Result<serde_json::Value, String> {
    Err("Chora requires the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn set_gpu_camera_mode(mode: &str) -> Result<(), String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(
        &args,
        &"mode".into(),
        &wasm_bindgen::JsValue::from_str(mode),
    )
    .map_err(|_| "args".to_string())?;
    tauri_invoke("set_gpu_camera_mode", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    Ok(())
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn set_gpu_camera_mode(_mode: &str) -> Result<(), String> {
    Err("Chora requires the Tauri desktop host".into())
}
