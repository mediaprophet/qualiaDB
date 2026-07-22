//! Cooperative work items / Kanban board

use super::*;
use serde::Deserialize;

#[cfg(target_arch = "wasm32")]
use crate::components::qapp_engine::tauri_invoke;
#[cfg(target_arch = "wasm32")]
use js_sys;


#[derive(Debug, Clone, Deserialize)]
pub struct BoardCardDto {
    pub work_item_id: String,
    pub title: String,
    pub item_type: String,
    pub priority: String,
    pub status: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BoardColumnDto {
    pub status: String,
    pub cards: Vec<BoardCardDto>,
}

#[cfg(target_arch = "wasm32")]
pub async fn add_work_item(
    project_id: &str,
    item_type: &str,
    title: &str,
) -> Result<HealthRecordDto, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"projectId".into(), &wasm_bindgen::JsValue::from_str(project_id))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"itemType".into(), &wasm_bindgen::JsValue::from_str(item_type))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"title".into(), &wasm_bindgen::JsValue::from_str(title))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_add_work_item", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let out = js.as_string().ok_or_else(|| "work item response not JSON".to_string())?;
    serde_json::from_str(&out).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn add_work_item(
    _project_id: &str,
    _item_type: &str,
    _title: &str,
) -> Result<HealthRecordDto, String> {
    Err("Work items require the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn add_work_item_status(work_item_id: &str, status: &str) -> Result<HealthRecordDto, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"workItemId".into(), &wasm_bindgen::JsValue::from_str(work_item_id))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"status".into(), &wasm_bindgen::JsValue::from_str(status))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_add_work_item_status", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let out = js.as_string().ok_or_else(|| "status response not JSON".to_string())?;
    serde_json::from_str(&out).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn add_work_item_status(_work_item_id: &str, _status: &str) -> Result<HealthRecordDto, String> {
    Err("Work items require the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn fetch_work_item_board(project_id: &str) -> Result<Vec<BoardColumnDto>, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"projectId".into(), &wasm_bindgen::JsValue::from_str(project_id))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"limit".into(), &wasm_bindgen::JsValue::from(256u32))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_work_item_board", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let json = js.as_string().ok_or_else(|| "board response not JSON".to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn fetch_work_item_board(_project_id: &str) -> Result<Vec<BoardColumnDto>, String> {
    Ok(vec![])
}

