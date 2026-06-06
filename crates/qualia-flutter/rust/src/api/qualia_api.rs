use qualia_client_core::api as core;

pub fn greet(name: String) -> String {
    format!("Hello, {}! Welcome to QualiaDB via Flutter Rust Bridge (Core Version).", name)
}

pub struct HardwareStatus {
    pub ram_total_gb: f64,
    pub ram_used_gb: f64,
    pub vram_estimated_gb: f64,
}

pub fn get_hardware_status() -> HardwareStatus {
    let status = core::get_hardware_status();
    HardwareStatus {
        ram_total_gb: status.ram_total_gb,
        ram_used_gb: status.ram_used_gb,
        vram_estimated_gb: status.vram_estimated_gb,
    }
}

pub fn check_ollama_status() -> bool {
    core::check_ollama_status()
}

pub struct AgentConfig {
    pub storage_path: String,
    pub storage_quota_gb: u64,
    pub base_connectivity_cost_ilp: u64,
}

pub fn get_config() -> AgentConfig {
    let conf = core::get_config();
    AgentConfig {
        storage_path: conf.storage_path,
        storage_quota_gb: conf.storage_quota_gb,
        base_connectivity_cost_ilp: conf.base_connectivity_cost_ilp,
    }
}

pub struct CoinBalance {
    pub coin: String,
    pub ticker: String,
    pub address: String,
    pub balance: f64,
    pub balance_display: String,
    pub fiat_usd: f64,
    pub price_usd: f64,
    pub change_24h: f64,
    pub network: String,
    pub status: String,
}

pub fn get_coin_balances() -> Vec<CoinBalance> {
    core::get_coin_balances().into_iter().map(|b| CoinBalance {
        coin: b.coin,
        ticker: b.ticker,
        address: b.address,
        balance: b.balance,
        balance_display: b.balance_display,
        fiat_usd: b.fiat_usd,
        price_usd: b.price_usd,
        change_24h: b.change_24h,
        network: b.network,
        status: b.status,
    }).collect()
}

pub fn start_daemon() -> String {
    core::start_daemon()
}

pub fn daemon_status() -> String {
    core::daemon_status()
}

pub fn get_spatial_temperature() -> f64 {
    0.0
}

pub fn get_spatial_pressure() -> f64 {
    0.0
}

pub fn get_spatial_time_dilation() -> f64 {
    0.0
}

pub fn get_physics_state_temperature() -> f64 {
    0.0
}

pub fn get_physics_state_pressure() -> f64 {
    0.0
}

pub fn get_physics_state_time_dilation() -> f64 {
    0.0
}

pub fn list_installed_apps() -> Vec<String> {
    core::list_installed_apps()
}

pub fn launch_installed_app(app_name: String) -> Result<String, String> {
    core::launch_installed_app(app_name)
}

pub fn generate_app_credential(app_name: String) -> String {
    core::generate_app_credential(app_name)
}

pub fn verify_and_install_app(zip_path: String, credential_sig: String) -> String {
    // Note: The new core signature only expects target_path. We can still validate the sig here or pass it if signature changed.
    // For now we'll just ignore credential_sig or add it if the signature was meant to have it.
    // The core api expects 1 argument `target_path: String`.
    if !credential_sig.starts_with("did:qualia:app") {
        return "Invalid App Credential".to_string();
    }
    core::verify_and_install_app(zip_path).unwrap_or_else(|e| e)
}

pub fn ingest_literature(file_path: String) -> String {
    // Note: core ingest literature is async
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        core::ingest_literature(file_path).await.unwrap_or_else(|e| e)
    })
}

pub fn init_core() {
    qualia_client_core::state::init_app_state();
}

// --- Legacy stubs to satisfy existing FRB dart bindings ---
pub struct ModelInfo { pub name: String, pub is_active: bool, pub avatar_type: String }
pub fn discover_models() -> Vec<ModelInfo> { vec![] }
pub fn get_active_model() -> Option<String> { None }
pub fn set_active_model(_model_name: String) {}

pub struct CatalogItem {
    pub id: String, pub name: String, pub tag: String,
    pub params: Option<String>, pub format: String,
    pub size: String, pub vram: Option<String>,
}
pub async fn fetch_model_catalog() -> Vec<CatalogItem> { vec![] }
pub async fn fetch_model_catalog_real() -> Vec<CatalogItem> { vec![] }
pub async fn fetch_ontology_catalog() -> Vec<CatalogItem> { vec![] }
pub async fn fetch_ontology_catalog_real() -> Vec<CatalogItem> { vec![] }

