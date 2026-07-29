#![allow(non_snake_case)]

use super::*;
use tauri::{command, AppHandle, Manager};

#[command]
pub fn wellfair_add_life_event(app: AppHandle, report_json: String) -> Result<String, String> {
    let report: wellfare_core::life_records::LifeEventReport =
        serde_json::from_str(&report_json).map_err(|e| format!("invalid life event JSON: {e}"))?;
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_mut()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let entry = host.add_life_event(&report)?;
        serde_json::to_string(&entry).map_err(|e| e.to_string())
    })?
}

#[command]
pub fn wellfair_add_welfare_case(app: AppHandle, report_json: String) -> Result<String, String> {
    let report: wellfare_core::life_records::WelfareCaseReport = serde_json::from_str(&report_json)
        .map_err(|e| format!("invalid welfare case JSON: {e}"))?;
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_mut()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let entry = host.add_welfare_case(&report)?;
        serde_json::to_string(&entry).map_err(|e| e.to_string())
    })?
}

#[command]
pub fn wellfair_add_case_task(app: AppHandle, report_json: String) -> Result<String, String> {
    let report: wellfare_core::life_records::CaseTaskReport =
        serde_json::from_str(&report_json).map_err(|e| format!("invalid case task JSON: {e}"))?;
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_mut()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let entry = host.add_case_task(&report)?;
        serde_json::to_string(&entry).map_err(|e| e.to_string())
    })?
}
