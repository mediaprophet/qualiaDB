//! QApp vault, config, wallet status

#![allow(non_snake_case)]

use super::*;

use crate::state::*;
use crate::qapp_paths::qapps_dir;
use crate::qapp_registry;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use sysinfo::Disks;


pub fn list_installed_qapps() -> Vec<String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let data_dir = state.config.lock().unwrap().storage_path.clone();
    let qapps_dir = qapps_dir(&data_dir);
    let mut qapps = Vec::new();
    if let Ok(entries) = std::fs::read_dir(qapps_dir) {
        for entry in entries.filter_map(Result::ok) {
            if entry.path().is_dir() {
                qapps.push(entry.file_name().to_string_lossy().to_string());
            }
        }
    }
    qapps
}

pub fn mcp_list_qapps() -> Result<String, String> {
    crate::qapp_mcp::list_qapp_catalogue_json()
}

pub fn mcp_get_qapp_manifest(qapp_name: String) -> Result<String, String> {
    crate::qapp_mcp::get_qapp_manifest_json(&qapp_name)
}

pub fn mcp_inspect_qapp_readiness(qapp_name: String) -> Result<String, String> {
    crate::qapp_mcp::inspect_qapp_readiness_json(&qapp_name)
}

pub fn mcp_list_qapp_updates() -> Result<String, String> {
    crate::qapp_mcp::list_qapp_updates_json()
}

pub fn mcp_describe_qapp_surface_schema() -> Result<String, String> {
    crate::qapp_mcp::describe_qapp_surface_schema_json()
}

pub fn generate_qapp_credential(qapp_name: String) -> String {
    format!("did:qualia:qapp:{}:signed_vc", qapp_name)
}

pub fn verify_and_install_qapp(target_path: String) -> Result<String, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let storage_path = std::path::PathBuf::from(&storage);

    if let Ok(port) = target_path.parse::<u16>() {
        return Err(format!(
            "Dev proxy port registration ({port}) requires a package directory path, not a bare port"
        ));
    }

    let source_dir = std::path::PathBuf::from(&target_path);
    if !source_dir.is_dir() {
        return Err(format!("Qapp source directory not found: {target_path}"));
    }

    let entry = crate::qapp_install::install_package_atomic(
        &storage_path,
        &source_dir,
        crate::qapp_install::InstallPolicy::Development,
        None,
    )
    .map_err(|e| e.to_string())?;

    let package_dir = crate::qapp_paths::resolve_active_package_dir(&storage_path, &entry.package_id);
    let manifest = load_qapp_package_from_dir(&package_dir)?;
    let qapp_did = format!(
        "did:qualia:qapp:{}",
        manifest.name.to_lowercase().replace(" ", "-")
    );

    let registered_qapp = qapp_registry::RegisteredQapp {
        did: qapp_did.clone(),
        manifest: manifest.clone(),
        target: qapp_registry::QappTarget::IsolatedVault(entry.package_id.clone()),
    };

    let qapp_id_hash = crate::qapp_manifest::install_qapp_capabilities(&manifest)
        .map_err(|e| format!("Qapp capability compile failed: {e:?}"))?;

    {
        let mut installed = state.installed_qapps.lock().unwrap();
        installed.retain(|q| {
            q.manifest.name != entry.package_id && q.did != qapp_did
        });
        installed.push(registered_qapp);
    }
    save_directory_state();

    Ok(format!(
        "{qapp_did} v{} hash={} content={}",
        entry.active_version, qapp_id_hash, entry.content_hash
    ))
}

#[derive(Serialize)]
pub struct WalletStatus {
    /// XEC balance in satoshis (live from Chronik when identity is set).
    pub xec_sats: i64,
    /// Total ILP micro-cents dispatched (from persistent ledger).
    pub ilp_dispatched_microcents: u64,
    /// Whether the Nym mixnet relay is active.
    pub nym_connected: bool,
    /// Sync status: "synced" | "offline" | "no_identity".
    pub sync_status: String,
}

pub fn get_wallet_status() -> WalletStatus {
    let nym_connected = crate::state::APP_STATE
        .get()
        .map(|s| s.nym_relay_active.load(Ordering::Relaxed))
        .unwrap_or(false);

    let state = crate::state::APP_STATE.get();
    let storage_path = state
        .map(|s| s.config.lock().unwrap().storage_path.clone())
        .unwrap_or_default();

    // Read ILP dispatched total from persistent ledger
    let ilp_dispatched = crate::wallet::ledger::total_ilp_sent_micro_cents(
        &std::path::Path::new(&storage_path),
    );

    // Query live XEC balance from Chronik if identity is set
    let id = read_identity();
    let (xec_sats, sync_status) = match id
        .as_ref()
        .and_then(|v| v.get("ecash_hash160"))
        .and_then(|v| v.as_str())
    {
        Some(hash160) => {
            let client = crate::wallet::chronik::ChronikClient::new("https://chronik.be.cash");
            match client.fetch_utxos_p2pkh(hash160) {
                Ok(utxos) => {
                    let sats: i64 = utxos
                        .iter()
                        .filter(|u| u.slp_meta.is_none())
                        .map(|u| u.value)
                        .sum();
                    (sats, "synced".to_string())
                }
                Err(_) => (0, "offline".to_string()),
            }
        }
        None => (0, "no_identity".to_string()),
    };

    WalletStatus {
        xec_sats,
        ilp_dispatched_microcents: ilp_dispatched,
        nym_connected,
        sync_status,
    }
}

pub fn nym_mixnet_opted_in() -> bool {
    crate::state::APP_STATE
        .get()
        .map(|s| s.nym_relay_active.load(Ordering::Relaxed))
        .unwrap_or(false)
}

pub fn get_config() -> AgentConfig {
    let state = crate::state::APP_STATE.get().unwrap();
    state.config.lock().unwrap().clone()
}

pub fn save_config(new_config: AgentConfig) -> Result<(), String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let disks = Disks::new_with_refreshed_list();
    let path = PathBuf::from(&new_config.storage_path);
    let mut available = u64::MAX;
    for disk in disks.list() {
        if path.starts_with(disk.mount_point()) {
            available = disk.available_space();
            break;
        }
    }
    let margin: u64 = 15 * 1024 * 1024 * 1024;
    let requested = new_config.storage_quota_gb * 1024 * 1024 * 1024;
    if available.saturating_sub(requested) < margin {
        return Err(
            "OS_SAFETY_VIOLATION: Would leave the host OS with less than the 15 GB safety margin."
                .to_string(),
        );
    }
    // Persist config to disk
    let meta = app_meta_dir();
    std::fs::create_dir_all(&meta).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(&new_config).map_err(|e| e.to_string())?;
    std::fs::write(config_file_path(), json).map_err(|e| e.to_string())?;
    // Ensure data directories exist under the new path
    init_data_directories(&new_config.storage_path);
    // Mirror inference_backend string into structured settings (Local/Remote/Hybrid/Ollama).
    let mut ib = crate::inference_backend::load_inference_backend_settings();
    ib.apply_agent_config_backend_string(&new_config.inference_backend);
    let _ = crate::inference_backend::save_inference_backend_settings(&ib);
    *state.config.lock().unwrap() = new_config;
    Ok(())
}

