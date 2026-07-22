#![allow(non_snake_case)]

use super::*;
use tauri::{command, AppHandle, Manager};

/// Score + record a sitting. `responses` is a comma-separated list of ordinal values (one per item,
/// in order). Returns the scored result (total, band, interpretation, any safety flags).
#[command]
pub fn wellfair_record_assessment(
    app: AppHandle,
    instrument_id: String,
    responses: String,
) -> Result<String, String> {
    let parsed: Result<Vec<u8>, _> = responses
        .split(',')
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().parse::<u8>())
        .collect();
    let parsed = parsed.map_err(|e| format!("invalid responses: {e}"))?;
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_mut()
            .ok_or_else(|| "Host API not initialized â€” unlock vault first".to_string())?;
        let result = host.record_assessment(&instrument_id, parsed)?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    })?
}

/// Past assessment results (newest-first).
#[command]
pub fn wellfair_list_assessments(app: AppHandle) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized â€” unlock vault first".to_string())?;
        serde_json::to_string(&host.list_assessments(64)?).map_err(|e| e.to_string())
    })?
}

