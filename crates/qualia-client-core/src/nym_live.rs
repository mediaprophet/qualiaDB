//! Live Nym mixnet client bind (official `nym-sdk`).
//!
//! Settings/Wallet enable hits this path. Live only after connect succeeds and a
//! real Recipient address is claimed into identity key `nym`. No fabricated n1….
//! Default network: sandbox (matches historic `NymConfig`). Override with
//! `network` arg or `WEBIZEN_NYM_NETWORK=sandbox|mainnet`.
//!
//! Identity seed is KeyVault-held under closed keyRole `mixnet`
//! (≠ `session-authentication`). Host requests seed at enable; agents request
//! purpose grants only — never hold material.

#![cfg(not(target_arch = "wasm32"))]

use crate::api::{read_identity, save_identity};
use crate::state::{app_meta_dir, APP_STATE};
use nym_crypto::hkdf::DerivationMaterial;
use nym_sdk::mixnet::{MixnetClientBuilder, NymNetworkDetails, StoragePaths};
use qualia_core_db::key_vault::{KeyRole, KeyRoleGrant};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::{Mutex, OnceLock};
use tokio::sync::{mpsc, oneshot};

const MIXNET_DERIVATION_SALT: &[u8] = b"webizen:keyRole:mixnet:nym-sdk:v1";
const MIXNET_DERIVATION_INDEX: u32 = 0;

#[derive(Debug, Clone, Serialize)]
pub struct NymLiveStatus {
    pub live: bool,
    pub enabled_wanted: bool,
    pub address: String,
    pub network: String,
    pub error: Option<String>,
    pub client: &'static str,
    /// Closed keyRole for this client identity seed.
    pub key_role: &'static str,
    /// Where the mixnet seed lives — never the agent.
    pub seed_holder: &'static str,
}

struct Runtime {
    stop_tx: Option<mpsc::Sender<()>>,
    address: String,
    network: String,
    last_error: Option<String>,
}

static RUNTIME: OnceLock<Mutex<Runtime>> = OnceLock::new();

fn runtime() -> &'static Mutex<Runtime> {
    RUNTIME.get_or_init(|| {
        Mutex::new(Runtime {
            stop_tx: None,
            address: String::new(),
            network: String::new(),
            last_error: None,
        })
    })
}

fn resolve_network(requested: Option<&str>) -> Result<(String, NymNetworkDetails), String> {
    let raw = requested
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .or_else(|| std::env::var("WEBIZEN_NYM_NETWORK").ok())
        .unwrap_or_else(|| "sandbox".to_string());
    let key = raw.to_ascii_lowercase();
    match key.as_str() {
        "sandbox" | "sandbox-testnet" | "testnet" => Ok((
            "sandbox".into(),
            nym_network_defaults::sandbox::network_details(),
        )),
        "mainnet" | "main" => Ok(("mainnet".into(), NymNetworkDetails::new_mainnet())),
        other => Err(format!(
            "Nym network '{other}' not approved — use sandbox or mainnet (or WEBIZEN_NYM_NETWORK)"
        )),
    }
}

/// Vault-scoped client dir so first connect derives from KeyVault mixnet seed
/// (orphan PEMs from pre-vault tips must not win over keyRole material).
fn client_dir(network: &str) -> PathBuf {
    app_meta_dir()
        .join("nym-client")
        .join(network)
        .join("kr-mixnet-v1")
}

fn claim_nym_address(address: &str) -> Result<(), String> {
    if address.is_empty() || address.starts_with("n1") {
        return Err("refusing empty or fabricated n1… Nym locator".into());
    }
    let mut id = read_identity().unwrap_or_else(|| serde_json::json!({}));
    if let Some(obj) = id.as_object_mut() {
        obj.insert("nym".into(), serde_json::Value::String(address.to_string()));
        obj.remove("nym_mixnet");
    }
    save_identity(id)
}

fn clear_nym_claim() {
    if let Some(mut id) = read_identity() {
        if let Some(obj) = id.as_object_mut() {
            obj.insert("nym".into(), serde_json::Value::String(String::new()));
            obj.remove("nym_mixnet");
            let _ = save_identity(id);
        }
    }
}

