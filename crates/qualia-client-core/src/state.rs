use std::sync::{Arc, Mutex, OnceLock};
use std::sync::atomic::{AtomicBool, AtomicU32};
use std::collections::HashMap;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use qualia_core_db::rpc::TaxRecipientSuite;
use crate::qapp_registry;

pub static APP_STATE: OnceLock<Arc<AppState>> = OnceLock::new();

pub fn init_app_state() -> Arc<AppState> {
    let config = load_config_from_disk();
    init_data_directories(&config.storage_path);
    let meta = app_meta_dir();
    let _ = std::fs::create_dir_all(&meta);
    let meta_str = meta.to_string_lossy().into_owned();
    let state = Arc::new(AppState {
        config: Mutex::new(config.clone()),
        tax_suite: Mutex::new(load_suite_from_disk(&config.storage_path)),
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
        key_vault: Arc::new(Mutex::new(
            qualia_core_db::key_vault::KeyVault::load_or_generate(&meta_str)
                .unwrap_or_else(|e| panic!("KeyVault init failed ({meta_str}): {e}")),
        )),
        download_events: tokio::sync::broadcast::channel(100).0,
    });
    APP_STATE.set(state.clone()).ok();
    state
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
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            storage_path: dirs_default_path(),
            storage_quota_gb: 10,
            base_connectivity_cost_ilp: 5000,
            daemon_host: "127.0.0.1".to_string(),
            daemon_port: 4242,
        }
    }
}

pub fn dirs_default_path() -> String {
    #[cfg(target_os = "windows")]
    { std::env::var("APPDATA").map(|d| format!("{}\\QualiaData", d))
        .unwrap_or_else(|_| "C:\\QualiaData".to_string()) }
    #[cfg(target_os = "macos")]
    { std::env::var("HOME").map(|h| format!("{}/Library/Application Support/QualiaData", h))
        .unwrap_or_else(|_| "/tmp/QualiaData".to_string()) }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    { std::env::var("HOME").map(|h| format!("{}/.local/share/QualiaData", h))
        .unwrap_or_else(|_| "/tmp/QualiaData".to_string()) }
}

pub fn app_meta_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    { PathBuf::from(std::env::var("APPDATA").unwrap_or_else(|_| "C:\\Users\\Default\\AppData\\Roaming".to_string())).join("Qualia") }
    #[cfg(target_os = "macos")]
    { PathBuf::from(std::env::var("HOME").unwrap_or_default()).join("Library/Application Support/Qualia") }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    { PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".config/qualia") }
}

pub fn config_file_path() -> PathBuf { app_meta_dir().join("config.json") }
pub fn identity_file_path() -> PathBuf { app_meta_dir().join("identity.json") }

pub fn load_config_from_disk() -> AgentConfig {
    std::fs::read_to_string(config_file_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(AgentConfig::default)
}

pub fn init_data_directories(storage_path: &str) {
    let base = PathBuf::from(storage_path);
    for sub in &["Models", "Index", "Qapps", "SemanticLibrary", "Identity", "Chats"] {
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

pub fn suite_file_path(data_dir: &str) -> PathBuf {
    PathBuf::from(data_dir).join("tax_suite.json")
}

pub fn load_suite_from_disk(data_dir: &str) -> TaxRecipientSuite {
    let path = suite_file_path(data_dir);
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(TaxRecipientSuite::default_cooperative)
}
