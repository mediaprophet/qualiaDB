//! Nym mixnet adapter surface for core-db.
//!
//! Historical demo/mock path removed. Live enable/bind lives in
//! `qualia_client_core::nym_live` (official `nym-sdk`). This module keeps the
//! config shape and fails closed unless a real client reports live.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NymConfig {
    /// When true, prefer sandbox (default). Production uses mainnet + credentials.
    pub is_demo_mode: bool,
    pub mixnet_proxy_port: u16,
    pub active_network: String,
}

impl Default for NymConfig {
    fn default() -> Self {
        Self {
            is_demo_mode: true,
            mixnet_proxy_port: 1080,
            active_network: "sandbox".to_string(),
        }
    }
}

/// Former mock initializer — no longer simulates a SOCKS5 bind.
/// Callers must use `qualia_client_core::nym_live::enable_nym` for a real client.
pub async fn initialize_nym_proxy(config: &NymConfig) -> Result<(), String> {
    Err(format!(
        "nym_adapter::initialize_nym_proxy is not a live bind (network={}). \
         Use Wallet/Settings enable → qualia_client_core::nym_live (nym-sdk).",
        config.active_network
    ))
}

/// Former mock Sphinx dispatch — fails closed. Real routing is a follow-up once
/// the mixnet client is live and claimed.
pub async fn route_through_mixnet(_payload: &[u8]) -> Result<Vec<u8>, String> {
    Err(
        "nym_adapter::route_through_mixnet has no live path — \
         mixnet client claim+bind required (no simulated MIXNET_RESPONSE_OK)"
            .into(),
    )
}
