//! Sovereign QPU Oracle — BYOK remote quantum offload for NP-hard tasks.
//!
//! API keys are encrypted at rest via KeyVault-derived material. Only anonymized
//! numeric matrices (QUBO / VQE parameter vectors) may egress; classified data
//! is blocked by the Sentinel before any HTTP dispatch.
//!
//! # Activation
//! The QPU Oracle requires the user to affirm the Universal Human Rights commitment
//! before any remote QPU egress is permitted. The commitment code is:
//! `SSBBZmZpcm0gTXkgQ29tbWl0bWVudCB0byBVbml2ZXJzYWwgSHVtYW4gUmlnaHRz`
//! (base64 for "I Affirm My Commitment to Universal Human Rights")

use crate::state::{app_meta_dir, APP_STATE};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

static QPU_CACHE: Mutex<Option<QpuOracleState>> = Mutex::new(None);

const COMMITMENT_B64: &str =
    "SSBBZmZpcm0gTXkgQ29tbWl0bWVudCB0byBVbml2ZXJzYWwgSHVtYW4gUmlnaHRz";
const COMMITMENT_TEXT: &str = "I Affirm My Commitment to Universal Human Rights";

// Monthly free-tier quota estimates (minutes of QPU time)
const IBM_MONTHLY_MINUTES: f64 = 10.0;
const DWAVE_MONTHLY_MINUTES: f64 = 1.0;
const IONQ_MONTHLY_MINUTES: f64 = 0.0; // pay-per-use
const RIGETTI_MONTHLY_MINUTES: f64 = 0.0; // pay-per-use
const AZURE_MONTHLY_MINUTES: f64 = 0.0; // credits-based
const BRAKET_MONTHLY_MINUTES: f64 = 0.0; // pay-per-use
const GOOGLE_MONTHLY_MINUTES: f64 = 0.0; // credits-based
const QUANTINUUM_MONTHLY_MINUTES: f64 = 0.0; // pay-per-use

fn qpu_config_path() -> PathBuf {
    app_meta_dir().join("qpu_oracle.json")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QpuArchitecture {
    GateModel,
    Annealer,
    TrappedIon,
    PhotonicGate,
    NeutralAtom,
}

/// All supported QPU providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QpuProvider {
    /// IBM Quantum Network / IBM Cloud (gate model, superconducting)
    Ibm,
    /// D-Wave Leap (quantum annealing)
    DWave,
    /// IonQ Cloud (gate model, trapped ion)
    IonQ,
    /// Rigetti Quantum Cloud Services (gate model, superconducting)
    Rigetti,
    /// Microsoft Azure Quantum (multi-hardware)
    Azure,
    /// Amazon Braket (multi-hardware)
    Braket,
    /// Google Quantum AI (gate model, superconducting)
    Google,
    /// Quantinuum (gate model, trapped ion)
    Quantinuum,
}

