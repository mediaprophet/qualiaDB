//! Clinical documents / Welfare support / Sync inbox

use super::*;

#[cfg(target_arch = "wasm32")]
use crate::components::qapp_engine::tauri_invoke;
#[cfg(target_arch = "wasm32")]
use js_sys;


#[cfg(target_arch = "wasm32")]
pub async fn add_clinical_report(
    title: &str,
    report_type: &str,
    body: &str,
    author_label: Option<&str>,
) -> Result<HealthRecordDto, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"title".into(), &wasm_bindgen::JsValue::from_str(title))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"reportType".into(), &wasm_bindgen::JsValue::from_str(report_type))
        .map_err(|_| "failed to build invoke args".to_string())?;
    // 0 â†’ the host stamps "now".
    js_sys::Reflect::set(&args, &"observedAtUnix".into(), &wasm_bindgen::JsValue::from(0u32))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"body".into(), &wasm_bindgen::JsValue::from_str(body))
        .map_err(|_| "failed to build invoke args".to_string())?;
    if let Some(a) = author_label {
        js_sys::Reflect::set(&args, &"authorLabel".into(), &wasm_bindgen::JsValue::from_str(a))
            .map_err(|_| "failed to build invoke args".to_string())?;
    }
    let js = tauri_invoke("wellfair_add_clinical_report", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let out = js.as_string().ok_or_else(|| "clinical response was not JSON".to_string())?;
    serde_json::from_str(&out).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn add_clinical_report(
    _title: &str,
    _report_type: &str,
    _body: &str,
    _author_label: Option<&str>,
) -> Result<HealthRecordDto, String> {
    Err("Clinical reports require the Tauri desktop host".into())
}

#[cfg(target_arch = "wasm32")]
pub async fn add_clinical_attachment_from_path(
    path: &str,
    media_type: Option<&str>,
) -> Result<HealthRecordDto, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"path".into(), &wasm_bindgen::JsValue::from_str(path))
        .map_err(|_| "failed to build invoke args".to_string())?;
    if let Some(m) = media_type {
        js_sys::Reflect::set(&args, &"mediaType".into(), &wasm_bindgen::JsValue::from_str(m))
            .map_err(|_| "failed to build invoke args".to_string())?;
    }
    let js = tauri_invoke("wellfair_add_clinical_attachment_from_path", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let out = js.as_string().ok_or_else(|| "attachment response not JSON".to_string())?;
    serde_json::from_str(&out).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn add_clinical_attachment_from_path(
    _path: &str,
    _media_type: Option<&str>,
) -> Result<HealthRecordDto, String> {
    Err("Clinical attachments require the Tauri desktop host".into())
}

/// Export an attachment's bytes to a destination path; returns the host's JSON summary.
#[cfg(target_arch = "wasm32")]
pub async fn export_attachment(record_id: &str, dest_path: &str) -> Result<String, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(&args, &"recordId".into(), &wasm_bindgen::JsValue::from_str(record_id))
        .map_err(|_| "failed to build invoke args".to_string())?;
    js_sys::Reflect::set(&args, &"destPath".into(), &wasm_bindgen::JsValue::from_str(dest_path))
        .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_export_attachment", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    js.as_string().ok_or_else(|| "export response not JSON".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn export_attachment(_record_id: &str, _dest_path: &str) -> Result<String, String> {
    Err("Attachment export requires the Tauri desktop host".into())
}

/// Parse a host `Option<String>` command response (JSON `null` or a JSON string) into
/// `Option<String>`. The dialog commands return the chosen path, or `null` on cancel.
#[cfg(target_arch = "wasm32")]
fn parse_optional_path(js: wasm_bindgen::JsValue) -> Result<Option<String>, String> {
    if js.is_null() || js.is_undefined() {
        return Ok(None);
    }
    if let Some(s) = js.as_string() {
        // Serde may hand back either a bare JS string or a JSON-encoded string.
        if let Ok(inner) = serde_json::from_str::<Option<String>>(&s) {
            return Ok(inner);
        }
        return Ok(Some(s));
    }
    serde_wasm_bindgen::from_value(js).map_err(|e| e.to_string())
}

/// Open a native OS file-open dialog on the desktop host; returns the chosen absolute path
/// (or `None` if the user cancelled).
#[cfg(target_arch = "wasm32")]
pub async fn pick_file_path() -> Result<Option<String>, String> {
    let js = tauri_invoke("wellfair_pick_file_path", wasm_bindgen::JsValue::NULL)
        .await
        .map_err(|e| format!("{e:?}"))?;
    parse_optional_path(js)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn pick_file_path() -> Result<Option<String>, String> {
    Ok(None)
}

/// Open a native OS file-save dialog on the desktop host, seeded with `default_name`;
/// returns the chosen absolute path (or `None` if the user cancelled).
#[cfg(target_arch = "wasm32")]
pub async fn pick_save_path(default_name: &str) -> Result<Option<String>, String> {
    let args = js_sys::Object::new();
    js_sys::Reflect::set(
        &args,
        &"defaultName".into(),
        &wasm_bindgen::JsValue::from_str(default_name),
    )
    .map_err(|_| "failed to build invoke args".to_string())?;
    let js = tauri_invoke("wellfair_pick_save_path", args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    parse_optional_path(js)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn pick_save_path(_default_name: &str) -> Result<Option<String>, String> {
    Ok(None)
}

/// Open a native OS folder-picker; returns the chosen directory (or `None` if cancelled).
#[cfg(target_arch = "wasm32")]
pub async fn pick_directory() -> Result<Option<String>, String> {
    let js = tauri_invoke("wellfair_pick_directory", wasm_bindgen::JsValue::NULL)
        .await
        .map_err(|e| format!("{e:?}"))?;
    parse_optional_path(js)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn pick_directory() -> Result<Option<String>, String> {
    Ok(None)
}

