//! Dashboard engine command

#![allow(non_snake_case)]

use super::*;

use crate::state::*;
use crate::qapp_registry;


pub fn run_engine_command(cmd: String) -> String {
    match cmd.as_str() {
        "ingest_bench" => profile_energy_circumstance(),
        "zk_screen" => format!(
            "Daemon: {} | Ollama: {}",
            daemon_status(),
            check_ollama_status()
        ),
        _ => "Unknown command".to_string(),
    }
}

// Tray functionality removed with Tauri

pub fn toggle_window() {
    // No-op without Tauri
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// Agent Directory & Delegation Manager
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct DirectoryState {
    pub actors: Vec<Actor>,
    pub rules: Vec<DelegationRule>,
    pub front_doors: Vec<FrontDoor>,
    pub installed_qapps: Vec<qapp_registry::RegisteredQapp>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct SignedDirectoryState {
    pub state: DirectoryState,
    pub signature_hex: String,
}

pub fn save_directory_state() {
    let state = crate::state::APP_STATE.get().unwrap();
    let home = std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\".to_string());
    let qualia_dir = std::path::PathBuf::from(home).join(".qualia");
    if !qualia_dir.exists() {
        let _ = std::fs::create_dir_all(&qualia_dir);
    }

    let ds = DirectoryState {
        actors: state.directory_actors.lock().unwrap().clone(),
        rules: state.delegation_rules.lock().unwrap().clone(),
        front_doors: state.front_doors.lock().unwrap().clone(),
        installed_qapps: state.installed_qapps.lock().unwrap().clone(),
    };

    let payload = serde_json::to_string(&ds).unwrap();
    let vault = state.key_vault.lock().unwrap();
    // Since we don't have the derived key in scope here easily, we sign with master for persistence.
    // In a real implementation, we'd sign with the specific identity.
    let sig = vault.sign_payload(&vault.derive_key("persistence"), payload.as_bytes());
    let sig_hex = hex::encode(sig.to_bytes());

    let signed_state = SignedDirectoryState {
        state: ds,
        signature_hex: sig_hex,
    };

    let state_path = qualia_dir.join("directory_state.json");
    let _ = std::fs::write(
        &state_path,
        serde_json::to_string_pretty(&signed_state).unwrap(),
    );
}

pub fn load_directory_state(vault: &qualia_core_db::key_vault::KeyVault) -> DirectoryState {
    let home = std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\".to_string());
    let state_path = std::path::PathBuf::from(home)
        .join(".qualia")
        .join("directory_state.json");

    if state_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&state_path) {
            if let Ok(signed_state) = serde_json::from_str::<SignedDirectoryState>(&content) {
                let payload = serde_json::to_string(&signed_state.state).unwrap();
                let sig_bytes = hex::decode(&signed_state.signature_hex).unwrap_or_default();
                if sig_bytes.len() == 64 {
                    let mut sig_arr = [0u8; 64];
                    sig_arr.copy_from_slice(&sig_bytes);
                    let persistence_key = vault.derive_key("persistence");
                    let pk = ed25519_dalek::VerifyingKey::from(&persistence_key);
                    if qualia_core_db::key_vault::KeyVault::verify_signature(
                        pk.as_bytes(),
                        payload.as_bytes(),
                        &sig_arr,
                    )
                    .is_ok()
                    {
                        return signed_state.state;
                    } else {
                        eprintln!("WARNING: directory_state.json signature validation failed! Tampering detected.");
                    }
                }
            }
        }
    }

    DirectoryState {
        actors: Vec::new(),
        rules: Vec::new(),
        front_doors: Vec::new(),
        installed_qapps: Vec::new(),
    }
}

pub fn get_front_doors() -> Result<Vec<FrontDoor>, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let doors = state.front_doors.lock().unwrap().clone();
    Ok(doors)
}

pub fn generate_front_door(label: String) -> Result<FrontDoor, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();

    let vault = state.key_vault.lock().unwrap();
    let fd_id = format!("fd-{}", now);
    let derived_key = vault.derive_key(&fd_id);
    let pub_key_hex = hex::encode(ed25519_dalek::VerifyingKey::from(&derived_key).as_bytes());
    let did_uri = format!("did:qualia:frontdoor:{}", pub_key_hex);

    // Optional: Pre-generate the WebID-TLS cert here if needed
    // let (cert, _) = vault.generate_webid_tls_cert(&derived_key, &did_uri).unwrap();

    let door = FrontDoor {
        id: fd_id,
        did_uri,
        label,
        created_at: now.to_string(),
        routing_hints: vec![],
    };

    drop(vault);
    state.front_doors.lock().unwrap().push(door.clone());
    save_directory_state();
    Ok(door)
}

pub fn get_directory_actors() -> Result<Vec<Actor>, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let actors = state.directory_actors.lock().unwrap().clone();
    Ok(actors)
}

pub fn add_directory_actor(actor: Actor) -> Result<(), String> {
    let state = crate::state::APP_STATE.get().unwrap();
    state.directory_actors.lock().unwrap().push(actor);
    save_directory_state();
    Ok(())
}

pub fn get_delegation_rules() -> Result<Vec<DelegationRule>, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let rules = state.delegation_rules.lock().unwrap().clone();
    Ok(rules)
}

pub fn add_delegation_rule(rule: DelegationRule) -> Result<(), String> {
    let state = crate::state::APP_STATE.get().unwrap();
    state.delegation_rules.lock().unwrap().push(rule);
    save_directory_state();
    Ok(())
}

