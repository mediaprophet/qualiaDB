//! Flutter Rust Bridge API — delegates to `qualia-client-core` and `qualia-core-db`.

use crate::frb_generated::StreamSink;
use qualia_client_core::api as core;
use qualia_client_core::state::{
    self, Actor, AgentConfig as CoreAgentConfig, DelegationRule, FrontDoor,
    ProgressPayload as CoreProgress,
};
use qualia_core_db::rpc::TaxRecipientSuite as CoreTaxSuite;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

// ── Physics state (UI sliders; not persisted in core) ─────────────────────────

struct PhysicsStore {
    temperature: f64,
    pressure: f64,
    time_dilation: f64,
}

static PHYSICS: Mutex<PhysicsStore> = Mutex::new(PhysicsStore {
    temperature: 50.0,
    pressure: 50.0,
    time_dilation: 1.0,
});

static DAEMON_SPAWNED: AtomicBool = AtomicBool::new(false);

fn block_on<F: std::future::Future>(f: F) -> F::Output {
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        handle.block_on(f)
    } else {
        tokio::runtime::Runtime::new()
            .expect("tokio runtime")
            .block_on(f)
    }
}

fn find_open_port(host: &str, start: u16) -> u16 {
    for port in start..=4300 {
        if std::net::TcpListener::bind((host, port)).is_ok() {
            return port;
        }
    }
    start
}

fn spawn_daemon_background() -> String {
    if DAEMON_SPAWNED.swap(true, Ordering::SeqCst) {
        return format!("Daemon already running ({})", core::daemon_status());
    }
    let state = state::APP_STATE.get().expect("APP_STATE");
    if *state.daemon_running.lock().unwrap() {
        return "Daemon already running".into();
    }
    let config = state.config.lock().unwrap().clone();
    let port = find_open_port(&config.daemon_host, config.daemon_port);
    core::set_active_daemon_port(port);
    std::env::set_var("QUALIA_STORAGE_PATH", &config.storage_path);
    qualia_core_db::daemon_graph::init_daemon_graph(&config.storage_path);
    let vault = state.key_vault.clone();
    let flag = state.daemon_running.clone();
    let storage_path = config.storage_path.clone();
    std::thread::spawn(move || {
        block_on(async {
            *flag.lock().unwrap() = true;
            tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                let _ = core::sync_workbench_torrent_seeds(&storage_path);
            });
            qualia_core_db::daemon::start_local_daemon(port, vault).await;
            *flag.lock().unwrap() = false;
            DAEMON_SPAWNED.store(false, Ordering::SeqCst);
        });
    });
    format!("Daemon starting on {}:{}", config.daemon_host, port)
}

fn load_persisted_directory() {
    let state = state::APP_STATE.get().expect("APP_STATE");
    let vault = state.key_vault.lock().unwrap();
    let ds = core::load_directory_state(&vault);
    drop(vault);
    *state.directory_actors.lock().unwrap() = ds.actors;
    *state.delegation_rules.lock().unwrap() = ds.rules;
    *state.front_doors.lock().unwrap() = ds.front_doors;
}

// ── Greeting / init ───────────────────────────────────────────────────────────

pub fn greet(name: String) -> String {
    format!(
        "Hello, {}! Welcome to QualiaDB via Flutter Rust Bridge.",
        name
    )
}

pub fn init_core() {
    let _ = core::configure_webview2_runtime();
    state::init_app_state();
    core::restore_active_model_on_startup();
    load_persisted_directory();
    let _ = core::start_qualia_protocol();
    let _ = core::seed_bundled_qapps();
    let _ = spawn_daemon_background();
    core::start_chat_relay_poller();
}

// ── Hardware / system ─────────────────────────────────────────────────────────

pub struct HardwareStatus {
    pub ram_total_gb: f64,
    pub ram_used_gb: f64,
    pub vram_estimated_gb: f64,
}

pub fn get_hardware_status() -> HardwareStatus {
    let s = core::get_hardware_status();
    HardwareStatus {
        ram_total_gb: s.ram_total_gb,
        ram_used_gb: s.ram_used_gb,
        vram_estimated_gb: s.vram_estimated_gb,
    }
}

pub struct HardwareTelemetry {
    pub cpu_percent: f64,
    pub ram_used_gb: f64,
    pub ram_total_gb: f64,
    pub daemon_status: String,
}