impl QpuProvider {
    pub fn name(self) -> &'static str {
        match self {
            Self::Ibm => "IBM Quantum",
            Self::DWave => "D-Wave Leap",
            Self::IonQ => "IonQ Cloud",
            Self::Rigetti => "Rigetti QCS",
            Self::Azure => "Azure Quantum",
            Self::Braket => "Amazon Braket",
            Self::Google => "Google Quantum AI",
            Self::Quantinuum => "Quantinuum",
        }
    }

    pub fn architecture(self) -> QpuArchitecture {
        match self {
            Self::DWave => QpuArchitecture::Annealer,
            Self::IonQ | Self::Quantinuum => QpuArchitecture::TrappedIon,
            Self::Ibm | Self::Rigetti | Self::Google => QpuArchitecture::GateModel,
            Self::Azure | Self::Braket => QpuArchitecture::GateModel, // multi-hardware, default
        }
    }

    pub fn docs_url(self) -> &'static str {
        match self {
            Self::Ibm => "https://quantum.cloud.ibm.com",
            Self::DWave => "https://cloud.dwavesys.com/leap/",
            Self::IonQ => "https://cloud.ionq.com",
            Self::Rigetti => "https://qcs.rigetti.com",
            Self::Azure => "https://azure.microsoft.com/products/quantum",
            Self::Braket => "https://aws.amazon.com/braket/",
            Self::Google => "https://quantumai.google",
            Self::Quantinuum => "https://www.quantinuum.com/computingtechnology/nexus",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QpuOracleState {
    pub feature_unlocked: bool,
    /// ISO-8601 timestamp of when the commitment was affirmed.
    pub commitment_affirmed_at: Option<String>,

    // ── Provider tokens (encrypted at rest) ────────────────────────────────
    pub ibm_token_enc: String,
    pub dwave_token_enc: String,
    pub ionq_token_enc: String,
    pub rigetti_token_enc: String,
    /// Azure: "<subscription_id>/<resource_group>/<workspace>/<api_key>"
    pub azure_credentials_enc: String,
    /// Braket: "<aws_access_key_id>|<aws_secret_access_key>|<region>"
    pub braket_credentials_enc: String,
    pub google_token_enc: String,
    pub quantinuum_token_enc: String,

    // ── Usage accounting ────────────────────────────────────────────────────
    pub ibm_minutes_used: f64,
    pub dwave_minutes_used: f64,
    pub ionq_minutes_used: f64,
    pub rigetti_minutes_used: f64,
    pub azure_minutes_used: f64,
    pub braket_minutes_used: f64,
    pub google_minutes_used: f64,
    pub quantinuum_minutes_used: f64,

    // ── Feature flags ────────────────────────────────────────────────────────
    pub max_shots_per_task: u32,
    pub fallback_to_classical: bool,
    pub enable_qubo_routing: bool,
    pub enable_dft_ground_state: bool,
    pub enable_defeasible_resolution: bool,
}

impl Default for QpuOracleState {
    fn default() -> Self {
        Self {
            feature_unlocked: false,
            commitment_affirmed_at: None,
            ibm_token_enc: String::new(),
            dwave_token_enc: String::new(),
            ionq_token_enc: String::new(),
            rigetti_token_enc: String::new(),
            azure_credentials_enc: String::new(),
            braket_credentials_enc: String::new(),
            google_token_enc: String::new(),
            quantinuum_token_enc: String::new(),
            ibm_minutes_used: 0.0,
            dwave_minutes_used: 0.0,
            ionq_minutes_used: 0.0,
            rigetti_minutes_used: 0.0,
            azure_minutes_used: 0.0,
            braket_minutes_used: 0.0,
            google_minutes_used: 0.0,
            quantinuum_minutes_used: 0.0,
            max_shots_per_task: 1000,
            fallback_to_classical: true,
            enable_qubo_routing: true,
            enable_dft_ground_state: true,
            enable_defeasible_resolution: false,
        }
    }
}

/// Per-provider status returned to the frontend — no raw tokens exposed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QpuProviderStatus {
    pub provider: String,
    pub name: String,
    pub architecture: String,
    pub configured: bool,
    pub docs_url: String,
    pub minutes_used: f64,
    pub monthly_quota: f64,
}

/// Public settings view returned to the desktop UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QpuOracleSettings {
    pub feature_unlocked: bool,
    pub commitment_affirmed: bool,
    // Legacy fields (kept for UI compatibility)
    pub ibm_token_configured: bool,
    pub dwave_token_configured: bool,
    // All providers
    pub providers: Vec<QpuProviderStatus>,
    // Feature flags
    pub max_shots_per_task: u32,
    pub fallback_to_classical: bool,
    pub enable_qubo_routing: bool,
    pub enable_dft_ground_state: bool,
    pub enable_defeasible_resolution: bool,
    // Legacy quota fields
    pub ibm_quota_minutes_remaining: f64,
    pub dwave_quota_minutes_remaining: f64,
}

/// Input from the UI for saving settings.
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
    pub ionq_token: Option<String>,
    pub rigetti_token: Option<String>,
    /// Format: "<subscription_id>/<resource_group>/<workspace>/<api_key>"
    pub azure_credentials: Option<String>,
    /// Format: "<aws_access_key_id>|<aws_secret_access_key>|<region>"
    pub braket_credentials: Option<String>,
    pub google_token: Option<String>,
    pub quantinuum_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QpuChatCommandResult {
    pub handled: bool,
    pub response: String,
    pub feature_unlocked: bool,
}

