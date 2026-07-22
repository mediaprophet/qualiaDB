#![allow(non_snake_case)]

use super::*;
use tauri::{command, AppHandle, Manager};

// --- Wellbeing self-assessment instruments (T2.2; PHQ-9 / GAD-7) -----------------------------

/// The instruments this build ships (items, options, bands, disclaimer).
#[command]
pub fn wellfair_list_assessment_instruments(app: AppHandle) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized â€” unlock vault first".to_string())?;
        serde_json::to_string(&host.list_assessment_instruments()).map_err(|e| e.to_string())
    })?
}

