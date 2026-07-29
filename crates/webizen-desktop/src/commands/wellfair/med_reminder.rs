#![allow(non_snake_case)]

use super::*;
use tauri::{command, AppHandle, Manager};

#[command]
pub fn wellfair_med_reminder_prefs(app: AppHandle) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        serde_json::to_string(&host.med_reminder_prefs()).map_err(|e| e.to_string())
    })?
}

#[command]
pub fn wellfair_grant_med_reminder_permission(app: AppHandle) -> Result<String, String> {
    let app_clone = app.clone();
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let prefs = host.grant_med_reminder_permission()?;
        let _ = crate::med_reminder_notifier::request_os_notification_permission(&app_clone);
        serde_json::to_string(&prefs).map_err(|e| e.to_string())
    })?
}

#[command]
pub fn wellfair_set_med_reminders_enabled(app: AppHandle, enabled: bool) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let prefs = host.set_med_reminders_enabled(enabled)?;
        serde_json::to_string(&prefs).map_err(|e| e.to_string())
    })?
}

#[command]
pub fn wellfair_list_due_med_reminders(
    app: AppHandle,
    window_minutes: Option<i32>,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let due = host.list_due_med_reminders(window_minutes.unwrap_or(30))?;
        serde_json::to_string(&due).map_err(|e| e.to_string())
    })?
}

#[command]
pub fn wellfair_query_graph_coverage(
    app: AppHandle,
    limit: Option<usize>,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let rows = host.query_graph_coverage(limit.unwrap_or(64))?;
        serde_json::to_string(&rows).map_err(|e| e.to_string())
    })?
}
