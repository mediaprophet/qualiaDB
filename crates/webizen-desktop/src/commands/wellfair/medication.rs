#![allow(non_snake_case)]

use super::*;
use tauri::{command, AppHandle, Manager};

fn parse_administration_status(s: &str) -> wellfare_core::medication::AdministrationStatus {
    match s.to_ascii_lowercase().as_str() {
        "skipped" => wellfare_core::medication::AdministrationStatus::Skipped,
        "overdue" => wellfare_core::medication::AdministrationStatus::Overdue,
        _ => wellfare_core::medication::AdministrationStatus::Taken,
    }
}

#[command]
pub fn wellfair_add_medication(
    app: AppHandle,
    name: String,
    dose: String,
    route: String,
    schedule: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_mut()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let times: Vec<String> = schedule
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        let entry = host.add_medication(&name, &dose, &route, times)?;
        serde_json::to_string(&entry).map_err(|e| e.to_string())
    })?
}

#[command]
pub fn wellfair_record_administration(
    app: AppHandle,
    medication_id: String,
    medication_name: String,
    status: String,
    notes: Option<String>,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_mut()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let st = parse_administration_status(&status);
        let entry = host.record_administration(&medication_id, &medication_name, st, notes)?;
        serde_json::to_string(&entry).map_err(|e| e.to_string())
    })?
}

#[command]
pub fn wellfair_add_diet_entry(
    app: AppHandle,
    description: String,
    meal_type: String,
    calories_kcal: Option<u32>,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_mut()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let entry = host.add_diet_entry(&description, &meal_type, calories_kcal)?;
        serde_json::to_string(&entry).map_err(|e| e.to_string())
    })?
}

#[command]
pub fn wellfair_sleep_analytics(app: AppHandle) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let (debt, heatmap) = host.default_sleep_analytics()?;
        let out = serde_json::json!({ "debt": debt, "heatmap": heatmap });
        serde_json::to_string(&out).map_err(|e| e.to_string())
    })?
}

#[command]
pub fn wellfair_add_emergency_contact(
    app: AppHandle,
    display_name: String,
    relationship: String,
    phone: Option<String>,
    email: Option<String>,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let contact = host.add_emergency_contact(&display_name, &relationship, phone, email, None)?;
        serde_json::to_string(&contact).map_err(|e| e.to_string())
    })?
}

#[command]
pub fn wellfair_list_emergency_contacts(app: AppHandle) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let contacts = host.list_emergency_contacts()?;
        serde_json::to_string(&contacts).map_err(|e| e.to_string())
    })?
}


