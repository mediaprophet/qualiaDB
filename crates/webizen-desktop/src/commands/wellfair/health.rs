#![allow(non_snake_case)]

use super::super::*;
use super::*;
use tauri::{command, AppHandle, Manager, State};

#[command]
pub fn wellfair_host_snapshot(
    app_state: State<'_, std::sync::Arc<qualia_client_core::state::AppState>>,
    host_state: State<'_, HostApiState>,
) -> Result<String, String> {
    let app_state_arc = std::sync::Arc::clone(&*app_state);
    host_state.0.execute_sync(move |guard| {
        let kv = app_state_arc.key_vault.lock().map_err(|e| e.to_string())?;
        let owner_label = api::read_identity()
            .and_then(|v| {
                v.get("display_name")
                    .and_then(|n| n.as_str())
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| "Owner vault".to_string());
        let storage_root = std::path::PathBuf::from(
            app_state_arc.config.lock().map_err(|e| e.to_string())?.storage_path.clone(),
        );
        let snapshot = if let Some(host) = guard.as_mut() {
            host.build_snapshot(&kv, &owner_label)
        } else {
            qualia_client_core::wellfair::snapshot::build_host_snapshot_with_storage(
                &kv,
                false,
                &owner_label,
                false,
                Some(&storage_root),
            )
        };
        serde_json::to_string(&snapshot).map_err(|e| e.to_string())
    })?
}

#[command]
pub fn wellfair_save_accessibility(
    app: AppHandle,
    prefs_json: String,
) -> Result<String, String> {
    let prefs: qualia_client_core::wellfair::host_state::AccessibilityPreferences =
        serde_json::from_str(&prefs_json).map_err(|e| format!("invalid prefs JSON: {e}"))?;
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        host.save_accessibility(&prefs)?;
        serde_json::to_string(&prefs).map_err(|e| e.to_string())
    })?
}

#[command]
pub fn wellfair_list_health_records(
    app: AppHandle,
    limit: Option<usize>,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let records = host.list_health_records(limit.unwrap_or(64))?;
        serde_json::to_string(&records).map_err(|e| e.to_string())
    })?
}

#[command]
pub fn wellfair_list_receipts(app: AppHandle, limit: Option<usize>) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let receipts = host.list_receipts(limit.unwrap_or(32))?;
        serde_json::to_string(&receipts).map_err(|e| e.to_string())
    })?
}

#[command]
pub fn wellfair_export_health_package(
    app: AppHandle,
    limit: Option<usize>,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_mut()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let (package, receipt) = host.export_health_package(limit.unwrap_or(256))?;
        serde_json::to_string(&serde_json::json!({
            "package": package,
            "receipt": receipt,
        }))
        .map_err(|e| e.to_string())
    })?
}

#[command]
pub fn wellfair_import_samsung_folder(
    app: AppHandle,
    folder_path: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_mut()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let report = host.import_samsung_health_folder(std::path::Path::new(&folder_path));
        serde_json::to_string(&report).map_err(|e| e.to_string())
    })?
}

#[command]
pub fn wellfair_companion_pairing() -> Result<String, String> {
    let port = crate::companion_gateway::companion_listen_port();
    let info = crate::companion_gateway::companion_pairing_info(port);
    serde_json::to_string(&info).map_err(|e| e.to_string())
}

#[command]
pub fn wellfair_ingest_companion_health(
    app: AppHandle,
    bundle_json: String,
) -> Result<String, String> {
    let bundle: wellfare_core::companion_sync::CompanionHealthBundle =
        serde_json::from_str(&bundle_json).map_err(|e| format!("invalid bundle JSON: {e}"))?;
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_mut()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let report = host.ingest_companion_health_bundle(&bundle);
        serde_json::to_string(&report).map_err(|e| e.to_string())
    })?
}

