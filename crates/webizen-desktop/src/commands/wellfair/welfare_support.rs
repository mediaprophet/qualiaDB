#![allow(non_snake_case)]

use super::*;
use tauri::{command, AppHandle, Manager};

#[command]
pub fn wellfair_add_assistance_need(
    app: AppHandle,
    category: String,
    description: String,
    urgency: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_mut()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let entry = host.add_assistance_need(&category, &description, parse_urgency(&urgency))?;
        serde_json::to_string(&entry).map_err(|e| e.to_string())
    })?
}

#[command]
pub fn wellfair_add_welfare_stream(
    app: AppHandle,
    program_name: String,
    reference: Option<String>,
    status: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_mut()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let entry = host.add_welfare_stream(&program_name, reference, parse_stream_status(&status))?;
        serde_json::to_string(&entry).map_err(|e| e.to_string())
    })?
}

#[command]
pub fn wellfair_add_government_letter(
    app: AppHandle,
    sender: String,
    subject: String,
    action_required: bool,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_mut()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let entry = host.add_government_letter(&sender, &subject, action_required)?;
        serde_json::to_string(&entry).map_err(|e| e.to_string())
    })?
}

#[command]
pub fn wellfair_list_sync_inbox(app: AppHandle, limit: usize) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let inbox = host.list_sync_inbox(limit)?;
        serde_json::to_string(&inbox).map_err(|e| e.to_string())
    })?
}