pub fn get_hardware_telemetry() -> HardwareTelemetry {
    use sysinfo::System;
    let mut sys = System::new_all();
    sys.refresh_cpu();
    sys.refresh_memory();
    HardwareTelemetry {
        cpu_percent: sys.global_cpu_info().cpu_usage() as f64,
        ram_used_gb: sys.used_memory() as f64 / 1024.0 / 1024.0 / 1024.0,
        ram_total_gb: sys.total_memory() as f64 / 1024.0 / 1024.0 / 1024.0,
        daemon_status: core::daemon_status(),
    }
}

pub fn profile_energy_circumstance() -> String {
    core::profile_energy_circumstance()
}

pub fn check_ollama_status() -> bool {
    core::check_ollama_status()
}

pub fn run_engine_command(cmd: String) -> String {
    core::run_engine_command(cmd)
}

// ── Config ────────────────────────────────────────────────────────────────────

pub struct AgentConfig {
    pub storage_path: String,
    pub storage_quota_gb: u64,
    pub base_connectivity_cost_ilp: u64,
}

pub fn get_config() -> AgentConfig {
    let c = core::get_config();
    AgentConfig {
        storage_path: c.storage_path,
        storage_quota_gb: c.storage_quota_gb,
        base_connectivity_cost_ilp: c.base_connectivity_cost_ilp,
    }
}

pub fn save_config(new_config: AgentConfig) -> Result<(), String> {
    let existing = core::get_config();
    core::save_config(CoreAgentConfig {
        storage_path: new_config.storage_path,
        storage_quota_gb: new_config.storage_quota_gb,
        base_connectivity_cost_ilp: new_config.base_connectivity_cost_ilp,
        daemon_host: existing.daemon_host,
        daemon_port: existing.daemon_port,
    })
}

pub fn is_first_run() -> bool {
    core::is_first_run()
}

// ── QPU Oracle ────────────────────────────────────────────────────────────────

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

pub struct QpuOracleSettingsInput {
    pub max_shots_per_task: u32,
    pub fallback_to_classical: bool,
    pub enable_qubo_routing: bool,
    pub enable_dft_ground_state: bool,
    pub enable_defeasible_resolution: bool,
    pub ibm_token: Option<String>,
    pub dwave_token: Option<String>,
}

pub struct QpuChatCommandResult {
    pub handled: bool,
    pub response: String,
    pub feature_unlocked: bool,
}

pub fn get_qpu_settings() -> QpuOracleSettings {
    let s = core::get_qpu_settings();
    QpuOracleSettings {
        feature_unlocked: s.feature_unlocked,
        ibm_token_configured: s.ibm_token_configured,
        dwave_token_configured: s.dwave_token_configured,
        max_shots_per_task: s.max_shots_per_task,
        fallback_to_classical: s.fallback_to_classical,
        enable_qubo_routing: s.enable_qubo_routing,
        enable_dft_ground_state: s.enable_dft_ground_state,
        enable_defeasible_resolution: s.enable_defeasible_resolution,
        ibm_quota_minutes_remaining: s.ibm_quota_minutes_remaining,
        dwave_quota_minutes_remaining: s.dwave_quota_minutes_remaining,
    }
}

pub fn is_qpu_feature_unlocked() -> bool {
    core::is_qpu_feature_unlocked()
}

pub fn save_qpu_settings(input: QpuOracleSettingsInput) -> Result<QpuOracleSettings, String> {
    let s = core::save_qpu_settings(core::QpuOracleSettingsInput {
        max_shots_per_task: input.max_shots_per_task,
        fallback_to_classical: input.fallback_to_classical,
        enable_qubo_routing: input.enable_qubo_routing,
        enable_dft_ground_state: input.enable_dft_ground_state,
        enable_defeasible_resolution: input.enable_defeasible_resolution,
        ibm_token: input.ibm_token,
        dwave_token: input.dwave_token,
    })?;
    Ok(QpuOracleSettings {
        feature_unlocked: s.feature_unlocked,
        ibm_token_configured: s.ibm_token_configured,
        dwave_token_configured: s.dwave_token_configured,
        max_shots_per_task: s.max_shots_per_task,
        fallback_to_classical: s.fallback_to_classical,
        enable_qubo_routing: s.enable_qubo_routing,
        enable_dft_ground_state: s.enable_dft_ground_state,
        enable_defeasible_resolution: s.enable_defeasible_resolution,
        ibm_quota_minutes_remaining: s.ibm_quota_minutes_remaining,
        dwave_quota_minutes_remaining: s.dwave_quota_minutes_remaining,
    })
}