/// Host-only: request mixnet seed from KeyVault and build Nym DerivationMaterial.
/// Seed bytes never leave this function into logs, status, or agent grants.
fn request_mixnet_derivation() -> Result<DerivationMaterial, String> {
    let state = APP_STATE
        .get()
        .ok_or("APP_STATE not initialized — cannot request KeyVault mixnet seed")?;
    let vault = state
        .key_vault
        .lock()
        .map_err(|_| "KeyVault lock poisoned".to_string())?;
    if vault.is_locked() {
        return Err("KeyVault locked — unlock before enabling Nym mixnet".into());
    }
    // Purpose tag mixnet ≠ session-authentication (hard-negative).
    let _session_ctx = KeyRole::SessionAuthentication.vault_context();
    let mix_ctx = KeyRole::Mixnet.vault_context();
    debug_assert_ne!(_session_ctx, mix_ctx);
    let seed = vault.request_role_seed(KeyRole::Mixnet)?;
    Ok(DerivationMaterial::new(
        seed,
        MIXNET_DERIVATION_INDEX,
        MIXNET_DERIVATION_SALT,
    ))
}

/// Agent-facing: request mixnet keyRole purpose — never returns seed.
pub fn agent_request_mixnet_key_role() -> Result<KeyRoleGrant, String> {
    let state = APP_STATE
        .get()
        .ok_or("APP_STATE not initialized")?;
    let vault = state
        .key_vault
        .lock()
        .map_err(|_| "KeyVault lock poisoned".to_string())?;
    Ok(vault.agent_request_role(KeyRole::Mixnet))
}

/// Agent-facing: request any closed keyRole — never returns seed.
pub fn agent_request_key_role(role: &str) -> Result<KeyRoleGrant, String> {
    let role = KeyRole::parse(role).ok_or_else(|| {
        format!("keyRole '{role}' held / not-yet — unknown roles are not other-id")
    })?;
    let state = APP_STATE
        .get()
        .ok_or("APP_STATE not initialized")?;
    let vault = state
        .key_vault
        .lock()
        .map_err(|_| "KeyVault lock poisoned".to_string())?;
    Ok(vault.agent_request_role(role))
}

pub fn nym_live_status() -> NymLiveStatus {
    let enabled_wanted = APP_STATE
        .get()
        .map(|s| s.nym_relay_active.load(Ordering::Relaxed))
        .unwrap_or(false);
    let g = runtime().lock().unwrap();
    let live = enabled_wanted && g.stop_tx.is_some() && !g.address.is_empty();
    NymLiveStatus {
        live,
        enabled_wanted,
        address: g.address.clone(),
        network: g.network.clone(),
        error: g.last_error.clone(),
        client: "nym-sdk@1.21.6",
        key_role: KeyRole::Mixnet.as_str(),
        seed_holder: "keyvault",
    }
}

