//! Wallet, identity, tokens, tax, send, vault/federated

#![allow(non_snake_case)]

use qualia_client_core::api;
use qualia_client_core::api::{CoinBalance, SendPreview, TokenEntry, TxRecord, WalletStatus};
use qualia_core_db::ilp_dispatcher::DispatchResult;
use qualia_core_db::rpc::TaxRecipientSuite;
use tauri::command;

// â”€â”€ Wallet / identity â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[command]
pub fn get_wallet_status() -> WalletStatus {
    api::get_wallet_status()
}

#[command]
pub fn is_first_run() -> bool {
    api::is_first_run()
}

#[command]
pub fn read_identity() -> Option<serde_json::Value> {
    api::read_identity()
}

#[command]
pub fn save_identity(wallets: serde_json::Value) -> Result<(), String> {
    api::save_identity(wallets)
}

#[command]
pub fn load_identity() -> Result<Option<serde_json::Value>, String> {
    api::load_identity()
}

#[command]
pub fn get_coin_balances() -> Vec<CoinBalance> {
    api::get_coin_balances()
}

#[command]
pub fn get_transaction_history(ticker: String) -> Vec<TxRecord> {
    api::get_transaction_history(ticker)
}

#[command]
pub async fn generate_bip39_seed() -> Result<String, String> {
    api::generate_bip39_seed().await
}

#[command]
pub async fn derive_wallets_from_seed(seed: String) -> Result<serde_json::Value, String> {
    api::derive_wallets_from_seed(seed).await
}

#[command]
pub async fn import_external_seed(
    network: String,
    seed: String,
    label: String,
) -> Result<String, String> {
    api::import_external_seed(network, seed, label).await
}

// â”€â”€ Tokens â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[command]
pub fn get_tokens() -> Vec<TokenEntry> {
    api::get_tokens()
}

#[command]
pub fn add_token(
    chain: String,
    token_type: String,
    contract: String,
    symbol: String,
    name: String,
    decimals: u8,
) -> Result<TokenEntry, String> {
    api::add_token(chain, token_type, contract, symbol, name, decimals)
}

#[command]
pub fn remove_token(id: String) -> Result<(), String> {
    api::remove_token(id)
}

// â”€â”€ Tax / ILP â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[command]
pub fn get_tax_suite() -> TaxRecipientSuite {
    api::get_tax_suite()
}

#[command]
pub fn save_tax_suite(suite: TaxRecipientSuite) -> Result<(), String> {
    api::save_tax_suite(suite)
}

#[command]
pub fn dispatch_tax_payment(gross_amount_micro_cents: u64) -> Result<DispatchResult, String> {
    api::dispatch_tax_payment(gross_amount_micro_cents)
}

// â”€â”€ Wallet send â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[command]
pub fn build_send_xec(destination_address: String, amount_sats: i64) -> Result<SendPreview, String> {
    api::build_send_xec(&destination_address, amount_sats)
}

#[command]
pub fn confirm_send_xec(raw_hex: String) -> Result<String, String> {
    api::confirm_send_xec(&raw_hex)
}

#[command]
pub fn send_ecash_token(token_id: String, destination_address: String, amount: u64) -> Result<String, String> {
    api::send_ecash_token(&token_id, &destination_address, amount)
}

// â”€â”€ Vault / federated â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[command]
pub fn accept_vault_handshake(did_key: String, payload: String) -> Result<String, String> {
    api::accept_vault_handshake(did_key, payload)
}

#[command]
pub fn receive_vault_job(
    job_id: String,
    task_type: String,
    data_blob_cbor_ld: Vec<u8>,
) -> Result<String, String> {
    api::receive_vault_job(job_id, task_type, data_blob_cbor_ld)
}