pub fn handle_qpu_chat_command(text: String) -> QpuChatCommandResult {
    map_qpu_chat_result(core::handle_qpu_chat_command(text))
}

pub fn handle_engine_chat_command(text: String) -> QpuChatCommandResult {
    map_qpu_chat_result(core::handle_engine_chat_command(text))
}

fn map_qpu_chat_result(r: core::QpuChatCommandResult) -> QpuChatCommandResult {
    QpuChatCommandResult {
        handled: r.handled,
        response: r.response,
        feature_unlocked: r.feature_unlocked,
    }
}

// ── Daemon ────────────────────────────────────────────────────────────────────

pub fn start_daemon() -> String {
    spawn_daemon_background()
}

pub fn daemon_status() -> String {
    core::daemon_status()
}

// ── Spatial physics (local UI state) ──────────────────────────────────────────

pub struct SpatialPhysicsState {
    pub temperature: f64,
    pub pressure: f64,
    pub time_dilation: f64,
}

pub fn get_spatial_temperature() -> f64 {
    PHYSICS.lock().unwrap().temperature
}

pub fn get_spatial_pressure() -> f64 {
    PHYSICS.lock().unwrap().pressure
}

pub fn get_spatial_time_dilation() -> f64 {
    PHYSICS.lock().unwrap().time_dilation
}

pub fn get_physics_state_temperature() -> f64 {
    get_spatial_temperature()
}

pub fn get_physics_state_pressure() -> f64 {
    get_spatial_pressure()
}

pub fn get_physics_state_time_dilation() -> f64 {
    get_spatial_time_dilation()
}

pub fn get_physics_state() -> SpatialPhysicsState {
    let p = PHYSICS.lock().unwrap();
    SpatialPhysicsState {
        temperature: p.temperature,
        pressure: p.pressure,
        time_dilation: p.time_dilation,
    }
}

pub fn update_physics_state(temperature: f64, pressure: f64, time_dilation: f64) {
    let mut p = PHYSICS.lock().unwrap();
    p.temperature = temperature;
    p.pressure = pressure;
    p.time_dilation = time_dilation;
}

// ── Wallet ────────────────────────────────────────────────────────────────────

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

pub struct WalletStatus {
    pub lightning_sats: u64,
    pub ilp_microcents: u64,
    pub nym_connected: bool,
}

pub fn get_wallet_status() -> WalletStatus {
    let s = core::get_wallet_status();
    let v = serde_json::to_value(&s).unwrap_or_default();
    WalletStatus {
        lightning_sats: v["lightning_sats"].as_u64().unwrap_or(0),
        ilp_microcents: v["ilp_microcents"].as_u64().unwrap_or(0),
        nym_connected: v["nym_connected"].as_bool().unwrap_or(false),
    }
}

pub fn get_coin_balances() -> Vec<CoinBalance> {
    core::get_coin_balances()
        .into_iter()
        .map(|b| CoinBalance {
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
        })
        .collect()
}

pub struct TxRecord {
    pub txid: String,
    pub ticker: String,
    pub direction: String,
    pub amount: String,
    pub label: String,
    pub timestamp: String,
    pub status: String,
    pub confirmations: u32,
    pub fee: String,
    pub counterparty: String,
}

pub fn get_transaction_history(ticker: String) -> Vec<TxRecord> {
    core::get_transaction_history(ticker)
        .into_iter()
        .map(|t| {
            let v = serde_json::to_value(&t).unwrap_or_default();
            TxRecord {
                txid: v["txid"].as_str().unwrap_or("").into(),
                ticker: v["ticker"].as_str().unwrap_or("").into(),
                direction: v["direction"].as_str().unwrap_or("").into(),
                amount: v["amount"].as_str().unwrap_or("").into(),
                label: v["label"].as_str().unwrap_or("").into(),
                timestamp: v["timestamp"].as_str().unwrap_or("").into(),
                status: v["status"].as_str().unwrap_or("").into(),
                confirmations: v["confirmations"].as_u64().unwrap_or(0) as u32,
                fee: v["fee"].as_str().unwrap_or("").into(),
                counterparty: v["counterparty"].as_str().unwrap_or("").into(),
            }
        })
        .collect()
}

// ── Tokens ────────────────────────────────────────────────────────────────────

pub struct TokenEntry {
    pub id: String,
    pub chain: String,
    pub token_type: String,
    pub contract: String,
    pub symbol: String,
    pub name: String,
    pub balance: String,
    pub decimals: u8,
    pub fiat_usd: f64,
}

