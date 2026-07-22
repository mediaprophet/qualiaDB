#![allow(non_snake_case)]

use super::*;
use tauri::{command, AppHandle, Manager};

#[command]
pub fn wellfair_ledger_append(
    app: AppHandle,
    kind: String,
    payload_json: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized â€” unlock vault first".to_string())?;
        let entry = host.ledger_append(&kind, &payload_json)?;
        serde_json::to_string(&entry).map_err(|e| e.to_string())
    })?
}

/// Verify the whole ledger chain. Returns `{ "ok": bool, "tamper": <detail|null> }`.
#[command]
pub fn wellfair_ledger_verify(app: AppHandle) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized â€” unlock vault first".to_string())?;
        let tamper = host.ledger_verify()?;
        serde_json::to_string(&serde_json::json!({ "ok": tamper.is_none(), "tamper": tamper }))
            .map_err(|e| e.to_string())
    })?
}

/// The most-recent ledger entries (newest first), capped to `limit` (default 64).
#[command]
pub fn wellfair_ledger_entries(app: AppHandle, limit: Option<usize>) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized â€” unlock vault first".to_string())?;
        let entries = host.ledger_entries(limit.unwrap_or(64))?;
        serde_json::to_string(&entries).map_err(|e| e.to_string())
    })?
}