/// Enable real Nym client. Seed from KeyVault keyRole mixnet; fails honestly if
/// vault locked or connect/claim does not succeed.
pub async fn enable_nym(network: Option<String>) -> Result<NymLiveStatus, String> {
    // Stop any prior session first.
    let _ = disable_nym().await;

    let (net_name, net_details) = resolve_network(network.as_deref())?;
    let derivation = request_mixnet_derivation()?;

    let dir = client_dir(&net_name);
    std::fs::create_dir_all(&dir).map_err(|e| format!("nym client dir: {e}"))?;
    let storage_paths = StoragePaths::new_from_dir(&dir).map_err(|e| format!("nym storage: {e}"))?;

    log::info!(
        "Nym: connecting via nym-sdk ({net_name}) keyRole=mixnet seed_holder=keyvault storage={}",
        dir.display()
    );

    let mut builder = MixnetClientBuilder::new_with_default_storage(storage_paths)
        .await
        .map_err(|e| format!("nym builder storage: {e}"))?
        .network_details(net_details)
        .with_derivation_material(derivation);

    if net_name == "mainnet" {
        // Mainnet bandwidth requires credentials; surface failures honestly.
        builder = builder.enable_credentials_mode();
    }

    let disconnected = builder
        .build()
        .map_err(|e| format!("nym build: {e}"))?;

    let client = disconnected
        .connect_to_mixnet()
        .await
        .map_err(|e| format!("nym connect ({net_name}): {e}"))?;

    let address = client.nym_address().to_string();
    if address.is_empty() || address.starts_with("n1") {
        client.disconnect().await;
        return Err("nym connect returned empty/fabricated address — not claiming".into());
    }

    claim_nym_address(&address)?;

    let (stop_tx, mut stop_rx) = mpsc::channel::<()>(1);
    let (ready_tx, ready_rx) = oneshot::channel::<()>();

    tokio::spawn(async move {
        let mut client = client;
        let _ = ready_tx.send(());
        loop {
            tokio::select! {
                _ = stop_rx.recv() => {
                    client.disconnect().await;
                    break;
                }
                msgs = client.wait_for_messages() => {
                    if msgs.is_none() {
                        break;
                    }
                    // Drain; QFrame routing is a later tip — bind+claim is this tip.
                }
            }
        }
    });

    let _ = ready_rx.await;

    if let Some(state) = APP_STATE.get() {
        state.nym_relay_active.store(true, Ordering::Relaxed);
    }

    {
        let mut g = runtime().lock().unwrap();
        g.stop_tx = Some(stop_tx);
        g.address = address;
        g.network = net_name;
        g.last_error = None;
    }

    log::info!("Nym: live — address claimed (keyRole=mixnet, seed in KeyVault)");
    Ok(nym_live_status())
}

pub async fn disable_nym() -> Result<NymLiveStatus, String> {
    let stop = {
        let mut g = runtime().lock().unwrap();
        g.stop_tx.take()
    };
    if let Some(tx) = stop {
        let _ = tx.send(()).await;
        // Brief yield so disconnect can run.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    if let Some(state) = APP_STATE.get() {
        state.nym_relay_active.store(false, Ordering::Relaxed);
    }
    clear_nym_claim();
    {
        let mut g = runtime().lock().unwrap();
        g.address.clear();
        g.last_error = None;
    }
    Ok(nym_live_status())
}

/// Toggle: enable sandbox (or WEBIZEN_NYM_NETWORK) when off; disable when on.
pub async fn toggle_nym_relay() -> Result<bool, String> {
    let currently = APP_STATE
        .get()
        .map(|s| s.nym_relay_active.load(Ordering::Relaxed))
        .unwrap_or(false);
    if currently {
        disable_nym().await?;
        Ok(false)
    } else {
        match enable_nym(None).await {
            Ok(st) => Ok(st.live),
            Err(e) => {
                if let Some(state) = APP_STATE.get() {
                    state.nym_relay_active.store(false, Ordering::Relaxed);
                }
                {
                    let mut g = runtime().lock().unwrap();
                    g.last_error = Some(e.clone());
                    g.stop_tx = None;
                    g.address.clear();
                }
                clear_nym_claim();
                Err(e)
            }
        }
    }
}

/// Compile-time / unit helper: mixnet purpose context ≠ session-authentication.
#[cfg(test)]
mod purpose_sep_tests {
    use super::*;
    use qualia_core_db::key_vault::KeyVault;

    #[test]
    fn mixnet_key_role_is_not_session_authentication() {
        assert_ne!(
            KeyRole::Mixnet.as_str(),
            KeyRole::SessionAuthentication.as_str()
        );
        assert_ne!(
            KeyRole::Mixnet.vault_context(),
            KeyRole::SessionAuthentication.vault_context()
        );
        assert_eq!(KeyRole::Mixnet.as_str(), "mixnet");
    }

    #[test]
    fn derivation_salt_names_mixnet_not_session() {
        let salt = String::from_utf8_lossy(MIXNET_DERIVATION_SALT);
        assert!(salt.contains("mixnet"));
        assert!(!salt.contains("session-authentication"));
    }

    #[test]
    fn locked_vault_refuses_seed() {
        let mut vault = KeyVault::new();
        vault.lock();
        assert!(vault.request_role_seed(KeyRole::Mixnet).is_err());
        let g = vault.agent_request_role(KeyRole::Mixnet);
        assert!(g.vault_locked);
        assert!(!g.material_held_by_agent);
    }
}