fn map_token(t: core::TokenEntry) -> TokenEntry {
    let v = serde_json::to_value(&t).unwrap_or_default();
    TokenEntry {
        id: v["id"].as_str().unwrap_or("").into(),
        chain: v["chain"].as_str().unwrap_or("").into(),
        token_type: v["token_type"].as_str().unwrap_or("").into(),
        contract: v["contract"].as_str().unwrap_or("").into(),
        symbol: v["symbol"].as_str().unwrap_or("").into(),
        name: v["name"].as_str().unwrap_or("").into(),
        balance: v["balance"].as_str().unwrap_or("").into(),
        decimals: v["decimals"].as_u64().unwrap_or(8) as u8,
        fiat_usd: v["fiat_usd"].as_f64().unwrap_or(0.0),
    }
}

pub fn get_tokens() -> Vec<TokenEntry> {
    core::get_tokens().into_iter().map(map_token).collect()
}

pub fn add_token(
    chain: String,
    token_type: String,
    contract: String,
    symbol: String,
    name: String,
    decimals: u8,
) -> Result<TokenEntry, String> {
    core::add_token(chain, token_type, contract, symbol, name, decimals).map(map_token)
}

pub fn remove_token(id: String) -> Result<(), String> {
    core::remove_token(id)
}

// ── Tax / ILP ─────────────────────────────────────────────────────────────────

pub struct TaxRecipient {
    pub label: String,
    pub ilp_address: String,
    pub share_percent: u64,
    pub use_nym: bool,
}

pub struct TaxRecipientSuite {
    pub jurisdiction_did: String,
    pub recipients: Vec<TaxRecipient>,
}

fn map_tax_suite(s: CoreTaxSuite) -> TaxRecipientSuite {
    TaxRecipientSuite {
        jurisdiction_did: s.jurisdiction_did,
        recipients: s
            .recipients
            .into_iter()
            .map(|r| TaxRecipient {
                label: r.label,
                ilp_address: r.ilp_address,
                share_percent: r.share_percent,
                use_nym: r.use_nym,
            })
            .collect(),
    }
}

fn to_core_tax_suite(s: TaxRecipientSuite) -> CoreTaxSuite {
    CoreTaxSuite {
        jurisdiction_did: s.jurisdiction_did,
        recipients: s
            .recipients
            .into_iter()
            .map(|r| qualia_core_db::rpc::TaxRecipient {
                label: r.label,
                ilp_address: r.ilp_address,
                share_percent: r.share_percent,
                use_nym: r.use_nym,
            })
            .collect(),
    }
}

pub fn get_tax_suite() -> TaxRecipientSuite {
    map_tax_suite(core::get_tax_suite())
}

pub fn save_tax_suite(suite: TaxRecipientSuite) -> Result<(), String> {
    core::save_tax_suite(to_core_tax_suite(suite))
}

pub struct PaymentReceipt {
    pub recipient_label: String,
    pub ilp_address: String,
    pub amount_micro_cents: u64,
    pub status: String,
}

pub struct DispatchResult {
    pub gross_amount_micro_cents: u64,
    pub tax_pool_micro_cents: u64,
    pub principal_remainder_micro_cents: u64,
    pub total_sent: u64,
    pub total_queued: u64,
    pub total_failed: u64,
}

pub fn dispatch_tax_payment(gross_amount_micro_cents: u64) -> Result<DispatchResult, String> {
    let r = core::dispatch_tax_payment(gross_amount_micro_cents)?;
    Ok(DispatchResult {
        gross_amount_micro_cents: r.gross_amount_micro_cents,
        tax_pool_micro_cents: r.tax_pool_micro_cents,
        principal_remainder_micro_cents: r.principal_remainder_micro_cents,
        total_sent: r.total_sent,
        total_queued: r.total_queued,
        total_failed: r.total_failed,
    })
}

// ── Identity ──────────────────────────────────────────────────────────────────

pub fn derive_wallets_from_seed(seed: String) -> Result<String, String> {
    block_on(core::derive_wallets_from_seed(seed))
        .and_then(|v| serde_json::to_string(&v).map_err(|e| e.to_string()))
}

pub fn generate_bip39_seed() -> Result<String, String> {
    block_on(core::generate_bip39_seed())
}

pub fn import_external_seed(
    network: String,
    seed: String,
    label: String,
) -> Result<String, String> {
    block_on(core::import_external_seed(network, seed, label))
}

