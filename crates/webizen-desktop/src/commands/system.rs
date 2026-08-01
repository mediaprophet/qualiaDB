//! Hardware, daemon, config

#![allow(non_snake_case)]

use qualia_client_core::api;
use qualia_client_core::api::HardwareStatus;
use qualia_client_core::setup::{SetupProfile, SetupState};
use qualia_client_core::state::AgentConfig;
use tauri::command;
use tauri::Manager;

// ── Hardware / system ─────────────────────────────────────────────────────────

#[command]
pub fn get_hardware_status() -> HardwareStatus {
    api::get_hardware_status()
}

#[command]
pub fn profile_energy_circumstance() -> String {
    api::profile_energy_circumstance()
}

// ── Daemon ────────────────────────────────────────────────────────────────────

#[command]
pub fn start_daemon() -> String {
    api::start_daemon()
}

#[command]
pub fn daemon_status() -> String {
    api::daemon_status()
}

#[command]
pub fn get_active_daemon_port() -> u16 {
    api::get_active_daemon_port()
}

#[command]
pub fn qualia_protocol_port() -> u16 {
    api::qualia_protocol_port()
}

#[command]
pub fn run_engine_command(cmd: String) -> String {
    api::run_engine_command(cmd)
}

// ── Config ────────────────────────────────────────────────────────────────────

#[command]
pub fn get_config() -> AgentConfig {
    api::get_config()
}

#[command]
pub fn save_config(new_config: AgentConfig) -> Result<(), String> {
    api::save_config(new_config)
}

#[command]
pub fn get_setup_state() -> Result<SetupState, String> {
    api::get_setup_state()
}

#[command]
pub fn complete_setup_step(step: String) -> Result<SetupState, String> {
    api::complete_setup_step(step)
}

#[command]
pub fn update_setup_profile(profile: SetupProfile) -> Result<SetupState, String> {
    api::update_setup_profile(profile)
}

#[command]
pub fn finish_setup(app: tauri::AppHandle) -> Result<SetupState, String> {
    let state = api::finish_setup()?;
    // First-run defers the optional WGPU diffusion runtime so a driver fault cannot
    // kill the setup window. Start it now that foundations are saved.
    if let Some(app_state) =
        app.try_state::<std::sync::Arc<qualia_client_core::state::AppState>>()
    {
        crate::runtime::ensure_runtime_started(&app, app_state.inner().clone());
    }
    Ok(state)
}

// ── Person / apparatus identity (person ≠ machine ≠ OS account) ───────────────

#[command]
pub fn get_identity_plane() -> Result<qualia_client_core::identity_plane::IdentityPlaneSnapshot, String>
{
    api::get_identity_plane()
}

#[command]
pub fn list_apparatus_devices(
) -> Result<Vec<qualia_client_core::identity_plane::DeviceRecordPublic>, String> {
    api::list_apparatus_devices()
}

#[command]
pub fn export_person_public() -> Result<qualia_client_core::identity_plane::PersonPublic, String> {
    api::export_person_public()
}

#[command]
pub fn export_person_transfer_bundle(
) -> Result<qualia_client_core::identity_plane::PersonTransferBundle, String> {
    api::export_person_transfer_bundle()
}

#[command]
pub fn import_person_transfer_bundle(
    bundle: qualia_client_core::identity_plane::PersonTransferBundle,
) -> Result<qualia_client_core::identity_plane::IdentityPlaneSnapshot, String> {
    api::import_person_transfer_bundle(bundle)
}

#[command]
pub fn register_remote_apparatus_device(
    device: qualia_client_core::identity_plane::DeviceRecordPublic,
) -> Result<qualia_client_core::identity_plane::IdentityPlaneSnapshot, String> {
    api::register_remote_apparatus_device(device)
}

#[command]
pub fn resolve_job_device_placement(
    target_device_id: Option<String>,
) -> Result<qualia_client_core::identity_plane::JobPlacement, String> {
    api::resolve_job_device_placement(target_device_id)
}

#[command]
pub fn schedule_job_on_device(
    kind_json: String,
    target_device_id: Option<String>,
) -> Result<serde_json::Value, String> {
    api::schedule_job_on_device(kind_json, target_device_id)
}

#[command]
pub fn set_local_control_base_url(
    url: String,
) -> Result<qualia_client_core::identity_plane::IdentityPlaneSnapshot, String> {
    api::set_local_control_base_url(url)
}

#[command]
pub fn list_remote_job_outbox(
) -> Result<Vec<qualia_client_core::identity_plane::RemoteOutboxEntry>, String> {
    api::list_remote_job_outbox()
}

#[command]
pub fn retry_remote_job_outbox() -> Result<usize, String> {
    api::retry_remote_job_outbox()
}

#[command]
pub fn mint_person_webid_tls_cert() -> Result<serde_json::Value, String> {
    api::mint_person_webid_tls_cert()
}

#[command]
pub fn accept_fleet_job_envelope(
    envelope: qualia_client_core::identity_plane::FleetJobEnvelope,
) -> Result<serde_json::Value, String> {
    api::accept_fleet_job_envelope(envelope)
}
