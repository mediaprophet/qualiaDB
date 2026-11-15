use crate::qapp_registry;
use qualia_core_db::rpc::TaxRecipientSuite;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32};
use std::sync::{Arc, Mutex, OnceLock};

pub static APP_STATE: OnceLock<Arc<AppState>> = OnceLock::new();

/// Initializes app state.
pub fn init_app_state() -> Arc<AppState> {
    let config = load_config_from_disk();
    init_data_directories(&config.storage_path);
    let meta = app_meta_dir();
    let _ = std::fs::create_dir_all(&meta);
    let meta_str = meta.to_string_lossy().into_owned();
    let tax_suite = load_suite_from_disk(&config.storage_path);
    let vault = qualia_core_db::key_vault::KeyVault::load_or_generate(&meta_str)
        .unwrap_or_else(|e| panic!("KeyVault init failed ({meta_str}): {e}"));
    let state = build_app_state(config, vault, tax_suite);
    APP_STATE.set(state.clone()).ok();
    // Person ≠ machine: ensure person principal + local apparatus exist early.
    // Failures are non-fatal at boot (e.g. read-only meta dir); setup/finish retries.
    let _ = crate::identity_plane::ensure_local_apparatus(None);
    state
}

/// Attach an explicitly opened native vault without creating identity, keys,
/// directories, fleet records or services. Existing application state is never
/// replaced. The native operator selects and opens the vault before this call.
#[cfg(not(target_arch = "wasm32"))]
pub fn init_inference_host_state(
    vault: qualia_core_db::key_vault::KeyVault,
) -> Result<Arc<AppState>, String> {
    if vault.is_locked() {
        return Err("An explicitly opened unlocked vault is required".into());
    }
    if APP_STATE.get().is_some() {
        return Err("Application state is already initialized; use its existing host".into());
    }
    let state = build_app_state(
        AgentConfig::default(),
        vault,
        TaxRecipientSuite::default_cooperative(),
    );
    APP_STATE
        .set(Arc::clone(&state))
        .map_err(|_| "Application state was initialized concurrently")?;
    Ok(state)
}
fn build_app_state(
    config: AgentConfig,
    vault: qualia_core_db::key_vault::KeyVault,
    tax_suite: TaxRecipientSuite,
) -> Arc<AppState> {
    Arc::new(AppState {
        config: Mutex::new(config.clone()),
        tax_suite: Mutex::new(tax_suite),
        daemon_running: Arc::new(Mutex::new(false)),
        nym_relay_active: Arc::new(AtomicBool::new(false)),
        stark_prover_active: Arc::new(AtomicBool::new(false)),
        simulated_solar_watts: Arc::new(AtomicU32::new(0)),
        download_handles: Arc::new(Mutex::new(HashMap::new())),
        active_downloads: Arc::new(Mutex::new(HashMap::new())),
        active_model: Arc::new(Mutex::new(None)),
        // rqbit_session: Arc::new(tokio::sync::Mutex::new(None)),
        directory_actors: Arc::new(Mutex::new(Vec::new())),
        delegation_rules: Arc::new(Mutex::new(Vec::new())),
        front_doors: Arc::new(Mutex::new(Vec::new())),
        installed_qapps: Arc::new(Mutex::new(Vec::new())),
        key_vault: Arc::new(Mutex::new(vault)),
        download_events: tokio::sync::broadcast::channel(100).0,
    })
}

#[derive(Clone, Serialize)]
pub struct RelayTelemetry {
    pub packets_routed: u32,
    pub packets_dropped: u32,
    pub buffer_memory_mb: f64,
    pub is_congested: bool,
}

#[derive(Clone, Serialize)]
pub struct StarkTelemetry {
    pub status: String,
    pub cpu_utilization: f64,
    pub ram_usage_mb: f64,
    pub fragments_paged: u32,
}

// ── Config ────────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone)]
pub struct AgentConfig {
    pub storage_path: String,
    pub storage_quota_gb: u64,
    pub base_connectivity_cost_ilp: u64,
    pub daemon_host: String,
    pub daemon_port: u16,
    pub inference_backend: String,
    /// User-configurable port for the local web server (settings portal +
    /// Studio WASM + companion gateway). Default: 8080. If 0, auto-finds
    /// an open port starting from 8080.
    pub settings_port: u16,
    /// User-configurable external model directories scanned recursively for `.p64` / `.gguf` packages.
    #[serde(default = "default_model_search_paths")]
    pub model_search_paths: Vec<String>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            storage_path: dirs_default_path(),
            storage_quota_gb: 10,
            base_connectivity_cost_ilp: 5000,
            daemon_host: "127.0.0.1".to_string(),
            daemon_port: 4242,
            inference_backend: "local".to_string(),
            settings_port: 8080,
            model_search_paths: default_model_search_paths(),
        }
    }
}

/// Default dirs path.
pub fn default_model_search_paths() -> Vec<String> {
    let mut paths = Vec::new();
    for env_key in ["QUALIA_LLM_VAULT", "QUALIA_MODEL_PATHS"] {
        if let Some(val) = std::env::var_os(env_key) {
            for p in std::env::split_paths(&val) {
                if !p.as_os_str().is_empty() {
                    paths.push(p.to_string_lossy().into_owned());
                }
            }
        }
    }
    #[cfg(target_os = "windows")]
    {
        for candidate in [r"C:\LLM_files", r"C:\LLM_Models"] {
            if !paths.iter().any(|p| p.eq_ignore_ascii_case(candidate)) {
                paths.push(candidate.to_string());
            }
        }
    }
    paths
}