pub fn load_identity() -> Result<Option<String>, String> {
    core::load_identity().and_then(|opt| {
        opt.map(|v| serde_json::to_string(&v).map_err(|e| e.to_string()))
            .transpose()
    })
}

pub fn save_identity(wallets_json: String) -> Result<(), String> {
    let val: serde_json::Value = serde_json::from_str(&wallets_json).map_err(|e| e.to_string())?;
    core::save_identity(val)
}

pub fn load_imported_accounts() -> Result<String, String> {
    core::load_imported_accounts()
        .and_then(|v| serde_json::to_string(&v).map_err(|e| e.to_string()))
}

pub fn save_imported_accounts(accounts_json: String) -> Result<(), String> {
    let val: serde_json::Value = serde_json::from_str(&accounts_json).map_err(|e| e.to_string())?;
    core::save_imported_accounts(val)
}

// ── Qapp vault ────────────────────────────────────────────────────────────────

pub fn list_installed_qapps() -> Vec<String> {
    core::list_installed_qapps()
}

pub fn launch_installed_qapp(qapp_name: String) -> Result<String, String> {
    core::launch_installed_qapp(qapp_name)
}

pub fn launch_installed_qapp_with_context(
    qapp_name: String,
    entrypoint: Option<String>,
    surface: Option<String>,
    payload_json: Option<String>,
    source: Option<String>,
) -> Result<String, String> {
    core::launch_installed_qapp_with_context(qapp_name, entrypoint, surface, payload_json, source)
}

pub fn inspect_installed_qapp_readiness(qapp_name: String) -> Result<String, String> {
    core::inspect_installed_qapp_readiness(qapp_name)
}

pub fn list_installed_ontology_artifacts() -> Vec<String> {
    core::list_installed_ontology_artifacts()
}

pub fn remove_installed_ontology(ontology_id: String) -> Result<String, String> {
    core::remove_installed_ontology(ontology_id)
}

pub fn remove_installed_model(model_id: String) -> Result<String, String> {
    core::remove_installed_model(model_id)
}

pub fn test_sparql_endpoint(endpoint_or_id: String) -> Result<String, String> {
    core::test_sparql_endpoint(endpoint_or_id)
}

pub fn start_qualia_protocol() -> Result<u16, String> {
    core::start_qualia_protocol()
}

pub fn qualia_protocol_port() -> u16 {
    core::qualia_protocol_port()
}

pub async fn download_and_install_update(url: String) -> Result<(), String> {
    core::download_and_install_update(url).await
}

pub fn register_qualia_uri_handler(exe_path: String) -> Result<(), String> {
    core::register_qualia_uri_handler(exe_path)
}

// ── Windows runtime prerequisites ─────────────────────────────────────────────

pub struct PrerequisiteStatus {
    pub platform_requires_check: bool,
    pub webview2_ready: bool,
    pub webview2_bundled: bool,
    pub webview2_evergreen: bool,
    pub vc_redist_ready: bool,
    pub all_ready: bool,
    pub bundled_webview2_dir: String,
}

pub fn check_prerequisites() -> PrerequisiteStatus {
    let s = core::check_prerequisites();
    PrerequisiteStatus {
        platform_requires_check: s.platform_requires_check,
        webview2_ready: s.webview2_ready,
        webview2_bundled: s.webview2_bundled,
        webview2_evergreen: s.webview2_evergreen,
        vc_redist_ready: s.vc_redist_ready,
        all_ready: s.all_ready,
        bundled_webview2_dir: s.bundled_webview2_dir,
    }
}

pub fn configure_webview2_runtime() -> bool {
    core::configure_webview2_runtime()
}

pub async fn install_prerequisite(kind: String) -> Result<(), String> {
    core::install_prerequisite(kind).await
}

pub fn generate_qapp_credential(qapp_name: String) -> String {
    core::generate_qapp_credential(qapp_name)
}

pub fn build_anatomy_graph_context_json(
    qapp_name: String,
    user_prompt: String,
    agent_reply: String,
) -> Result<String, String> {
    core::build_anatomy_graph_context_json(qapp_name, user_prompt, agent_reply)
}

pub fn build_anatomy_graph_context_json_with_dicom(
    qapp_name: String,
    user_prompt: String,
    agent_reply: String,
    dicom_file_path: Option<String>,
) -> Result<String, String> {
    core::build_anatomy_graph_context_json_with_dicom(
        qapp_name,
        user_prompt,
        agent_reply,
        dicom_file_path,
    )
}

