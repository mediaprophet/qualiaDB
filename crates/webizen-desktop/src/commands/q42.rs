//! Unified Q42 volume console — list, inspect, verify, magnet, compact.

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
