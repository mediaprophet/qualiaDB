#![allow(non_snake_case)]

use super::*;
use tauri::{command, AppHandle, Manager};

// --- Sync transport (T3.1): sync against an HTTP relay -----------------------------------------

/// Drain the outbox to the relay at `base_url`, then pull + admit from it. Returns
/// `{ "pushed": n, "pulled": n, "validated": n, "duplicate": n, "rejected": n }`. `since` is the
/// pull cursor (0 = from the start; admission dedups so re-pulling is safe).
#[command]
pub fn wellfair_sync_with_relay(
    app: AppHandle,
    base_url: String,
    since: u64,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let (pushed, report) = host.sync_with_http_relay(&base_url, since)?;
        Ok(serde_json::json!({
            "pushed": pushed,
            "pulled": report.pulled,
            "validated": report.validated,
            "duplicate": report.duplicate,
            "rejected": report.rejected,
        })
        .to_string())
    })?
}

// --- Backup / restore (T3.3) -----------------------------------------------------------------

/// Write a portable backup of this node's WellFair data to `path`. Returns `{ "files": n, "bytes": n }`.
#[command]
pub fn wellfair_export_backup(app: AppHandle, path: String) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let report = host.export_backup_to_path(&path)?;
        Ok(serde_json::json!({ "files": report.files, "bytes": report.bytes }).to_string())
    })?
}

/// Restore a backup archive from `path` into this node's storage. Returns `{ "files": n, "bytes": n }`.
#[command]
pub fn wellfair_import_backup(app: AppHandle, path: String) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let report = host.import_backup_from_path(&path)?;
        Ok(serde_json::json!({ "files": report.files, "bytes": report.bytes }).to_string())
    })?
}

/// A node health/status snapshot (records, sync queues, data footprint, Sanctuary state, version).
#[command]
pub fn wellfair_diagnostics(app: AppHandle) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        serde_json::to_string(&host.diagnostics_report()?).map_err(|e| e.to_string())
    })?
}