pub fn parse_dicom_metadata_json(file_path: String) -> Result<String, String> {
    core::parse_dicom_metadata_json(file_path)
}

pub fn build_dicom_overlay_spec_json(file_path: String) -> Result<String, String> {
    core::build_dicom_overlay_spec_json(file_path)
}

pub fn verify_and_install_qapp(zip_path: String, credential_sig: String) -> String {
    if !credential_sig.starts_with("did:qualia:qapp") {
        return "Invalid Qapp Credential".to_string();
    }
    core::verify_and_install_qapp(zip_path).unwrap_or_else(|e| e)
}

// ── Ingest ────────────────────────────────────────────────────────────────────

pub fn ingest_literature(file_path: String) -> String {
    block_on(async {
        core::ingest_literature(file_path)
            .await
            .unwrap_or_else(|e| e)
    })
}

pub fn upsert_cmld_definition(term: String, context_did: String) -> String {
    block_on(async {
        core::upsert_cmld_definition(term, context_did)
            .await
            .unwrap_or_else(|e| e)
    })
}

pub fn ingest_pdf(file_name: String) -> Result<String, String> {
    block_on(async {
        let result = core::ingest_pdf(file_name).await?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    })
}

pub fn ingest_ontology(file_name: String) -> Result<String, String> {
    block_on(async {
        let val = core::ingest_ontology(file_name).await?;
        serde_json::to_string(&val).map_err(|e| e.to_string())
    })
}

pub fn ingest_image(file_path: String) -> Result<String, String> {
    block_on(async {
        let val = core::ingest_image(file_path).await?;
        serde_json::to_string(&val).map_err(|e| e.to_string())
    })
}

pub fn ingest_image_async(file_path: String, typology: String) -> Result<(), String> {
    block_on(async { core::ingest_image_async(file_path, typology).await })
}

pub fn export_to_solid(input_q42_path: String, output_dir_path: String) -> Result<String, String> {
    block_on(async { core::export_to_solid(input_q42_path, output_dir_path).await })
}

pub async fn mint_semantic_token(asset_id: String) -> Result<String, String> {
    core::mint_semantic_token(asset_id).await
}

pub async fn fetch_wallet_portfolio() -> Result<String, String> {
    core::fetch_wallet_portfolio()
        .await
        .and_then(|v| serde_json::to_string(&v).map_err(|e| e.to_string()))
}

pub async fn fetch_torrent_telemetry() -> Result<String, String> {
    core::fetch_torrent_telemetry()
        .await
        .and_then(|v| serde_json::to_string(&v).map_err(|e| e.to_string()))
}

// ── Models / downloads ────────────────────────────────────────────────────────

pub struct ModelInfo {
    pub name: String,
    pub is_active: bool,
    pub avatar_type: String,
}

pub fn discover_models() -> Vec<ModelInfo> {
    block_on(async {
        core::discover_models()
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|m| ModelInfo {
                name: m.name,
                is_active: m.is_active,
                avatar_type: m.avatar_type,
            })
            .collect()
    })
}

pub fn get_active_model() -> Option<String> {
    core::get_active_model()
}

pub fn set_active_model(model_name: String) -> Result<(), String> {
    core::set_active_model(model_name)
}

pub struct CatalogItem {
    pub id: String,
    pub name: String,
    pub tag: String,
    pub params: Option<String>,
    pub format: String,
    pub size: String,
    pub vram: Option<String>,
}

pub async fn fetch_remote_manifest(url: String) -> Result<String, String> {
    core::fetch_remote_manifest(url).await
}

pub async fn fetch_model_catalog() -> Vec<CatalogItem> {
    parse_manifest_items(
        &fetch_remote_manifest(
            "https://raw.githubusercontent.com/mediaprophet/qualiaDB/main/resources/manifest.json"
                .into(),
        )
        .await
        .unwrap_or_default(),
        "model",
    )
}

pub async fn fetch_model_catalog_real() -> Vec<CatalogItem> {
    fetch_model_catalog().await
}

pub async fn fetch_ontology_catalog() -> Vec<CatalogItem> {
    parse_manifest_items(
        &fetch_remote_manifest(
            "https://raw.githubusercontent.com/mediaprophet/qualiaDB/main/resources/ontology_manifest.json"
                .into(),
        )
        .await
        .unwrap_or_default(),
        "ontology",
    )
}

