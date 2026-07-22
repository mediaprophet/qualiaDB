//! Daemon port + app session tokens

#![allow(non_snake_case)]

use super::*;

use crate::qapp_paths::qapps_dir;
use std::sync::atomic::{AtomicU16, Ordering};


static ACTIVE_DAEMON_PORT: AtomicU16 = AtomicU16::new(0);

/// Records the loopback port chosen when the graph daemon last started.
pub fn set_active_daemon_port(port: u16) {
    ACTIVE_DAEMON_PORT.store(port, Ordering::SeqCst);
}

/// Returns the active daemon port when known, otherwise the configured default.
pub fn get_active_daemon_port() -> u16 {
    let active = ACTIVE_DAEMON_PORT.load(Ordering::SeqCst);
    if active != 0 {
        return active;
    }
    crate::state::APP_STATE
        .get()
        .map(|state| state.config.lock().unwrap().daemon_port)
        .unwrap_or(4242)
}

/// Issues a signed semantic app token scoped to the installed app's manifest shapes.
pub fn build_anatomy_graph_context_json(
    qapp_name: String,
    user_prompt: String,
    agent_reply: String,
) -> Result<String, String> {
    crate::anatomy_context::build_anatomy_graph_context_json(qapp_name, user_prompt, agent_reply)
}

pub fn build_anatomy_graph_context_json_with_dicom(
    qapp_name: String,
    user_prompt: String,
    agent_reply: String,
    dicom_file_path: Option<String>,
) -> Result<String, String> {
    crate::anatomy_context::build_anatomy_graph_context_json_with_dicom(
        qapp_name,
        user_prompt,
        agent_reply,
        dicom_file_path,
    )
}

pub fn parse_dicom_metadata_json(file_path: String) -> Result<String, String> {
    crate::anatomy_context::parse_dicom_metadata_json(file_path)
}

pub fn build_dicom_overlay_spec_json(file_path: String) -> Result<String, String> {
    crate::anatomy_context::build_dicom_overlay_spec_json(file_path)
}

pub fn submit_dicom_ingest(file_path: String, patient_did_hash: u64) -> Result<u64, String> {
    crate::qapp_api::submit_dicom_ingest(file_path, patient_did_hash)
}

pub fn dicom_ingest_status(job_id: u64) -> u8 {
    crate::qapp_api::dicom_ingest_status(job_id)
}

pub fn execute_dicom_volume_query(
    patient_did_hash: u64,
    series_hash: u64,
) -> Result<Vec<u8>, String> {
    crate::qapp_api::execute_dicom_volume_query(patient_did_hash, series_hash)
}

pub fn issue_qapp_session_token(qapp_name: &str) -> Result<String, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let data_dir = state.config.lock().unwrap().storage_path.clone();
    let qapp_dir = qapps_dir(&data_dir).join(qapp_name);
    if !qapp_dir.exists() {
        return Err(format!("Qapp directory not found: {qapp_name}"));
    }

    let manifest = load_qapp_package_from_dir(&qapp_dir)?;
    let qapp_did = format!(
        "did:qualia:qapp:{}",
        manifest.name.to_lowercase().replace(' ', "-")
    );
    let vault = state.key_vault.lock().unwrap();
    vault.issue_qapp_token(
        &qapp_did,
        "localhost",
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() + 86400,
        &uuid::Uuid::new_v4().to_string(),
        manifest.required_shapes.clone(),
        qualia_core_db::identity::key_vault::SubgraphLayer::Professional,
    )
}

