#![allow(non_snake_case)]

use super::*;
use tauri::{command, AppHandle, Manager};

#[command]
pub fn wellfair_sanctuary_prefs(app: AppHandle) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized â€” unlock vault first".to_string())?;
        serde_json::to_string(&host.sanctuary_prefs()).map_err(|e| e.to_string())
    })?
}

#[command]
pub fn wellfair_setup_sanctuary(
    app: AppHandle,
    real_pin: String,
    decoy_pin: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized â€” unlock vault first".to_string())?;
        let prefs = host.setup_sanctuary(&real_pin, &decoy_pin)?;
        serde_json::to_string(&prefs).map_err(|e| e.to_string())
    })?
}

#[command]
pub fn wellfair_lock_sanctuary(app: AppHandle) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized â€” unlock vault first".to_string())?;
        let prefs = host.lock_sanctuary()?;
        serde_json::to_string(&prefs).map_err(|e| e.to_string())
    })?
}

#[command]
pub fn wellfair_unlock_sanctuary(app: AppHandle, pin: String) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized â€” unlock vault first".to_string())?;
        let prefs = host.unlock_sanctuary(&pin)?;
        serde_json::to_string(&prefs).map_err(|e| e.to_string())
    })?
}

