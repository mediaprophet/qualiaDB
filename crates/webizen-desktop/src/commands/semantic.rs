//! Semantic web, portfolio, imported accounts

#![allow(non_snake_case)]

use qualia_client_core::api;
use tauri::command;

// ── Semantic web / portfolio ──────────────────────────────────────────────────

#[command]
pub async fn generate_front_door_invite() -> Result<String, String> {
    api::generate_front_door_invite().await
}

#[command]
pub async fn mint_semantic_token(asset_id: String) -> Result<String, String> {
    api::mint_semantic_token(asset_id).await
}

#[command]
pub async fn fetch_wallet_portfolio() -> Result<serde_json::Value, String> {
    api::fetch_wallet_portfolio().await
}

#[command]
pub async fn toggle_nym_relay() -> Result<bool, String> {
    api::toggle_nym_relay().await
}

#[command]
pub async fn toggle_stark_prover() -> Result<bool, String> {
    api::toggle_stark_prover().await
}

#[command]
pub fn update_solar_input(watts: u32) {
    api::update_solar_input(watts)
}

#[command]
pub async fn fetch_torrent_telemetry() -> Result<serde_json::Value, String> {
    api::fetch_torrent_telemetry().await
}

#[command]
pub async fn fetch_remote_manifest(url: String) -> Result<String, String> {
    api::fetch_remote_manifest(url).await
}

// ── Imported accounts ─────────────────────────────────────────────────────────

#[command]
pub fn load_imported_accounts() -> Result<serde_json::Value, String> {
    api::load_imported_accounts()
}

#[command]
pub fn save_imported_accounts(accounts: serde_json::Value) -> Result<(), String> {
    api::save_imported_accounts(accounts)
}