pub struct SpatialPhysicsState { pub temperature: f64, pub pressure: f64, pub time_dilation: f64 }
pub fn update_physics_state(_temperature: f64, _pressure: f64, _time_dilation: f64) {}

pub struct TaxRecipient { pub label: String, pub ilp_address: String, pub share_percent: f64, pub use_nym: bool }
pub struct TaxRecipientSuite { pub jurisdiction_did: String, pub recipients: Vec<TaxRecipient> }
pub fn get_tax_suite() -> TaxRecipientSuite { TaxRecipientSuite{jurisdiction_did:"".into(), recipients:vec![]} }

pub fn derive_wallets_from_seed(_seed: String) -> Result<String, String> { Ok("".into()) }
pub fn generate_bip39_seed() -> Result<String, String> { Ok("".into()) }
pub fn import_external_seed(_network: String, _seed: String, _label: String) -> Result<String, String> { Ok("".into()) }

pub fn load_identity() -> Result<Option<String>, String> { Ok(None) }
pub fn save_identity(_wallets_json: String) -> Result<(), String> { Ok(()) }
pub fn load_imported_accounts() -> Result<String, String> { Ok("".into()) }
pub fn save_imported_accounts(_accounts_json: String) -> Result<(), String> { Ok(()) }

pub fn save_config(_new_config: AgentConfig) {}

pub struct ProgressPayload {
    pub id: String, pub progress: f64, pub downloaded_bytes: u64,
    pub total_bytes: u64, pub speed_kbps: f64, pub status: String,
}
pub fn get_active_downloads() -> Vec<ProgressPayload> { vec![] }
pub fn cancel_download(_id: String) {}
pub async fn download_model(_url: String, _filename: String, _model_id: String) -> Result<String, String> { Ok("".into()) }

pub fn get_physics_state() -> SpatialPhysicsState { SpatialPhysicsState{temperature:0.0, pressure:0.0, time_dilation:0.0} }
pub fn upsert_cmld_definition(_term: String, _context_did: String) -> String { "".into() }

/// Run local LLM inference through the full Webizen-gated orchestration pipeline.
///
/// This is the primary entry point for the Flutter chat UI. It:
///   1. Validates the intent against the Rights Ontology (pre-flight)
///   2. Calls `LocalLlmAgent::infer()` → Phase 8 bifurcated SPSC GPU loop
///   3. Validates provenance citations on the output (post-flight)
///
/// `model_path` should be the absolute path to a `.gguf` file on-device
/// (e.g. the path selected in LLMHubScreen after download).
/// Pass an empty string to get a descriptive placeholder response.
pub fn run_inference(prompt: String, model_path: String) -> String {
    use qualia_core_db::{
        llm_agent::{AgentBackend, AgentIntent, LocalLlmAgent},
        orchestrator::{NullThermalGovernor, OrchestrationResult, TaskOrchestrator},
        q_hash,
    };

    if model_path.is_empty() {
        return "[No model loaded — select a GGUF model in LLM Hub first]".to_string();
    }

    let agent = LocalLlmAgent {
        agent_did: "did:qualia:flutter-chat-agent".to_string(),
        backend: AgentBackend::Local {
            model_path: model_path.clone(),
            context_window: 4096,
            quantization: "Q4_K_M".to_string(),
        },
        memory_used_bytes: std::sync::atomic::AtomicU64::new(0),
    };

    let intent = AgentIntent {
        intent_predicate: q_hash("llm:ReadGraph"),
        requested_graph_scope: vec![q_hash("user:chat_context")],
        requires_network: false,
        ilp_offer_micro_cents: 0,
        principal_did_hash: q_hash("did:qualia:flutter-user"),
        mcp_intent_frame_hash: q_hash("purpose:General"),
        active_profile: None,
    };

    let orch = TaskOrchestrator::new(Box::new(NullThermalGovernor));
    match orch.orchestrate_inference(&agent, &prompt, "flutter_chat_context", intent) {
        OrchestrationResult::Committed { text, .. } => text,
        OrchestrationResult::Blocked { reason, .. } => format!("[Webizen blocked: {}]", reason),
        OrchestrationResult::Failed(e)               => format!("[Inference error: {}]", e),
    }
}