// ── Encryption helpers ────────────────────────────────────────────────────────

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

// ── Persistence ───────────────────────────────────────────────────────────────

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

fn cached_state() -> QpuOracleState {
    let mut cache = QPU_CACHE.lock().unwrap();
    if cache.is_none() {
        *cache = Some(load_state_from_disk());
    }
    cache.clone().unwrap()
}

pub(crate) fn cached_state_internal() -> QpuOracleState {
    cached_state()
}

fn update_state<F: FnOnce(&mut QpuOracleState)>(f: F) -> Result<QpuOracleState, String> {
    let mut state = cached_state();
    f(&mut state);
    persist_state(&state)?;
    *QPU_CACHE.lock().unwrap() = Some(state.clone());
    Ok(state)
}

// ── Commitment verification ────────────────────────────────────────────────────

/// Verifies that `input` matches the Universal Human Rights commitment text.
/// Accepts both the plain English text and the base64 form.
pub fn verify_commitment(input: &str) -> bool {
    let trimmed = input.trim();
    if trimmed.eq_ignore_ascii_case(COMMITMENT_TEXT) {
        return true;
    }
    if trimmed == COMMITMENT_B64 {
        return true;
    }
    // Also accept the decoded base64
    if let Ok(decoded) = base64_decode(COMMITMENT_B64) {
        if trimmed.eq_ignore_ascii_case(&decoded) {
            return true;
        }
    }
    false
}

fn base64_decode(s: &str) -> Result<String, ()> {
    // Simple base64 decode using the alphabet without external crate
    let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::new();
    let bytes: Vec<u8> = s.bytes().filter(|b| *b != b'=').collect();
    let mut i = 0;
    while i + 3 < bytes.len() + 1 {
        let chunk = &bytes[i..i.min(bytes.len()).min(i + 4)];
        if chunk.is_empty() { break; }
        let vals: Vec<u8> = chunk.iter().map(|b| {
            alphabet.iter().position(|a| a == b).unwrap_or(0) as u8
        }).collect();
        if vals.len() >= 2 {
            out.push((vals[0] << 2) | (vals[1] >> 4));
        }
        if vals.len() >= 3 {
            out.push((vals[1] << 4) | (vals[2] >> 2));
        }
        if vals.len() >= 4 {
            out.push((vals[2] << 6) | vals[3]);
        }
        i += 4;
    }
    String::from_utf8(out).map_err(|_| ())
}

/// Activate advanced capabilities by affirming the Universal Human Rights commitment.
///
/// `commitment_text` must be "I Affirm My Commitment to Universal Human Rights"
/// (or the base64 form). Returns the updated settings on success.
pub fn activate_with_commitment(commitment_text: &str) -> Result<QpuOracleSettings, String> {
    if !verify_commitment(commitment_text) {
        return Err(
            "Commitment not recognised. Please enter: \
             \"I Affirm My Commitment to Universal Human Rights\""
                .into(),
        );
    }
    let now = chrono::Utc::now().to_rfc3339();
    update_state(|state| {
        state.feature_unlocked = true;
        state.commitment_affirmed_at = Some(now.clone());
    })
    .map(|s| to_public_settings(&s))
}

// ── Public API ────────────────────────────────────────────────────────────────

