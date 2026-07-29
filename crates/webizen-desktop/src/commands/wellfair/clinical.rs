#![allow(non_snake_case)]

use super::*;
use tauri::{command, AppHandle, Manager};

fn parse_clinical_report_type(s: &str) -> wellfare_core::clinical::ClinicalReportType {
    use wellfare_core::clinical::ClinicalReportType::*;
    match s.to_ascii_lowercase().as_str() {
        "pathology" => Pathology,
        "imaging" => Imaging,
        "discharge" => Discharge,
        "referral" => Referral,
        _ => Other,
    }
}

#[command]
pub fn wellfair_add_clinical_report(
    app: AppHandle,
    title: String,
    report_type: String,
    observed_at_unix: u32,
    body: String,
    author_label: Option<String>,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_mut()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let observed = if observed_at_unix == 0 {
            wellfair_now_unix()
        } else {
            observed_at_unix
        };
        let entry = host.add_clinical_report(
            &title,
            parse_clinical_report_type(&report_type),
            observed,
            &body,
            author_label,
        )?;
        serde_json::to_string(&entry).map_err(|e| e.to_string())
    })?
}

fn guess_media_type(filename: &str) -> String {
    let ext = std::path::Path::new(filename)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "pdf" => "application/pdf",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "txt" => "text/plain",
        "csv" => "text/csv",
        "json" => "application/json",
        "dcm" => "application/dicom",
        _ => "application/octet-stream",
    }
    .to_string()
}

#[command]
pub fn wellfair_add_clinical_attachment_from_path(
    app: AppHandle,
    path: String,
    media_type: Option<String>,
) -> Result<String, String> {
    let bytes = std::fs::read(&path).map_err(|e| format!("cannot read {path}: {e}"))?;
    let filename = std::path::Path::new(&path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("attachment")
        .to_string();
    let media = media_type
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| guess_media_type(&filename));
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_mut()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let entry = host.add_clinical_attachment(&filename, &media, &bytes)?;
        serde_json::to_string(&entry).map_err(|e| e.to_string())
    })?
}

#[command]
pub fn wellfair_export_attachment(
    app: AppHandle,
    record_id: String,
    dest_path: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let bytes = host
            .attachment_bytes(&record_id)?
            .ok_or_else(|| "attachment not found".to_string())?;
        std::fs::write(&dest_path, &bytes).map_err(|e| format!("cannot write {dest_path}: {e}"))?;
        Ok(serde_json::json!({ "written": bytes.len(), "path": dest_path }).to_string())
    })?
}

/// Convert a dialog `FilePath` into an absolute path string.
///
/// On desktop the picker yields `FilePath::Path`, so `into_path()` returns the `PathBuf`
/// directly; `simplified()` first normalises Windows UNC prefixes. If the variant is a URL
/// that cannot be resolved to a filesystem path, fall back to its `Display` form so the
/// caller still receives a usable string rather than an error.
fn dialog_file_path_to_string(fp: tauri_plugin_dialog::FilePath) -> String {
    let display = fp.to_string();
    match fp.simplified().into_path() {
        Ok(path) => path.to_string_lossy().into_owned(),
        Err(_) => display,
    }
}

/// Open a native OS "open file" dialog (blocking). Returns the chosen absolute path,
/// or `None` if the user cancelled. Lets the operator browse instead of typing a path.
#[command]
pub fn wellfair_pick_file_path(app: AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let picked = app.dialog().file().blocking_pick_file();
    Ok(picked.map(dialog_file_path_to_string))
}

/// Open a native OS "save file" dialog (blocking), seeded with `default_name`. Returns the
/// chosen absolute path, or `None` if the user cancelled.
#[command]
pub fn wellfair_pick_save_path(
    app: AppHandle,
    default_name: String,
) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let mut builder = app.dialog().file();
    let trimmed = default_name.trim();
    if !trimmed.is_empty() {
        builder = builder.set_file_name(trimmed);
    }
    let picked = builder.blocking_save_file();
    Ok(picked.map(dialog_file_path_to_string))
}

/// Open a native OS folder-picker (blocking). Returns the chosen directory, or `None` if cancelled.
#[command]
pub fn wellfair_pick_directory(app: AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let picked = app.dialog().file().blocking_pick_folder();
    Ok(picked.map(dialog_file_path_to_string))
}

/// WP2: author a qapp and write its installable PWA bundle into `target_dir`. Returns the written
/// file paths (JSON array).
#[command]
#[allow(clippy::too_many_arguments)]
pub fn wellfair_publish_qapp_pwa(
    app: AppHandle,
    target_dir: String,
    id: String,
    name: String,
    kind: String,
    description: String,
    capabilities: String,
    wasm_filename: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let written = host.publish_qapp_pwa(
            &target_dir,
            &id,
            &name,
            &kind,
            &description,
            &capabilities,
            &wasm_filename,
        )?;
        serde_json::to_string(&written).map_err(|e| e.to_string())
    })?
}

#[command]
pub fn wellfair_add_government_letter_attachment_from_path(
    app: AppHandle,
    sender: String,
    subject: String,
    action_required: bool,
    path: String,
) -> Result<String, String> {
    let bytes = std::fs::read(&path).map_err(|e| format!("cannot read {path}: {e}"))?;
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_mut()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let entry =
            host.add_government_letter_attachment(&sender, &subject, action_required, &bytes)?;
        serde_json::to_string(&entry).map_err(|e| e.to_string())
    })?
}