pub async fn fetch_ontology_catalog_real() -> Vec<CatalogItem> {
    fetch_ontology_catalog().await
}

fn parse_manifest_items(json: &str, default_tag: &str) -> Vec<CatalogItem> {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(json) else {
        return vec![];
    };
    let items = v
        .as_array()
        .cloned()
        .or_else(|| v.get("items").and_then(|i| i.as_array()).cloned());
    let Some(arr) = items else {
        return vec![];
    };
    arr.into_iter()
        .filter_map(|item| {
            Some(CatalogItem {
                id: item.get("id")?.as_str()?.into(),
                name: item
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("Unknown")
                    .into(),
                tag: item
                    .get("tag")
                    .and_then(|t| t.as_str())
                    .unwrap_or(default_tag)
                    .into(),
                params: item
                    .get("params")
                    .and_then(|p| p.as_str())
                    .map(String::from),
                format: item
                    .get("format")
                    .and_then(|f| f.as_str())
                    .unwrap_or("")
                    .into(),
                size: item
                    .get("size")
                    .and_then(|s| s.as_str())
                    .unwrap_or("")
                    .into(),
                vram: item.get("vram").and_then(|v| v.as_str()).map(String::from),
            })
        })
        .collect()
}

pub struct ProgressPayload {
    pub id: String,
    pub progress: f64,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub speed_kbps: f64,
    pub status: String,
}

fn map_progress(p: CoreProgress) -> ProgressPayload {
    ProgressPayload {
        id: p.id,
        progress: p.progress,
        downloaded_bytes: p.downloaded_bytes,
        total_bytes: p.total_bytes,
        speed_kbps: p.speed_kbps,
        status: p.status,
    }
}

pub fn get_active_downloads() -> Vec<ProgressPayload> {
    core::get_active_downloads()
        .into_iter()
        .map(map_progress)
        .collect()
}

pub fn cancel_download(id: String) -> Result<(), String> {
    core::cancel_download(id)
}

pub async fn download_model(
    url: String,
    filename: String,
    model_id: String,
) -> Result<String, String> {
    core::download_model(url, filename, model_id).await
}

pub async fn download_and_vectorize(
    url: String,
    filename: String,
    item_id: String,
) -> Result<String, String> {
    core::download_and_vectorize(url, filename, item_id).await
}

// ── Privacy / energy toggles ──────────────────────────────────────────────────

pub async fn toggle_nym_relay() -> Result<bool, String> {
    core::toggle_nym_relay().await
}

pub async fn toggle_stark_prover() -> Result<bool, String> {
    core::toggle_stark_prover().await
}

pub fn update_solar_input(watts: u32) {
    core::update_solar_input(watts);
}

// ── Directory / agreements ────────────────────────────────────────────────────

pub struct FrontDoorBridge {
    pub id: String,
    pub did_uri: String,
    pub label: String,
    pub created_at: String,
}

pub struct ActorBridge {
    pub id: String,
    pub actor_type: String,
    pub name: String,
    pub organization: Option<String>,
    pub qualifications: Vec<String>,
    pub roles: Vec<String>,
    pub verification_status: String,
    pub pairwise_did: String,
    pub root_did_uri: Option<String>,
}

pub struct DelegationRuleBridge {
    pub id: String,
    pub actor_id: String,
    pub granted_roles: Vec<String>,
    pub legal_basis: String,
    pub privacy_mode_limit: String,
    pub allowed_record_types: Vec<String>,
    pub restricted_records: Vec<String>,
    pub is_active: bool,
}

fn map_front_door(fd: FrontDoor) -> FrontDoorBridge {
    FrontDoorBridge {
        id: fd.id,
        did_uri: fd.did_uri,
        label: fd.label,
        created_at: fd.created_at,
    }
}

fn map_actor(a: Actor) -> ActorBridge {
    ActorBridge {
        id: a.id,
        actor_type: a.actor_type,
        name: a.name,
        organization: a.organization,
        qualifications: a.qualifications,
        roles: a.roles,
        verification_status: a.verification_status,
        pairwise_did: a.pairwise_did,
        root_did_uri: a.root_did_uri,
    }
}

fn to_actor(a: ActorBridge) -> Actor {
    Actor {
        id: a.id,
        actor_type: a.actor_type,
        name: a.name,
        organization: a.organization,
        qualifications: a.qualifications,
        roles: a.roles,
        verification_status: a.verification_status,
        pairwise_did: a.pairwise_did,
        root_did_uri: a.root_did_uri,
        routing_hints: vec![],
    }
}

