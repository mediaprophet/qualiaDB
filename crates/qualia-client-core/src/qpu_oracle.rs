//! Sovereign QPU Oracle — BYOK remote quantum offload for NP-hard tasks.
//!
//! API keys are encrypted at rest via KeyVault-derived material. Only anonymized
//! numeric matrices (QUBO / VQE parameter vectors) may egress; classified data
//! is blocked by the Sentinel before any HTTP dispatch.

use crate::state::{app_meta_dir, APP_STATE};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

static QPU_CACHE: Mutex<Option<QpuOracleState>> = Mutex::new(None);

fn qpu_config_path() -> PathBuf {
    app_meta_dir().join("qpu_oracle.json")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QpuArchitecture {
    GateModel,
    Annealer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QpuOracleState {
    pub feature_unlocked: bool,
    pub ibm_token_enc: String,
    pub dwave_token_enc: String,
    pub max_shots_per_task: u32,
    pub fallback_to_classical: bool,
    pub enable_qubo_routing: bool,
    pub enable_dft_ground_state: bool,
    pub enable_defeasible_resolution: bool,
    pub ibm_minutes_used: f64,
    pub dwave_minutes_used: f64,
}

impl Default for QpuOracleState {
    fn default() -> Self {
        Self {
            feature_unlocked: false,
            ibm_token_enc: String::new(),
            dwave_token_enc: String::new(),
            max_shots_per_task: 1000,
            fallback_to_classical: true,
            enable_qubo_routing: true,
            enable_dft_ground_state: true,
            enable_defeasible_resolution: false,
            ibm_minutes_used: 0.0,
            dwave_minutes_used: 0.0,
        }
    }
}

/// Public view returned to Flutter — never exposes raw API tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QpuOracleSettings {
    pub feature_unlocked: bool,
    pub ibm_token_configured: bool,
    pub dwave_token_configured: bool,
    pub max_shots_per_task: u32,
    pub fallback_to_classical: bool,
    pub enable_qubo_routing: bool,
    pub enable_dft_ground_state: bool,
    pub enable_defeasible_resolution: bool,
    pub ibm_quota_minutes_remaining: f64,
    pub dwave_quota_minutes_remaining: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QpuOracleSettingsInput {
    pub max_shots_per_task: u32,
    pub fallback_to_classical: bool,
    pub enable_qubo_routing: bool,
    pub enable_dft_ground_state: bool,
    pub enable_defeasible_resolution: bool,
    /// `None` = leave unchanged; `Some("")` = clear token.
    pub ibm_token: Option<String>,
    pub dwave_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QpuChatCommandResult {
    pub handled: bool,
    pub response: String,
    pub feature_unlocked: bool,
}

const IBM_MONTHLY_MINUTES: f64 = 10.0;
const DWAVE_MONTHLY_MINUTES: f64 = 1.0;

fn encrypt_secret(plaintext: &str) -> Result<String, String> {
    if plaintext.is_empty() {
        return Ok(String::new());
    }
    let state = APP_STATE.get().ok_or("APP_STATE not initialized")?;
    let vault = state.key_vault.lock().map_err(|e| e.to_string())?;
    let key_bytes = vault.derive_key("qpu_oracle_secrets").to_bytes();
    let encrypted: Vec<u8> = plaintext
        .as_bytes()
        .iter()
        .enumerate()
        .map(|(i, b)| b ^ key_bytes[i % 32])
        .collect();
    Ok(hex::encode(encrypted))
}

fn decrypt_secret(ciphertext_hex: &str) -> Result<String, String> {
    if ciphertext_hex.is_empty() {
        return Ok(String::new());
    }
    let state = APP_STATE.get().ok_or("APP_STATE not initialized")?;
    let vault = state.key_vault.lock().map_err(|e| e.to_string())?;
    let encrypted = hex::decode(ciphertext_hex).map_err(|_| "Invalid encrypted token")?;
    let key_bytes = vault.derive_key("qpu_oracle_secrets").to_bytes();
    let decrypted: Vec<u8> = encrypted
        .iter()
        .enumerate()
        .map(|(i, b)| b ^ key_bytes[i % 32])
        .collect();
    String::from_utf8(decrypted).map_err(|_| "Token decryption failed".into())
}

fn load_state_from_disk() -> QpuOracleState {
    std::fs::read_to_string(qpu_config_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn persist_state(state: &QpuOracleState) -> Result<(), String> {
    let meta = app_meta_dir();
    std::fs::create_dir_all(&meta).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(state).map_err(|e| e.to_string())?;
    std::fs::write(qpu_config_path(), json).map_err(|e| e.to_string())?;
    Ok(())
}

pub(crate) fn cached_state_internal() -> QpuOracleState {
    cached_state()
}

pub fn record_usage(arch: QpuArchitecture, minutes: f64) -> Result<(), String> {
    update_state(|state| match arch {
        QpuArchitecture::GateModel => state.ibm_minutes_used += minutes,
        QpuArchitecture::Annealer => state.dwave_minutes_used += minutes,
    })
    .map(|_| ())
}

fn cached_state() -> QpuOracleState {
    let mut cache = QPU_CACHE.lock().unwrap();
    if cache.is_none() {
        *cache = Some(load_state_from_disk());
    }
    cache.clone().unwrap()
}

fn update_state<F: FnOnce(&mut QpuOracleState)>(f: F) -> Result<QpuOracleState, String> {
    let mut state = cached_state();
    f(&mut state);
    persist_state(&state)?;
    *QPU_CACHE.lock().unwrap() = Some(state.clone());
    Ok(state)
}

pub fn to_public_settings(state: &QpuOracleState) -> QpuOracleSettings {
    QpuOracleSettings {
        feature_unlocked: state.feature_unlocked,
        ibm_token_configured: !state.ibm_token_enc.is_empty(),
        dwave_token_configured: !state.dwave_token_enc.is_empty(),
        max_shots_per_task: state.max_shots_per_task,
        fallback_to_classical: state.fallback_to_classical,
        enable_qubo_routing: state.enable_qubo_routing,
        enable_dft_ground_state: state.enable_dft_ground_state,
        enable_defeasible_resolution: state.enable_defeasible_resolution,
        ibm_quota_minutes_remaining: (IBM_MONTHLY_MINUTES - state.ibm_minutes_used).max(0.0),
        dwave_quota_minutes_remaining: (DWAVE_MONTHLY_MINUTES - state.dwave_minutes_used).max(0.0),
    }
}

pub fn get_qpu_settings() -> QpuOracleSettings {
    to_public_settings(&cached_state())
}

pub fn is_qpu_feature_unlocked() -> bool {
    cached_state().feature_unlocked
}

pub fn save_qpu_settings(input: QpuOracleSettingsInput) -> Result<QpuOracleSettings, String> {
    if input.max_shots_per_task == 0 || input.max_shots_per_task > 1000 {
        return Err("max_shots_per_task must be between 1 and 1000 (SHACL bound)".into());
    }
    update_state(|state| {
        state.max_shots_per_task = input.max_shots_per_task;
        state.fallback_to_classical = input.fallback_to_classical;
        state.enable_qubo_routing = input.enable_qubo_routing;
        state.enable_dft_ground_state = input.enable_dft_ground_state;
        state.enable_defeasible_resolution = input.enable_defeasible_resolution;
        if let Some(ref tok) = input.ibm_token {
            state.ibm_token_enc = encrypt_secret(tok).unwrap_or_default();
        }
        if let Some(ref tok) = input.dwave_token {
            state.dwave_token_enc = encrypt_secret(tok).unwrap_or_default();
        }
    })
    .map(|s| to_public_settings(&s))
}

pub fn enable_qpu_feature() -> Result<QpuOracleSettings, String> {
    update_state(|state| state.feature_unlocked = true).map(|s| to_public_settings(&s))
}

pub fn disable_qpu_feature() -> Result<QpuOracleSettings, String> {
    update_state(|state| state.feature_unlocked = false).map(|s| to_public_settings(&s))
}

/// Intercept hidden chat commands before LLM inference.
pub fn handle_qpu_chat_command(text: &str) -> QpuChatCommandResult {
    let normalized = text.trim();
    let enable_cmds = [
        "[enable_QPU]",
        "[enable_QPU}",
        "[enable_qpu]",
        "[enable_qpu}",
    ];
    let disable_cmds = [
        "[disable_QPU]",
        "[disable_QPU}",
        "[disable_qpu]",
        "[disable_qpu}",
    ];

    if enable_cmds
        .iter()
        .any(|c| normalized.eq_ignore_ascii_case(c))
    {
        match enable_qpu_feature() {
            Ok(settings) => QpuChatCommandResult {
                handled: true,
                feature_unlocked: settings.feature_unlocked,
                response: "⚛️ **QPU Oracle unlocked.**\n\n\
                    Open **Settings → QPU Oracle** to configure IBM Quantum and D-Wave Leap API keys (BYOK).\n\n\
                    **Chat commands** (use the ∑ math keyboard to compose LaTeX):\n\
                    - `[qpu:qubo]` or `$$\\min_{x} ...$$` → D-Wave constraint routing\n\
                    - `[qpu:dft]` or `$$\\hat{H}\\Psi = E\\Psi$$` → IBM VQE ground states\n\
                    - `[qpu:defeasible]` → probabilistic obligation resolution\n\n\
                    Only anonymized numeric matrices egress; classified data is blocked."
                    .to_string(),
            },
            Err(e) => QpuChatCommandResult {
                handled: true,
                feature_unlocked: false,
                response: format!("🔴 QPU unlock failed: {e}"),
            },
        }
    } else if disable_cmds
        .iter()
        .any(|c| normalized.eq_ignore_ascii_case(c))
    {
        match disable_qpu_feature() {
            Ok(settings) => QpuChatCommandResult {
                handled: true,
                feature_unlocked: settings.feature_unlocked,
                response: "⚛️ **QPU Oracle hidden.** Configuration removed from Settings. \
                    Type `[enable_QPU]` to restore."
                    .to_string(),
            },
            Err(e) => QpuChatCommandResult {
                handled: true,
                feature_unlocked: cached_state().feature_unlocked,
                response: format!("🔴 QPU disable failed: {e}"),
            },
        }
    } else {
        QpuChatCommandResult {
            handled: false,
            response: String::new(),
            feature_unlocked: cached_state().feature_unlocked,
        }
    }
}

/// Returns decrypted IBM token if configured and feature unlocked.
pub fn resolve_ibm_token() -> Option<String> {
    let state = cached_state();
    if !state.feature_unlocked || state.ibm_token_enc.is_empty() {
        return None;
    }
    decrypt_secret(&state.ibm_token_enc).ok()
}

/// Returns decrypted D-Wave token if configured and feature unlocked.
pub fn resolve_dwave_token() -> Option<String> {
    let state = cached_state();
    if !state.feature_unlocked || state.dwave_token_enc.is_empty() {
        return None;
    }
    decrypt_secret(&state.dwave_token_enc).ok()
}

/// Architecture routing for a quantum-bound task class.
pub fn target_architecture(task: &str) -> Option<QpuArchitecture> {
    let state = cached_state();
    if !state.feature_unlocked {
        return None;
    }
    match task {
        "qubo_routing" if state.enable_qubo_routing => Some(QpuArchitecture::Annealer),
        "dft_ground_state" if state.enable_dft_ground_state => Some(QpuArchitecture::GateModel),
        "defeasible_resolution" if state.enable_defeasible_resolution => {
            Some(QpuArchitecture::GateModel)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_command_not_handled_for_normal_text() {
        let r = handle_qpu_chat_command("hello world");
        assert!(!r.handled);
    }
}
