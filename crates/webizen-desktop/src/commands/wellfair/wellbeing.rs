#![allow(non_snake_case)]

use super::*;
use tauri::{command, AppHandle, Manager};

#[command]
pub fn wellfair_add_wellbeing_observation(
    app: AppHandle,
    report_json: String,
) -> Result<String, String> {
    let report: wellfare_core::mental_wellbeing::WellbeingObservation =
        serde_json::from_str(&report_json).map_err(|e| format!("invalid wellbeing JSON: {e}"))?;
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_mut()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let entry = host.add_wellbeing_observation(&report)?;
        serde_json::to_string(&entry).map_err(|e| e.to_string())
    })?
}

#[command]
pub fn wellfair_add_therapy_note(app: AppHandle, report_json: String) -> Result<String, String> {
    let report: wellfare_core::mental_wellbeing::TherapyNote =
        serde_json::from_str(&report_json)
            .map_err(|e| format!("invalid therapy note JSON: {e}"))?;
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_mut()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let entry = host.add_therapy_note(&report)?;
        serde_json::to_string(&entry).map_err(|e| e.to_string())
    })?
}

#[derive(Debug, serde::Serialize)]
struct LiveShareRequestDto {
    id: String,
    device_id: String,
    purpose: String,
    requested_kinds: Vec<String>,
    ttl_seconds: u32,
}

#[command]
pub fn wellfair_list_pending_live_shares(app: AppHandle, limit: usize) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let pending = host.list_pending_live_shares(limit)?;
        let dtos: Vec<LiveShareRequestDto> = pending
            .into_iter()
            .map(|r| LiveShareRequestDto {
                id: r.id,
                device_id: r.device_id,
                purpose: r.purpose,
                requested_kinds: r.requested_kinds,
                ttl_seconds: r.ttl_seconds,
            })
            .collect();
        serde_json::to_string(&dtos).map_err(|e| e.to_string())
    })?
}

#[command]
pub fn wellfair_decide_live_share(
    app: AppHandle,
    request_id: String,
    approved: bool,
    projection_kinds: Vec<String>,
    reason: Option<String>,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let entry = host.decide_live_share_request(
            &request_id,
            approved,
            if approved { &projection_kinds } else { &[] },
            reason.as_deref(),
        )?;
        serde_json::to_string(&entry).map_err(|e| e.to_string())
    })?
}