fn map_rule(r: DelegationRule) -> DelegationRuleBridge {
    DelegationRuleBridge {
        id: r.id,
        actor_id: r.actor_id,
        granted_roles: r.granted_roles,
        legal_basis: r.legal_basis,
        privacy_mode_limit: r.privacy_mode_limit,
        allowed_record_types: r.allowed_record_types,
        restricted_records: r.restricted_records,
        is_active: r.is_active,
    }
}

fn to_rule(r: DelegationRuleBridge) -> DelegationRule {
    DelegationRule {
        id: r.id,
        actor_id: r.actor_id,
        granted_roles: r.granted_roles,
        legal_basis: r.legal_basis,
        privacy_mode_limit: r.privacy_mode_limit,
        allowed_record_types: r.allowed_record_types,
        restricted_records: r.restricted_records,
        is_active: r.is_active,
    }
}

pub fn get_front_doors() -> Result<Vec<FrontDoorBridge>, String> {
    core::get_front_doors().map(|v| v.into_iter().map(map_front_door).collect())
}

pub fn generate_front_door(label: String) -> Result<FrontDoorBridge, String> {
    core::generate_front_door(label).map(map_front_door)
}

pub async fn generate_front_door_invite() -> Result<String, String> {
    core::generate_front_door_invite().await
}

pub fn get_directory_actors() -> Result<Vec<ActorBridge>, String> {
    core::get_directory_actors().map(|v| v.into_iter().map(map_actor).collect())
}

pub fn add_directory_actor(actor: ActorBridge) -> Result<(), String> {
    core::add_directory_actor(to_actor(actor))
}

pub fn get_delegation_rules() -> Result<Vec<DelegationRuleBridge>, String> {
    core::get_delegation_rules().map(|v| v.into_iter().map(map_rule).collect())
}

pub fn add_delegation_rule(rule: DelegationRuleBridge) -> Result<(), String> {
    core::add_delegation_rule(to_rule(rule))
}

// ── Vault federation ──────────────────────────────────────────────────────────

pub fn accept_vault_handshake(did_key: String, payload: String) -> Result<String, String> {
    core::accept_vault_handshake(did_key, payload)
}

pub fn receive_vault_job(
    job_id: String,
    task_type: String,
    data_blob_cbor: Vec<u8>,
) -> Result<String, String> {
    core::receive_vault_job(job_id, task_type, data_blob_cbor)
}

// ── Inference ─────────────────────────────────────────────────────────────────

pub fn run_inference(prompt: String, _model_path: String) -> String {
    let session_id = match core::ensure_chat_session() {
        Ok(id) => id,
        Err(e) => return format!("[Session error: {e}]"),
    };
    match core::run_chat_inference(session_id, prompt) {
        Ok(text) => text,
        Err(e) => format!("[{e}]"),
    }
}

/// NDJSON stream: `{"event":"token","data":"..."}` then `{"event":"done","data":{...}}`.
pub fn run_inference_stream(
    prompt: String,
    _model_path: String,
    session_id: String,
    reply_to_fragment_id: Option<String>,
    sink: StreamSink<String>,
) {
    std::thread::spawn(move || {
        let (tx, rx) = std::sync::mpsc::sync_channel::<String>(512);
        std::thread::spawn(move || {
            while let Ok(line) = rx.recv() {
                let _ = sink.add(line);
            }
        });

        let sid = if session_id.is_empty() {
            match core::ensure_chat_session() {
                Ok(id) => id,
                Err(e) => {
                    let _ = tx.send(qualia_client_core::chat_inference::stream_event_error(&e));
                    return;
                }
            }
        } else {
            session_id
        };

        let tx_token = tx.clone();
        let on_token = std::sync::Arc::new(move |delta: String| {
            let _ = tx_token.send(qualia_client_core::chat_inference::stream_event_token(&delta));
        });

        let options = qualia_client_core::chat_inference::ChatInferenceOptions {
            reply_to_fragment_id: reply_to_fragment_id.filter(|s| !s.is_empty()),
        };
        let result = qualia_client_core::chat_inference::run_chat_inference_full(
            &sid,
            &prompt,
            Some(on_token),
            options,
        );

        let _ = tx.send(qualia_client_core::chat_inference::stream_event_done(&result));
    });
}

pub fn cancel_inference_stream() {
    core::cancel_chat_inference();
}
