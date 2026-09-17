//! Unified Q42 volume console — list, inspect, verify, magnet, compact, browse.

use qualia_client_core::api;
use tauri::command;

#[command]
pub fn list_q42_volumes() -> Result<api::Q42VolumeWorkspace, String> {
    api::list_q42_volumes()
}

#[command]
pub fn inspect_q42_volume(
    path: String,
) -> Result<qualia_core_db::q42_volume::Q42InspectReport, String> {
    api::inspect_q42_volume(path)
}

#[command]
pub async fn verify_q42_volume(
    path: String,
    level: Option<String>,
) -> Result<qualia_core_db::q42_volume::Q42VerifySetReport, String> {
    tokio::task::spawn_blocking(move || api::verify_q42_volume(path, level))
        .await
        .map_err(|e| format!("verify task: {e}"))?
}

#[command]
pub async fn magnet_q42_volume(path: String) -> Result<api::Q42MagnetResult, String> {
    tokio::task::spawn_blocking(move || api::magnet_q42_volume(path))
        .await
        .map_err(|e| format!("magnet task: {e}"))?
}

#[command]
pub async fn compact_q42_volume(path: String) -> Result<api::Q42CompactResult, String> {
    tokio::task::spawn_blocking(move || api::compact_q42_volume(path))
        .await
        .map_err(|e| format!("compact task: {e}"))?
}

/// Finder/Drive-style reopen — returns a `.q42` path without the human typing it.
#[command]
pub async fn open_q42_file_picker(app: tauri::AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;

    let (tx, rx) = std::sync::mpsc::channel();
    app.dialog()
        .file()
        .add_filter("Qualia volume", &["q42"])
        .pick_file(move |path| {
            let result = path
                .and_then(|p| p.into_path().ok())
                .map(|p| p.to_string_lossy().to_string());
            let _ = tx.send(result);
        });
    rx.recv().map_err(|e| format!("File picker channel: {e}"))
}

/// Start a new keep — save-as picker, still a `.q42` (no Host invent).
#[command]
pub async fn save_q42_file_picker(app: tauri::AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;

    let (tx, rx) = std::sync::mpsc::channel();
    app.dialog()
        .file()
        .add_filter("Qualia volume", &["q42"])
        .set_file_name("keep.q42")
        .save_file(move |path| {
            let result = path.and_then(|p| p.into_path().ok()).map(|p| {
                let mut s = p.to_string_lossy().to_string();
                if !s.to_ascii_lowercase().ends_with(".q42") {
                    s.push_str(".q42");
                }
                s
            });
            let _ = tx.send(result);
        });
    rx.recv().map_err(|e| format!("File picker channel: {e}"))
}