pub fn dirs_default_path() -> String {
    #[cfg(target_os = "windows")]
    {
        std::env::var("LOCALAPPDATA")
            .map(|d| format!("{}\\QualiaData", d))
            .unwrap_or_else(|_| "C:\\QualiaData".to_string())
    }
    #[cfg(target_os = "macos")]
    {
        std::env::var("HOME")
            .map(|h| format!("{}/Library/Application Support/QualiaData", h))
            .unwrap_or_else(|_| "/tmp/QualiaData".to_string())
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        std::env::var("HOME")
            .map(|h| format!("{}/.local/share/QualiaData", h))
            .unwrap_or_else(|_| "/tmp/QualiaData".to_string())
    }
}

/// App metadata dir.
pub fn app_meta_dir() -> PathBuf {
    // Tests and multi-profile installs may override the meta root without changing code.
    if let Ok(p) = std::env::var("QUALIA_APP_META_DIR") {
        let trimmed = p.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    #[cfg(target_os = "windows")]
    {
        PathBuf::from(
            std::env::var("LOCALAPPDATA")
                .unwrap_or_else(|_| "C:\\Users\\Default\\AppData\\Local".to_string()),
        )
        .join("Qualia")
    }
    #[cfg(target_os = "macos")]
    {
        PathBuf::from(std::env::var("HOME").unwrap_or_default())
            .join("Library/Application Support/Qualia")
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".config/qualia")
    }
}

/// Config file path.
pub fn config_file_path() -> PathBuf {
    app_meta_dir().join("config.json")
}
/// Identity file path.
pub fn identity_file_path() -> PathBuf {
    app_meta_dir().join("identity.json")
}

/// Loads config-from-disk.
pub fn load_config_from_disk() -> AgentConfig {
    std::fs::read_to_string(config_file_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(AgentConfig::default)
}

/// Initializes data directories.
pub fn init_data_directories(storage_path: &str) {
    let base = PathBuf::from(storage_path);
    for sub in &[
        "Models",
        "Index",
        "qapps",
        "library",
        "ontologies",
        "qlinks",
        "SemanticLibrary",
        "Identity",
        "Chats",
    ] {
        let _ = std::fs::create_dir_all(base.join(sub));
    }
}

// ── App state ─────────────────────────────────────────────────────────────────

#[derive(Clone, Serialize, Deserialize)]
pub struct ProgressPayload {
    pub id: String,
    pub progress: f64,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub speed_kbps: f64,
    pub status: String,
}

pub struct AppState {
    pub config: Mutex<AgentConfig>,
    pub tax_suite: Mutex<TaxRecipientSuite>,
    pub daemon_running: Arc<Mutex<bool>>,
    pub nym_relay_active: Arc<AtomicBool>,
    pub stark_prover_active: Arc<AtomicBool>,
    pub simulated_solar_watts: Arc<AtomicU32>,
    pub download_handles: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
    pub active_downloads: Arc<Mutex<HashMap<String, ProgressPayload>>>,
    pub active_model: Arc<Mutex<Option<String>>>,
    // pub rqbit_session: Arc<tokio::sync::Mutex<Option<std::sync::Arc<librqbit::Session>>>>,
    pub directory_actors: Arc<Mutex<Vec<Actor>>>,
    pub delegation_rules: Arc<Mutex<Vec<DelegationRule>>>,
    pub front_doors: Arc<Mutex<Vec<FrontDoor>>>,
    pub installed_qapps: Arc<Mutex<Vec<qapp_registry::RegisteredQapp>>>,
    pub key_vault: Arc<Mutex<qualia_core_db::key_vault::KeyVault>>,
    pub download_events: tokio::sync::broadcast::Sender<ProgressPayload>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct FrontDoor {
    pub id: String,
    pub did_uri: String,
    pub label: String,
    pub created_at: String,
    pub routing_hints: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Actor {
    pub id: String,
    pub actor_type: String,
    pub name: String,
    pub organization: Option<String>,
    pub qualifications: Vec<String>,
    pub roles: Vec<String>,
    pub verification_status: String,
    pub pairwise_did: String,
    pub root_did_uri: Option<String>,
    pub routing_hints: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DelegationRule {
    pub id: String,
    pub actor_id: String,
    pub granted_roles: Vec<String>,
    pub legal_basis: String,
    pub privacy_mode_limit: String,
    pub allowed_record_types: Vec<String>,
    pub restricted_records: Vec<String>,
    pub is_active: bool,
}

/// Suite file path.
pub fn suite_file_path(data_dir: &str) -> PathBuf {
    PathBuf::from(data_dir).join("tax_suite.json")
}

/// Loads suite-from-disk.
pub fn load_suite_from_disk(data_dir: &str) -> TaxRecipientSuite {
    let path = suite_file_path(data_dir);
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(TaxRecipientSuite::default_cooperative)
}
