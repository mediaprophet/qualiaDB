//! Hardware, daemon, config

#![allow(non_snake_case)]

use qualia_client_core::api;
use qualia_client_core::api::HardwareStatus;
use qualia_client_core::setup::{SetupProfile, SetupState};
use qualia_client_core::state::AgentConfig;
use tauri::command;

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
pub fn finish_setup() -> Result<SetupState, String> {
    api::finish_setup()
}