pub fn to_public_settings(state: &QpuOracleState) -> QpuOracleSettings {
    let providers = vec![
        QpuProviderStatus {
            provider: "ibm".into(),
            name: QpuProvider::Ibm.name().into(),
            architecture: "GateModel".into(),
            configured: !state.ibm_token_enc.is_empty(),
            docs_url: QpuProvider::Ibm.docs_url().into(),
            minutes_used: state.ibm_minutes_used,
            monthly_quota: IBM_MONTHLY_MINUTES,
        },
        QpuProviderStatus {
            provider: "dwave".into(),
            name: QpuProvider::DWave.name().into(),
            architecture: "Annealer".into(),
            configured: !state.dwave_token_enc.is_empty(),
            docs_url: QpuProvider::DWave.docs_url().into(),
            minutes_used: state.dwave_minutes_used,
            monthly_quota: DWAVE_MONTHLY_MINUTES,
        },
        QpuProviderStatus {
            provider: "ionq".into(),
            name: QpuProvider::IonQ.name().into(),
            architecture: "TrappedIon".into(),
            configured: !state.ionq_token_enc.is_empty(),
            docs_url: QpuProvider::IonQ.docs_url().into(),
            minutes_used: state.ionq_minutes_used,
            monthly_quota: IONQ_MONTHLY_MINUTES,
        },
        QpuProviderStatus {
            provider: "rigetti".into(),
            name: QpuProvider::Rigetti.name().into(),
            architecture: "GateModel".into(),
            configured: !state.rigetti_token_enc.is_empty(),
            docs_url: QpuProvider::Rigetti.docs_url().into(),
            minutes_used: state.rigetti_minutes_used,
            monthly_quota: RIGETTI_MONTHLY_MINUTES,
        },
        QpuProviderStatus {
            provider: "azure".into(),
            name: QpuProvider::Azure.name().into(),
            architecture: "GateModel".into(),
            configured: !state.azure_credentials_enc.is_empty(),
            docs_url: QpuProvider::Azure.docs_url().into(),
            minutes_used: state.azure_minutes_used,
            monthly_quota: AZURE_MONTHLY_MINUTES,
        },
        QpuProviderStatus {
            provider: "braket".into(),
            name: QpuProvider::Braket.name().into(),
            architecture: "GateModel".into(),
            configured: !state.braket_credentials_enc.is_empty(),
            docs_url: QpuProvider::Braket.docs_url().into(),
            minutes_used: state.braket_minutes_used,
            monthly_quota: BRAKET_MONTHLY_MINUTES,
        },
        QpuProviderStatus {
            provider: "google".into(),
            name: QpuProvider::Google.name().into(),
            architecture: "GateModel".into(),
            configured: !state.google_token_enc.is_empty(),
            docs_url: QpuProvider::Google.docs_url().into(),
            minutes_used: state.google_minutes_used,
            monthly_quota: GOOGLE_MONTHLY_MINUTES,
        },
        QpuProviderStatus {
            provider: "quantinuum".into(),
            name: QpuProvider::Quantinuum.name().into(),
            architecture: "TrappedIon".into(),
            configured: !state.quantinuum_token_enc.is_empty(),
            docs_url: QpuProvider::Quantinuum.docs_url().into(),
            minutes_used: state.quantinuum_minutes_used,
            monthly_quota: QUANTINUUM_MONTHLY_MINUTES,
        },
    ];

    QpuOracleSettings {
        feature_unlocked: state.feature_unlocked,
        commitment_affirmed: state.commitment_affirmed_at.is_some(),
        ibm_token_configured: !state.ibm_token_enc.is_empty(),
        dwave_token_configured: !state.dwave_token_enc.is_empty(),
        providers,
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
        if let Some(ref tok) = input.ionq_token {
            state.ionq_token_enc = encrypt_secret(tok).unwrap_or_default();
        }
        if let Some(ref tok) = input.rigetti_token {
            state.rigetti_token_enc = encrypt_secret(tok).unwrap_or_default();
        }
        if let Some(ref creds) = input.azure_credentials {
            state.azure_credentials_enc = encrypt_secret(creds).unwrap_or_default();
        }
        if let Some(ref creds) = input.braket_credentials {
            state.braket_credentials_enc = encrypt_secret(creds).unwrap_or_default();
        }
        if let Some(ref tok) = input.google_token {
            state.google_token_enc = encrypt_secret(tok).unwrap_or_default();
        }
        if let Some(ref tok) = input.quantinuum_token {
            state.quantinuum_token_enc = encrypt_secret(tok).unwrap_or_default();
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

pub fn record_usage(arch: QpuArchitecture, minutes: f64) -> Result<(), String> {
    update_state(|state| match arch {
        QpuArchitecture::GateModel => state.ibm_minutes_used += minutes,
        QpuArchitecture::Annealer => state.dwave_minutes_used += minutes,
        QpuArchitecture::TrappedIon => state.ionq_minutes_used += minutes,
        QpuArchitecture::PhotonicGate => state.google_minutes_used += minutes,
        QpuArchitecture::NeutralAtom => state.braket_minutes_used += minutes,
    })
    .map(|_| ())
}

pub fn record_provider_usage(provider: QpuProvider, minutes: f64) -> Result<(), String> {
    update_state(|state| match provider {
        QpuProvider::Ibm => state.ibm_minutes_used += minutes,
        QpuProvider::DWave => state.dwave_minutes_used += minutes,
        QpuProvider::IonQ => state.ionq_minutes_used += minutes,
        QpuProvider::Rigetti => state.rigetti_minutes_used += minutes,
        QpuProvider::Azure => state.azure_minutes_used += minutes,
        QpuProvider::Braket => state.braket_minutes_used += minutes,
        QpuProvider::Google => state.google_minutes_used += minutes,
        QpuProvider::Quantinuum => state.quantinuum_minutes_used += minutes,
    })
    .map(|_| ())
}

// ── Token resolution ──────────────────────────────────────────────────────────

pub fn resolve_ibm_token() -> Option<String> {
    let state = cached_state();
    if !state.feature_unlocked || state.ibm_token_enc.is_empty() {
        return None;
    }
    decrypt_secret(&state.ibm_token_enc).ok()
}

pub fn resolve_dwave_token() -> Option<String> {
    let state = cached_state();
    if !state.feature_unlocked || state.dwave_token_enc.is_empty() {
        return None;
    }
    decrypt_secret(&state.dwave_token_enc).ok()
}

pub fn resolve_ionq_token() -> Option<String> {
    let state = cached_state();
    if !state.feature_unlocked || state.ionq_token_enc.is_empty() {
        return None;
    }
    decrypt_secret(&state.ionq_token_enc).ok()
}

pub fn resolve_rigetti_token() -> Option<String> {
    let state = cached_state();
    if !state.feature_unlocked || state.rigetti_token_enc.is_empty() {
        return None;
    }
    decrypt_secret(&state.rigetti_token_enc).ok()
}

/// Returns `(subscription_id, resource_group, workspace, api_key)` if configured.
pub fn resolve_azure_credentials() -> Option<(String, String, String, String)> {
    let state = cached_state();
    if !state.feature_unlocked || state.azure_credentials_enc.is_empty() {
        return None;
    }
    let raw = decrypt_secret(&state.azure_credentials_enc).ok()?;
    let parts: Vec<&str> = raw.splitn(4, '/').collect();
    if parts.len() == 4 {
        Some((
            parts[0].to_string(),
            parts[1].to_string(),
            parts[2].to_string(),
            parts[3].to_string(),
        ))
    } else {
        None
    }
}

/// Returns `(access_key_id, secret_access_key, region)` if configured.
pub fn resolve_braket_credentials() -> Option<(String, String, String)> {
    let state = cached_state();
    if !state.feature_unlocked || state.braket_credentials_enc.is_empty() {
        return None;
    }
    let raw = decrypt_secret(&state.braket_credentials_enc).ok()?;
    let parts: Vec<&str> = raw.splitn(3, '|').collect();
    if parts.len() == 3 {
        Some((
            parts[0].to_string(),
            parts[1].to_string(),
            parts[2].to_string(),
        ))
    } else {
        None
    }
}

pub fn resolve_google_token() -> Option<String> {
    let state = cached_state();
    if !state.feature_unlocked || state.google_token_enc.is_empty() {
        return None;
    }
    decrypt_secret(&state.google_token_enc).ok()
}

pub fn resolve_quantinuum_token() -> Option<String> {
    let state = cached_state();
    if !state.feature_unlocked || state.quantinuum_token_enc.is_empty() {
        return None;
    }
    decrypt_secret(&state.quantinuum_token_enc).ok()
}

// ── Architecture routing ──────────────────────────────────────────────────────

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

/// Returns the best available provider for a given task, preferring remote QPU.
pub fn select_provider(task: &str) -> Option<QpuProvider> {
    let state = cached_state();
    if !state.feature_unlocked {
        return None;
    }
    match task {
        "qubo_routing" if state.enable_qubo_routing => {
            // Prefer D-Wave (annealer) for QUBO; fall back to IonQ or IBM
            if !state.dwave_token_enc.is_empty() {
                Some(QpuProvider::DWave)
            } else if !state.ionq_token_enc.is_empty() {
                Some(QpuProvider::IonQ)
            } else if !state.ibm_token_enc.is_empty() {
                Some(QpuProvider::Ibm)
            } else {
                None
            }
        }
        "dft_ground_state" if state.enable_dft_ground_state => {
            // Prefer trapped-ion for VQE accuracy; fall back to superconducting
            if !state.quantinuum_token_enc.is_empty() {
                Some(QpuProvider::Quantinuum)
            } else if !state.ionq_token_enc.is_empty() {
                Some(QpuProvider::IonQ)
            } else if !state.ibm_token_enc.is_empty() {
                Some(QpuProvider::Ibm)
            } else if !state.rigetti_token_enc.is_empty() {
                Some(QpuProvider::Rigetti)
            } else if !state.google_token_enc.is_empty() {
                Some(QpuProvider::Google)
            } else {
                None
            }
        }
        _ => None,
    }
}

// ── Chat command interception ─────────────────────────────────────────────────

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
        // Require commitment before enabling via chat command
        if !cached_state().commitment_affirmed_at.is_some() {
            return QpuChatCommandResult {
                handled: true,
                feature_unlocked: false,
                response: "⚛️ **QPU Oracle requires activation.**\n\n\
                    To activate, open **Settings → Advanced Capabilities** and affirm:\n\n\
                    > *\"I Affirm My Commitment to Universal Human Rights\"*\n\n\
                    Or pass the activation code in the Settings panel."
                    .to_string(),
            };
        }
        match enable_qpu_feature() {
            Ok(settings) => QpuChatCommandResult {
                handled: true,
                feature_unlocked: settings.feature_unlocked,
                response: "⚛️ **QPU Oracle unlocked.**\n\n\
                    Open **Settings → QPU Oracle** to configure provider API keys.\n\n\
                    **Supported providers:** IBM Quantum, D-Wave Leap, IonQ, Rigetti QCS, \
                    Azure Quantum, Amazon Braket, Google Quantum AI, Quantinuum\n\n\
                    **Chat commands:**\n\
                    - `[qpu:qubo]` or `$$\\min_{x} ...$$` → Annealing / QUBO routing\n\
                    - `[qpu:dft]` or `$$\\hat{H}\\Psi = E\\Psi$$` → VQE ground states\n\
                    - `[qpu:defeasible]` → probabilistic obligation resolution\n\n\
                    Only anonymised numeric matrices egress; classified data is blocked by Sentinel."
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
                response: "⚛️ **QPU Oracle suspended.** Type `[enable_QPU]` to restore."
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commitment_text_verifies() {
        assert!(verify_commitment("I Affirm My Commitment to Universal Human Rights"));
        assert!(verify_commitment(
            "SSBBZmZpcm0gTXkgQ29tbWl0bWVudCB0byBVbml2ZXJzYWwgSHVtYW4gUmlnaHRz"
        ));
        assert!(!verify_commitment("something else"));
        assert!(!verify_commitment(""));
    }

    #[test]
    fn chat_command_not_handled_for_normal_text() {
        let r = handle_qpu_chat_command("hello world");
        assert!(!r.handled);
    }

    #[test]
    fn provider_names_non_empty() {
        for p in [
            QpuProvider::Ibm,
            QpuProvider::DWave,
            QpuProvider::IonQ,
            QpuProvider::Rigetti,
            QpuProvider::Azure,
            QpuProvider::Braket,
            QpuProvider::Google,
            QpuProvider::Quantinuum,
        ] {
            assert!(!p.name().is_empty());
            assert!(!p.docs_url().is_empty());
        }
    }
}
