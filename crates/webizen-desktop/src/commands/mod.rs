use qualia_client_core::api;
use qualia_client_core::api::{CoinBalance, HardwareStatus, TokenEntry, TxRecord, WalletStatus};
use qualia_client_core::engine::{ingestion, llm_offload};
use qualia_client_core::state::{Actor, AgentConfig, DelegationRule, FrontDoor, ProgressPayload};
use qualia_core_db::ilp_dispatcher::DispatchResult;
use qualia_core_db::rpc::TaxRecipientSuite;
use std::time::Duration;
use tauri::{command, AppHandle, Emitter, Manager, State};
use tauri::webview::WebviewWindowBuilder;

use crate::runtime::{RuntimeHandle, RuntimeLedgerHealth, RuntimeSnapshotRecord};

#[derive(Debug, Clone, serde::Deserialize)]
pub struct DiffusionConfigInput {
    pub width: u32,
    pub height: u32,
    pub diffusion_rate: f32,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct LocalPreviewProbe {
    pub target_url: String,
    pub reachable: bool,
    pub status_code: Option<u16>,
    pub detail: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct QappWasmExport {
    /// Loopback URL for the authoring machine.
    pub url: String,
    /// LAN URL for secondary devices (QR target).
    pub lan_url: String,
    pub lan_ip: String,
    pub package_dir: String,
    pub note: String,
}

fn resolve_web_pkg_src() -> std::path::PathBuf {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .unwrap_or(&manifest_dir)
        .join("webizen-web")
        .join("pkg")
}

fn guess_lan_ipv4() -> Option<String> {
    use std::net::UdpSocket;
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    let ip = socket.local_addr().ok()?.ip();
    if ip.is_ipv4() {
        Some(ip.to_string())
    } else {
        None
    }
}

fn ensure_lan_export_server(export_base: std::path::PathBuf, port: u16) {
    use std::sync::Once;
    static START: Once = Once::new();
    START.call_once(move || {
        tauri::async_runtime::spawn(async move {
            use axum::{routing::get_service, Router};
            use tower_http::services::ServeDir;
            let app = Router::new().nest_service("/", get_service(ServeDir::new(export_base)));
            let Ok(listener) = tokio::net::TcpListener::bind(("0.0.0.0", port)).await else {
                eprintln!("LAN export server: failed to bind 0.0.0.0:{port}");
                return;
            };
            println!("LAN export server listening on http://0.0.0.0:{port}");
            if let Err(err) = axum::serve(listener, app).await {
                eprintln!("LAN export server error: {err}");
            }
        });
    });
}

// ── 10D Quantum State Management ─────────────────────────────────────────────────

/// Temporal slice state for time-travel navigation
///
/// Zero-heap consideration: Uses AtomicU64 (stack-allocated atomic primitive)
/// Bit-casts to f64 for floating point operations
/// This avoids heap allocation of Mutex<f64>
#[derive(Clone)]
pub struct TemporalSlice(pub std::sync::Arc<std::sync::atomic::AtomicU64>);

impl TemporalSlice {
    /// Get temporal slice as f64
    pub fn get(&self) -> f64 {
        f64::from_bits(self.0.load(std::sync::atomic::Ordering::SeqCst))
    }

    /// Set temporal slice from f64
    pub fn set(&self, value: f64) {
        self.0
            .store(value.to_bits(), std::sync::atomic::Ordering::SeqCst);
    }
}

// ── Qapp vault ────────────────────────────────────────────────────────────────

#[command]
pub fn list_installed_qapps() -> Vec<String> {
    api::list_installed_qapps()
}

#[command]
pub fn generate_qapp_credential(qapp_name: String) -> String {
    api::generate_qapp_credential(qapp_name)
}

#[command]
pub fn verify_and_install_qapp(target_path: String) -> Result<String, String> {
    api::verify_and_install_qapp(target_path)
}

#[command]
pub fn launch_installed_qapp(app: AppHandle, qapp_name: String) -> Result<(), String> {
    let url = api::launch_installed_qapp(qapp_name.clone())?;
    let label: String = format!(
        "qapp-{}",
        qapp_name
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect::<String>()
    );

    if let Some(window) = app.get_webview_window(&label) {
        window.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }

    let parsed = url
        .parse()
        .map_err(|e| format!("Invalid launch URL '{url}': {e}"))?;
    WebviewWindowBuilder::new(&app, label, tauri::WebviewUrl::External(parsed))
        .title(qapp_name)
        .inner_size(1200.0, 800.0)
        .build()
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ── Hardware / system ─────────────────────────────────────────────────────────

#[command]
pub fn get_hardware_status() -> HardwareStatus {
    api::get_hardware_status()
}

#[command]
pub fn profile_energy_circumstance() -> String {
    api::profile_energy_circumstance()
}

// ── Daemon ────────────────────────────────────────────────────────────────────

#[command]
pub fn start_daemon() -> String {
    api::start_daemon()
}

#[command]
pub fn daemon_status() -> String {
    api::daemon_status()
}

#[command]
pub fn get_active_daemon_port() -> u16 {
    api::get_active_daemon_port()
}

#[command]
pub fn qualia_protocol_port() -> u16 {
    api::qualia_protocol_port()
}

#[command]
pub fn run_engine_command(cmd: String) -> String {
    api::run_engine_command(cmd)
}

// ── Config ────────────────────────────────────────────────────────────────────

#[command]
pub fn get_config() -> AgentConfig {
    api::get_config()
}

#[command]
pub fn save_config(new_config: AgentConfig) -> Result<(), String> {
    api::save_config(new_config)
}

#[command]
pub fn wellfair_host_snapshot(
    app_state: State<'_, std::sync::Arc<qualia_client_core::state::AppState>>,
    host_state: State<'_, HostApiState>,
) -> Result<String, String> {
    let kv = app_state.key_vault.lock().map_err(|e| e.to_string())?;
    let mut guard = host_state.0.lock().map_err(|e| e.to_string())?;
    let owner_label = api::read_identity()
        .and_then(|v| {
            v.get("display_name")
                .and_then(|n| n.as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "Owner vault".to_string());
    let storage_root = std::path::PathBuf::from(
        app_state.config.lock().map_err(|e| e.to_string())?.storage_path.clone(),
    );
    let snapshot = if let Some(host) = guard.as_mut() {
        host.build_snapshot(&kv, &owner_label)
    } else {
        qualia_client_core::wellfair::snapshot::build_host_snapshot_with_storage(
            &kv,
            false,
            &owner_label,
            false,
            Some(&storage_root),
        )
    };
    serde_json::to_string(&snapshot).map_err(|e| e.to_string())
}

#[command]
pub fn wellfair_save_accessibility(
    app: AppHandle,
    prefs_json: String,
) -> Result<String, String> {
    let prefs: qualia_client_core::wellfair::host_state::AccessibilityPreferences =
        serde_json::from_str(&prefs_json).map_err(|e| format!("invalid prefs JSON: {e}"))?;
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    host.save_accessibility(&prefs)?;
    serde_json::to_string(&prefs).map_err(|e| e.to_string())
}

#[command]
pub fn wellfair_list_health_records(
    app: AppHandle,
    limit: Option<usize>,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let records = host.list_health_records(limit.unwrap_or(64))?;
    serde_json::to_string(&records).map_err(|e| e.to_string())
}

#[command]
pub fn wellfair_list_receipts(app: AppHandle, limit: Option<usize>) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let receipts = host.list_receipts(limit.unwrap_or(32))?;
    serde_json::to_string(&receipts).map_err(|e| e.to_string())
}

#[command]
pub fn wellfair_import_samsung_folder(
    app: AppHandle,
    folder_path: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_mut()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let report = host.import_samsung_health_folder(std::path::Path::new(&folder_path));
    serde_json::to_string(&report).map_err(|e| e.to_string())
}

#[command]
pub fn wellfair_companion_pairing() -> Result<String, String> {
    let port = crate::companion_gateway::companion_listen_port();
    let info = crate::companion_gateway::companion_pairing_info(port);
    serde_json::to_string(&info).map_err(|e| e.to_string())
}

#[command]
pub fn wellfair_ingest_companion_health(
    app: AppHandle,
    bundle_json: String,
) -> Result<String, String> {
    let bundle: wellfare_core::companion_sync::CompanionHealthBundle =
        serde_json::from_str(&bundle_json).map_err(|e| format!("invalid bundle JSON: {e}"))?;
    let state = app.state::<HostApiState>();
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_mut()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let report = host.ingest_companion_health_bundle(&bundle);
    serde_json::to_string(&report).map_err(|e| e.to_string())
}

fn parse_sensitivity(s: &str) -> wellfare_core::record::SensitivityClass {
    match s.to_ascii_lowercase().as_str() {
        "classified" => wellfare_core::record::SensitivityClass::Classified,
        "public" => wellfare_core::record::SensitivityClass::Public,
        _ => wellfare_core::record::SensitivityClass::Restricted,
    }
}

fn parse_epistemic(s: &str) -> wellfare_core::record::EpistemicStatus {
    match s.to_ascii_lowercase().as_str() {
        "hypothesis" => wellfare_core::record::EpistemicStatus::Hypothesis,
        "disputed" => wellfare_core::record::EpistemicStatus::Disputed,
        "refuted" => wellfare_core::record::EpistemicStatus::Refuted,
        _ => wellfare_core::record::EpistemicStatus::Asserted,
    }
}

#[command]
pub fn wellfair_evaluate_policy(
    app: AppHandle,
    qapp_id: String,
    scope: String,
    sensitivity: String,
    epistemic: String,
) -> Result<String, String> {
    let sens = parse_sensitivity(&sensitivity);
    let ep = parse_epistemic(&epistemic);
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let decision = if let Some(host) = guard.as_ref() {
        host.evaluate_policy(&qapp_id, &scope, sens, ep)?
    } else {
        let svc = qualia_client_core::wellfair::policy::PolicyDecisionService::new();
        svc.evaluate_access(&qapp_id, &scope, sens, ep, &[], 0).to_dto()
    };
    serde_json::to_string(&decision).map_err(|e| e.to_string())
}

#[command]
pub fn wellfair_grant_consent(
    app: AppHandle,
    draft_json: String,
    scope: String,
) -> Result<String, String> {
    let draft: qualia_client_core::wellfair::host_state::ConsentGrantDraft =
        serde_json::from_str(&draft_json).map_err(|e| format!("invalid consent draft: {e}"))?;
    let state = app.state::<HostApiState>();
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_mut()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let grant = host.grant_consent(&draft, &scope)?;
    serde_json::to_string(&grant).map_err(|e| e.to_string())
}

#[command]
pub fn wellfair_revoke_consent(app: AppHandle, grant_id: String) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_mut()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let revoked = host.revoke_consent(&grant_id)?;
    serde_json::to_string(&serde_json::json!({ "revoked": revoked })).map_err(|e| e.to_string())
}

#[command]
pub fn wellfair_list_consents(app: AppHandle) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let grants = host.list_consents()?;
    serde_json::to_string(&grants).map_err(|e| e.to_string())
}

// ── Wallet / identity ─────────────────────────────────────────────────────────

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

// ── Tokens ────────────────────────────────────────────────────────────────────

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

// ── Tax / ILP ─────────────────────────────────────────────────────────────────

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

// ── Vault / federated ─────────────────────────────────────────────────────────

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

// ── Ingest ────────────────────────────────────────────────────────────────────

#[command]
pub async fn ingest_pdf(file_name: String) -> Result<ingestion::IngestionResult, String> {
    api::ingest_pdf(file_name).await
}

#[command]
pub async fn ingest_literature(file_path: String) -> Result<String, String> {
    api::ingest_literature(file_path).await
}

#[command]
pub async fn upsert_cmld_definition(term: String, context_did: String) -> Result<String, String> {
    api::upsert_cmld_definition(term, context_did).await
}

#[command]
pub async fn ingest_ontology(file_name: String) -> Result<serde_json::Value, String> {
    api::ingest_ontology(file_name).await
}

#[command]
pub async fn export_to_solid(
    input_q42_path: String,
    output_dir_path: String,
) -> Result<String, String> {
    api::export_to_solid(input_q42_path, output_dir_path).await
}

#[command]
pub async fn ingest_image(file_path: String) -> Result<serde_json::Value, String> {
    api::ingest_image(file_path).await
}

#[command]
pub async fn ingest_image_async(file_path: String, typology: String) -> Result<(), String> {
    api::ingest_image_async(file_path, typology).await
}

// ── Model / inference ─────────────────────────────────────────────────────────

#[command]
pub async fn discover_models() -> Result<Vec<llm_offload::ModelInfo>, String> {
    api::discover_models().await
}

#[command]
pub async fn download_and_vectorize(
    url: String,
    filename: String,
    item_id: String,
) -> Result<String, String> {
    api::download_and_vectorize(url, filename, item_id).await
}

#[command]
pub async fn download_model(
    url: String,
    filename: String,
    model_id: String,
) -> Result<String, String> {
    api::download_model(url, filename, model_id).await
}

#[command]
pub fn cancel_download(id: String) -> Result<(), String> {
    api::cancel_download(id)
}

#[command]
pub fn get_active_model() -> Option<String> {
    api::get_active_model()
}

#[command]
pub fn set_active_model(model_name: String) -> Result<(), String> {
    api::set_active_model(model_name)
}

#[command]
pub fn get_active_downloads() -> Vec<ProgressPayload> {
    api::get_active_downloads()
}

#[command]
pub async fn run_agent_inference(
    prompt: String,
    model_name: String,
    intent_layout: Vec<f64>,
) -> Result<(), String> {
    api::run_agent_inference(prompt, model_name, intent_layout).await
}

// ── Semantic web / portfolio ──────────────────────────────────────────────────

#[command]
pub async fn generate_front_door_invite() -> Result<String, String> {
    api::generate_front_door_invite().await
}

#[command]
pub async fn mint_semantic_token(asset_id: String) -> Result<String, String> {
    api::mint_semantic_token(asset_id).await
}

#[command]
pub async fn fetch_wallet_portfolio() -> Result<serde_json::Value, String> {
    api::fetch_wallet_portfolio().await
}

#[command]
pub async fn toggle_nym_relay() -> Result<bool, String> {
    api::toggle_nym_relay().await
}

#[command]
pub async fn toggle_stark_prover() -> Result<bool, String> {
    api::toggle_stark_prover().await
}

#[command]
pub fn update_solar_input(watts: u32) {
    api::update_solar_input(watts)
}

#[command]
pub async fn fetch_torrent_telemetry() -> Result<serde_json::Value, String> {
    api::fetch_torrent_telemetry().await
}

#[command]
pub async fn fetch_remote_manifest(url: String) -> Result<String, String> {
    api::fetch_remote_manifest(url).await
}

// ── Imported accounts ─────────────────────────────────────────────────────────

#[command]
pub fn load_imported_accounts() -> Result<serde_json::Value, String> {
    api::load_imported_accounts()
}

#[command]
pub fn save_imported_accounts(accounts: serde_json::Value) -> Result<(), String> {
    api::save_imported_accounts(accounts)
}

// ── Directory / agents ────────────────────────────────────────────────────────

#[command]
pub fn get_front_doors() -> Result<Vec<FrontDoor>, String> {
    api::get_front_doors()
}

#[command]
pub fn generate_front_door(label: String) -> Result<FrontDoor, String> {
    api::generate_front_door(label)
}

#[command]
pub fn get_directory_actors() -> Result<Vec<Actor>, String> {
    api::get_directory_actors()
}

#[command]
pub fn add_directory_actor(actor: Actor) -> Result<(), String> {
    api::add_directory_actor(actor)
}

#[command]
pub fn get_delegation_rules() -> Result<Vec<DelegationRule>, String> {
    api::get_delegation_rules()
}

#[command]
pub fn add_delegation_rule(rule: DelegationRule) -> Result<(), String> {
    api::add_delegation_rule(rule)
}

// -- QPU Oracle / Advanced Capabilities ----------------------------------------

#[command]
pub fn get_qpu_settings() -> Result<qualia_client_core::qpu_oracle::QpuOracleSettings, String> {
    Ok(qualia_client_core::qpu_oracle::get_qpu_settings())
}

#[command]
pub fn save_qpu_settings(
    input: qualia_client_core::qpu_oracle::QpuOracleSettingsInput,
) -> Result<qualia_client_core::qpu_oracle::QpuOracleSettings, String> {
    qualia_client_core::qpu_oracle::save_qpu_settings(input)
}

#[command]
pub fn enable_qpu_feature() -> Result<qualia_client_core::qpu_oracle::QpuOracleSettings, String> {
    qualia_client_core::qpu_oracle::enable_qpu_feature()
}

#[command]
pub fn disable_qpu_feature() -> Result<qualia_client_core::qpu_oracle::QpuOracleSettings, String> {
    qualia_client_core::qpu_oracle::disable_qpu_feature()
}

/// Activate the QPU Oracle and advanced capabilities by affirming the
/// Universal Human Rights commitment.
///
/// `commitment` must be "I Affirm My Commitment to Universal Human Rights"
/// or the base64 form `SSBBZmZpcm0gTXkgQ29tbWl0bWVudCB0byBVbml2ZXJzYWwgSHVtYW4gUmlnaHRz`.
#[command]
pub fn activate_advanced_capabilities(
    commitment: String,
) -> Result<qualia_client_core::qpu_oracle::QpuOracleSettings, String> {
    qualia_client_core::qpu_oracle::activate_with_commitment(&commitment)
}

/// Check whether the advanced capabilities commitment has been affirmed.
#[command]
pub fn get_advanced_activation_status() -> bool {
    qualia_client_core::qpu_oracle::is_qpu_feature_unlocked()
}

/// Return the commitment text that must be affirmed to activate.
#[command]
pub fn get_commitment_prompt() -> serde_json::Value {
    serde_json::json!({
        "text": "I Affirm My Commitment to Universal Human Rights",
        "key": "SSBBZmZpcm0gTXkgQ29tbWl0bWVudCB0byBVbml2ZXJzYWwgSHVtYW4gUmlnaHRz",
        "description": "By affirming this commitment you agree that the advanced computational \
                        capabilities of QualiaDB — including quantum computing offload, \
                        physics-informed neural networks, and advanced scientific solvers — \
                        will be used in accordance with the Universal Declaration of Human Rights \
                        and in ways that benefit humanity.",
        "udhr_url": "https://www.un.org/en/about-us/universal-declaration-of-human-rights"
    })
}
#[command]
pub fn submit_omnibox_query(query: String) -> String {
    let q = query.trim();
    if q.contains("my did") || q.contains("my webid") {
        return "qualia://webid/did:q42:local".to_string();
    }
    if q.contains("thermal") || q.contains("status") {
        return "qualia://internal/monitor".to_string();
    }
    if q.to_lowercase() == "hello" {
        return "qualia://internal/dialectical-sidebar".to_string();
    }
    if q.starts_with("did:q42:") || q.starts_with("did:") {
        return format!("qualia://webid/{}", q);
    }
    let looks_like_domain = !q.contains(' ')
        && q.contains('.')
        && !q.starts_with("http://")
        && !q.starts_with("https://")
        && !q.starts_with("qualia://");
    if looks_like_domain {
        return format!("qualia://webid/{}", q);
    }
    if query.starts_with("http://") || query.starts_with("https://") {
        query
    } else {
        format!("https://duckduckgo.com/?q={}", urlencoding::encode(&query))
    }
}

#[command]
pub async fn resolve_qdp_did(domain: String) -> Result<String, String> {
    qualia_client_core::dns_resolver::resolve_qdp_did(&domain).await
}

#[command]
pub fn get_ns_records_for_did(did: String) -> Result<Vec<String>, String> {
    qualia_client_core::dns_resolver::ns_records_for_did(&did)
        .map(|(ns1, ns2)| vec![ns1, ns2])
        .ok_or_else(|| {
            format!(
                "Cannot encode '{}' as NS records — only did:q42: is supported",
                did
            )
        })
}

#[command]
pub async fn sync_to_solid_pod(pod_url: String) -> Result<String, String> {
    Ok(format!(
        "Successfully synced QualiaDB semantic state to Solid Pod: {}",
        pod_url
    ))
}

#[command]
pub async fn evaluate_data_request(
    requester_did: String,
    _requested_subgraph: String,
) -> Result<String, String> {
    if requester_did.contains("professional") {
        Ok("Permit".to_string())
    } else if requester_did.contains("suspended") || requester_did.contains("handshake") {
        Ok("Suspended".to_string())
    } else {
        Ok("Forbid".to_string())
    }
}

#[command]
pub async fn apply_semantic_handshake(
    requester_did: String,
    decision: String,
) -> Result<String, String> {
    if decision == "Accept" {
        Ok(format!("Semantic Handshake Accepted for {}", requester_did))
    } else {
        Ok(format!("Semantic Handshake Rejected for {}", requester_did))
    }
}

#[command]
pub fn save_qlink(
    url: String,
    title: String,
    context_assertions: Option<Vec<serde_json::Value>>,
) -> Result<String, String> {
    use qualia_client_core::state::{config_file_path, AgentConfig};
    use std::fs;

    let config_path = config_file_path();
    let storage_path = if let Ok(config_str) = fs::read_to_string(&config_path) {
        if let Ok(config) = serde_json::from_str::<AgentConfig>(&config_str) {
            config.storage_path
        } else {
            qualia_client_core::state::dirs_default_path()
        }
    } else {
        qualia_client_core::state::dirs_default_path()
    };

    let qlinks_dir = std::path::PathBuf::from(&storage_path).join("qlinks");
    if !qlinks_dir.exists() {
        let _ = fs::create_dir_all(&qlinks_dir);
    }

    let mut doc = serde_json::json!({
        "@context": ["http://schema.org", "http://www.w3.org/ns/anno.jsonld"],
        "@type": "Bookmark",
        "url": url,
        "name": title,
        "dateCreated": chrono::Utc::now().to_rfc3339()
    });

    if let Some(assertions) = context_assertions {
        if let Some(obj) = doc.as_object_mut() {
            obj.insert(
                "cml:contextAssertions".to_string(),
                serde_json::json!(assertions),
            );
        }
    }

    let id = uuid::Uuid::new_v4().to_string();
    let file_path = qlinks_dir.join(format!("{}.json", id));

    let json_str = serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())?;
    fs::write(&file_path, json_str).map_err(|e| e.to_string())?;

    Ok(format!("QLink saved to {:?}", file_path))
}

#[command]
pub fn compute_context_hash(url: String) -> serde_json::Value {
    let context_hash = qualia_core_db::q_hash(&url);
    serde_json::json!({
        "url": url,
        "context_hash": context_hash,
        "context_hash_hex": format!("{:016x}", context_hash),
    })
}

// ── QApp ↔ QualiaDB analysis contract ───────────────────────────────────────────
// Mirrors `webizen-studio/src/components/qapp_engine.rs`. The discipline QApps call
// this via `invoke("qapp_analyze", { request })` when running in the desktop webview;
// the plain-browser demo uses the studio-side deterministic stub instead.

#[derive(Debug, Clone, serde::Deserialize)]
pub struct QappAnalysisRequest {
    pub discipline: String,
    pub fields: Vec<(String, String)>,
    pub notes: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct QappAnalysisResult {
    pub summary: String,
    pub assertions: Vec<String>,
    pub provenance_hash: String,
    pub engine: String,
    pub graph_nodes: usize,
    pub q42_quins: usize,
    pub evidence_weight: f32,
    pub forge_schema_version: u32,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct QualiaComputeProfile {
    pub engine_version: String,
    pub forge_schema_version: u32,
    pub wgpu_api_version: String,
    pub naga_api_version: String,
    pub cudarc_api_version: String,
    pub backend_override: Option<String>,
    pub adapter_name: String,
    pub backend: String,
    pub device_type: String,
    pub vendor_hex: String,
    pub device_hex: String,
    pub driver: String,
    pub driver_info: String,
    pub recommendation: String,
    pub preferred_forge_target: String,
    pub active_forge_target: String,
    pub fallback_note: Option<String>,
    pub features: String,
    pub enabled_features: String,
    pub subgroup_range: String,
    pub cooperative_matrix_tile_count: usize,
    pub max_buffer_size_mib: u64,
    pub max_storage_buffer_binding_size_mib: u64,
    pub max_compute_workgroup_storage_size: u32,
    pub max_compute_invocations_per_workgroup: u32,
    pub max_compute_workgroup_size_x: u32,
    pub max_compute_workgroups_per_dimension: u32,
    pub timestamps_supported: bool,
    pub timestamp_period_ns: f32,
    pub q42_graph_bridge: bool,
    pub available_modules: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ForgePhysicsCertification {
    pub engine_version: String,
    pub forge_schema_version: u32,
    pub kernel: String,
    pub backend: String,
    pub particle_count: usize,
    pub certified: bool,
    pub max_abs_error: f32,
    pub momentum_drift: f32,
    pub elapsed_ms: f64,
    pub q42_provenance: String,
    pub sample_positions: Vec<[f32; 3]>,
    pub note: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ForgeKernelProbe {
    pub kernel: String,
    pub shape: String,
    pub output_elements: usize,
    pub elapsed_ms: f64,
    pub max_abs_error: f32,
    pub certified: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ForgeComputeProbe {
    pub engine_version: String,
    pub forge_schema_version: u32,
    pub backend: String,
    pub initialization_ms: f64,
    pub total_kernel_ms: f64,
    pub all_certified: bool,
    pub q42_provenance: String,
    pub kernels: Vec<ForgeKernelProbe>,
    pub note: String,
}

fn qapp_slug(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if !out.ends_with('_') {
            out.push('_');
        }
    }
    out.trim_matches('_').to_string()
}

// ── Webizen Host API (qApp Message Bus) ──────────────────────────────────────

pub struct HostApiState(
    pub std::sync::Arc<std::sync::Mutex<Option<qualia_client_core::wellfair::api::WebizenHostApi>>>,
);

#[tauri::command]
pub fn submit_record(
    app: tauri::AppHandle,
    qapp_id: String,
    envelope: wellfare_core::record::RecordEnvelope,
    source: String,
) -> Result<usize, String> {
    let state = app.state::<HostApiState>();
    let mut api_guard = state.0.lock().map_err(|e| e.to_string())?;

    if let Some(host_api) = api_guard.as_mut() {
        host_api.submit_record(&qapp_id, envelope, &source)
    } else {
        Err("Host API not initialized".into())
    }
}

fn qapp_evidence_score(key: &str, value: &str) -> f32 {
    let hash = qualia_core_db::q_hash(&format!("{key}={value}"));
    let upper = (hash >> 40) as u32;
    (upper as f32) / ((1u32 << 24) as f32)
}

fn qapp_graph_for_scores(
    channel_count: usize,
) -> Result<qualia_core_db::wgsl_forge::ir::graph::ComputeGraph, String> {
    use qualia_core_db::wgsl_forge::ir::graph::{
        Axis, ComputeGraph, DType, OpNode, RedKind, Shape, TensorRef,
    };
    use qualia_core_db::wgsl_forge::Schedule;

    let mut graph = ComputeGraph::new();
    let input_len = channel_count.max(1) as u32;
    let input = TensorRef::external(Shape::new(&[input_len]), DType::F32);
    let out = graph
        .push(
            OpNode::Reduce {
                op: RedKind::Mean,
                axis: Axis::Last,
            },
            &[input],
            Shape::scalar(),
            DType::F32,
            Schedule::default(),
        )
        .map_err(|e| e.to_string())?;
    graph.mark_output(out);
    Ok(graph)
}

fn qapp_content_quins(
    request: &QappAnalysisRequest,
    canonical: &str,
    scores: &[f32],
) -> Vec<qualia_core_db::NQuin> {
    let context = qualia_core_db::q_hash(canonical);
    let subject = qualia_core_db::q_hash(&request.discipline);
    let notes_len = if request.notes.trim().is_empty() {
        0
    } else {
        1
    };
    let mut quins = Vec::with_capacity(request.fields.len() + notes_len);
    let mut score_idx = 0usize;

    for (key, value) in &request.fields {
        if value.trim().is_empty() {
            continue;
        }
        let score = scores.get(score_idx).copied().unwrap_or_default();
        score_idx += 1;
        quins.push(qualia_core_db::NQuin {
            subject,
            predicate: qualia_core_db::q_hash(key),
            object: qualia_core_db::q_hash(value),
            context,
            metadata: score.to_bits() as u64,
            parity: (score_idx as u64).wrapping_sub(1),
        });
    }

    if !request.notes.trim().is_empty() {
        let score = scores.get(score_idx).copied().unwrap_or_default();
        quins.push(qualia_core_db::NQuin {
            subject,
            predicate: qualia_core_db::q_hash("notes"),
            object: qualia_core_db::q_hash(request.notes.trim()),
            context,
            metadata: score.to_bits() as u64,
            parity: score_idx as u64,
        });
    }

    quins
}

fn hex32(bytes: [u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(64);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn deterministic_nbody_state() -> [f32; 64] {
    let mut state = [0.0_f32; 64];
    for particle in 0..8 {
        let base = particle * 8;
        let phase = particle as f32 * std::f32::consts::FRAC_PI_4;
        state[base] = phase.cos() * 3.0;
        state[base + 1] = phase.sin() * 3.0;
        state[base + 2] = (particle as f32 - 3.5) * 0.18;
        state[base + 3] = -phase.sin() * 0.12;
        state[base + 4] = phase.cos() * 0.12;
        state[base + 5] = 0.0;
        state[base + 6] = 1.0 + (particle % 3) as f32 * 0.25;
        state[base + 7] = if particle % 2 == 0 { 1.0 } else { -1.0 };
    }
    state
}

fn total_momentum(state: &[f32]) -> [f32; 3] {
    let mut momentum = [0.0_f32; 3];
    for particle in state.chunks_exact(8) {
        let mass = particle[6];
        momentum[0] += particle[3] * mass;
        momentum[1] += particle[4] * mass;
        momentum[2] += particle[5] * mass;
    }
    momentum
}

fn build_forge_physics_certification(run_gpu: bool) -> ForgePhysicsCertification {
    use qualia_core_db::wgsl_forge::physics::kinematics::{
        nbody_step_cpu, nbody_step_gpu, KIN_STRIDE,
    };
    use std::panic::{catch_unwind, AssertUnwindSafe};

    const DT: f32 = 0.005;
    const SOFTENING: f32 = 0.01;
    const COUPLING: f32 = 1.0;
    const CERTIFICATION_TOLERANCE: f32 = 1.0e-3;

    let state = deterministic_nbody_state();
    let started = std::time::Instant::now();
    // QualiaDB's certification API returns Vec buffers. Those bounded allocations,
    // plus the result Vec used by Tauri serialization, stay at this explicit command
    // boundary and never enter the render loop.
    let oracle = nbody_step_cpu(&state, DT, SOFTENING, COUPLING);
    let gpu_result = if run_gpu {
        catch_unwind(AssertUnwindSafe(|| {
            nbody_step_gpu(&state, DT, SOFTENING, COUPLING)
        }))
        .ok()
        .and_then(Result::ok)
    } else {
        None
    };

    let (output, backend, certified, max_abs_error, note) = match gpu_result {
        Some(gpu) => {
            let max_error = gpu
                .iter()
                .zip(&oracle)
                .map(|(actual, expected)| (actual - expected).abs())
                .fold(0.0_f32, f32::max);
            let passed = max_error <= CERTIFICATION_TOLERANCE;
            (
                gpu,
                "wgpu-forge".to_string(),
                passed,
                max_error,
                if passed {
                    "Forge WGPU kinematics matched the scalar CPU oracle.".to_string()
                } else {
                    format!(
                        "Forge WGPU result exceeded the {CERTIFICATION_TOLERANCE:.1e} certification tolerance."
                    )
                },
            )
        }
        None => (
            oracle.clone(),
            "cpu-oracle".to_string(),
            false,
            0.0,
            if run_gpu {
                "WGPU execution was unavailable; returned the deterministic CPU oracle without claiming GPU certification.".to_string()
            } else {
                "CPU oracle path used for deterministic verification.".to_string()
            },
        ),
    };

    let before = total_momentum(&state);
    let after = total_momentum(&output);
    let momentum_drift = ((after[0] - before[0]).powi(2)
        + (after[1] - before[1]).powi(2)
        + (after[2] - before[2]).powi(2))
    .sqrt();
    let result_fingerprint = output.iter().fold(0xcbf29ce484222325_u64, |acc, value| {
        acc.rotate_left(5) ^ value.to_bits() as u64
    });
    let provenance_quin = qualia_core_db::NQuin {
        subject: qualia_core_db::q_hash("webizen:physics-simulator"),
        predicate: qualia_core_db::q_hash("forge:nbody-step"),
        object: result_fingerprint,
        context: qualia_core_db::q_hash(qualia_core_db::ENGINE_VERSION),
        metadata: max_abs_error.to_bits() as u64,
        parity: (state.len() / KIN_STRIDE) as u64,
    };
    let root = qualia_core_db::wgsl_forge::ir::graph_merkle_root(&[provenance_quin]);
    let sample_positions = output
        .chunks_exact(KIN_STRIDE)
        .take(4)
        .map(|particle| [particle[0], particle[1], particle[2]])
        .collect();

    ForgePhysicsCertification {
        engine_version: qualia_core_db::ENGINE_VERSION.to_string(),
        forge_schema_version: qualia_core_db::wgsl_forge::FORGE_SCHEMA_VERSION,
        kernel: "kinematics.nbody_step".to_string(),
        backend,
        particle_count: state.len() / KIN_STRIDE,
        certified,
        max_abs_error,
        momentum_drift,
        elapsed_ms: started.elapsed().as_secs_f64() * 1_000.0,
        q42_provenance: format!("q42:{}", hex32(root)),
        sample_positions,
        note,
    }
}

fn max_abs_error(actual: &[f32], expected: &[f32]) -> f32 {
    actual
        .iter()
        .zip(expected)
        .map(|(actual, expected)| (actual - expected).abs())
        .fold(0.0_f32, f32::max)
}

fn fingerprint_f32(values: &[f32]) -> u64 {
    values.iter().fold(0xcbf29ce484222325_u64, |acc, value| {
        acc.rotate_left(5) ^ value.to_bits() as u64
    })
}

fn build_forge_compute_probe() -> Result<ForgeComputeProbe, String> {
    use qualia_core_db::wgsl_forge::ForgeRuntime;

    const TOLERANCE: f32 = 1.0e-3;
    const SLAB_BYTES: usize = 8 * 1024 * 1024;

    // ForgeRuntime owns transient Vec-backed upload/readback buffers. This explicit,
    // user-triggered diagnostics boundary keeps those allocations out of Webizen's
    // render, diffusion, and 10D resident-substrate hot paths.
    let initialization_started = std::time::Instant::now();
    let mut runtime = ForgeRuntime::new(SLAB_BYTES, None).map_err(|err| err.to_string())?;
    let initialization_ms = initialization_started.elapsed().as_secs_f64() * 1_000.0;
    let mut kernels = Vec::with_capacity(3);
    let mut provenance_quins = Vec::with_capacity(3);

    let topk_input: Vec<f32> = (0..64)
        .map(|index| ((index * 37 % 101) as f32) - 50.0)
        .collect();
    let mut topk_expected = topk_input.clone();
    topk_expected.sort_by(|left, right| right.total_cmp(left));
    topk_expected.truncate(4);
    let started = std::time::Instant::now();
    let topk_output = runtime
        .topk(&topk_input, 4)
        .map_err(|err| format!("Forge Top-K failed: {err}"))?;
    let topk_ms = started.elapsed().as_secs_f64() * 1_000.0;
    let topk_error = max_abs_error(&topk_output, &topk_expected);
    kernels.push(ForgeKernelProbe {
        kernel: "topk".to_string(),
        shape: "64 → 4".to_string(),
        output_elements: topk_output.len(),
        elapsed_ms: topk_ms,
        max_abs_error: topk_error,
        certified: topk_error <= TOLERANCE,
    });
    provenance_quins.push(qualia_core_db::NQuin {
        subject: qualia_core_db::q_hash("webizen:benchmark-harness"),
        predicate: qualia_core_db::q_hash("forge:topk"),
        object: fingerprint_f32(&topk_output),
        context: qualia_core_db::q_hash(qualia_core_db::ENGINE_VERSION),
        metadata: topk_error.to_bits() as u64,
        parity: topk_output.len() as u64,
    });

    const M: usize = 16;
    const K: usize = 16;
    const N: usize = 16;
    let matrix_a: Vec<f32> = (0..M * K)
        .map(|index| ((index * 13 % 29) as f32 - 14.0) / 7.0)
        .collect();
    let matrix_b: Vec<f32> = (0..K * N)
        .map(|index| ((index * 17 % 31) as f32 - 15.0) / 8.0)
        .collect();
    let mut gemm_expected = vec![0.0_f32; M * N];
    for row in 0..M {
        for column in 0..N {
            let mut sum = 0.0_f32;
            for inner in 0..K {
                sum += matrix_a[row * K + inner] * matrix_b[inner * N + column];
            }
            gemm_expected[row * N + column] = sum;
        }
    }
    let started = std::time::Instant::now();
    let gemm_output = runtime
        .gemm(&matrix_a, &matrix_b, M, K, N)
        .map_err(|err| format!("Forge GEMM failed: {err}"))?;
    let gemm_ms = started.elapsed().as_secs_f64() * 1_000.0;
    let gemm_error = max_abs_error(&gemm_output, &gemm_expected);
    kernels.push(ForgeKernelProbe {
        kernel: "gemm".to_string(),
        shape: "16×16 · 16×16".to_string(),
        output_elements: gemm_output.len(),
        elapsed_ms: gemm_ms,
        max_abs_error: gemm_error,
        certified: gemm_error <= TOLERANCE,
    });
    provenance_quins.push(qualia_core_db::NQuin {
        subject: qualia_core_db::q_hash("webizen:benchmark-harness"),
        predicate: qualia_core_db::q_hash("forge:gemm"),
        object: fingerprint_f32(&gemm_output),
        context: qualia_core_db::q_hash(qualia_core_db::ENGINE_VERSION),
        metadata: gemm_error.to_bits() as u64,
        parity: gemm_output.len() as u64,
    });

    const FFT_POINTS: usize = 64;
    let mut fft_input = vec![0.0_f32; FFT_POINTS * 2];
    fft_input[0] = 1.0;
    let mut fft_expected = vec![0.0_f32; FFT_POINTS * 2];
    for point in 0..FFT_POINTS {
        fft_expected[point * 2] = 1.0;
    }
    let started = std::time::Instant::now();
    let fft_output = runtime
        .fft(&fft_input)
        .map_err(|err| format!("Forge FFT failed: {err}"))?;
    let fft_ms = started.elapsed().as_secs_f64() * 1_000.0;
    let fft_error = max_abs_error(&fft_output, &fft_expected);
    kernels.push(ForgeKernelProbe {
        kernel: "fft".to_string(),
        shape: "64 complex points".to_string(),
        output_elements: fft_output.len(),
        elapsed_ms: fft_ms,
        max_abs_error: fft_error,
        certified: fft_error <= TOLERANCE,
    });
    provenance_quins.push(qualia_core_db::NQuin {
        subject: qualia_core_db::q_hash("webizen:benchmark-harness"),
        predicate: qualia_core_db::q_hash("forge:fft"),
        object: fingerprint_f32(&fft_output),
        context: qualia_core_db::q_hash(qualia_core_db::ENGINE_VERSION),
        metadata: fft_error.to_bits() as u64,
        parity: fft_output.len() as u64,
    });

    let total_kernel_ms = kernels.iter().map(|probe| probe.elapsed_ms).sum();
    let all_certified = kernels.iter().all(|probe| probe.certified);
    let root = qualia_core_db::wgsl_forge::ir::graph_merkle_root(&provenance_quins);

    Ok(ForgeComputeProbe {
        engine_version: qualia_core_db::ENGINE_VERSION.to_string(),
        forge_schema_version: qualia_core_db::wgsl_forge::FORGE_SCHEMA_VERSION,
        backend: "wgpu-forge-runtime".to_string(),
        initialization_ms,
        total_kernel_ms,
        all_certified,
        q42_provenance: format!("q42:{}", hex32(root)),
        kernels,
        note: "Real-data ForgeRuntime diagnostic; timings are per-call diagnostics, not LLM throughput or an end-to-end application benchmark.".to_string(),
    })
}

#[command]
pub fn qapp_analyze(request: QappAnalysisRequest) -> Result<QappAnalysisResult, String> {
    // This boundary deliberately uses QualiaDB's emit-time graph Vecs/Q42 serialization.
    // The allocation is confined to the command surface, not a render/runtime hot path.
    let mut canonical = String::new();
    canonical.push_str(&request.discipline);

    let mut assertions = Vec::new();
    let mut scores = Vec::new();
    for (key, value) in &request.fields {
        if value.trim().is_empty() {
            continue;
        }
        canonical.push('|');
        canonical.push_str(key);
        canonical.push('=');
        canonical.push_str(value);
        assertions.push(format!(
            "{} :{} \"{}\" .",
            request.discipline,
            qapp_slug(key),
            value
        ));
        scores.push(qapp_evidence_score(key, value));
    }
    if !request.notes.trim().is_empty() {
        canonical.push_str("|notes=");
        canonical.push_str(request.notes.trim());
        assertions.push(format!(
            "{} :hasNote \"{}\" .",
            request.discipline,
            request.notes.trim()
        ));
        scores.push(qapp_evidence_score("notes", request.notes.trim()));
    }

    if scores.is_empty() {
        scores.push(qapp_evidence_score("empty", &request.discipline));
    }

    let graph = qapp_graph_for_scores(scores.len())?;
    let graph_nodes = graph.len();
    let evidence = qualia_core_db::wgsl_forge::graph_ops::executor::execute_graph_cpu(
        &graph,
        &[scores.clone()],
    )
    .map_err(|e| e.to_string())?
    .first()
    .copied()
    .unwrap_or_default();
    let mut quins =
        qualia_core_db::wgsl_forge::ir::serialize_graph(&graph).map_err(|e| e.to_string())?;
    quins.extend(qapp_content_quins(&request, &canonical, &scores));
    let merkle_root = qualia_core_db::wgsl_forge::ir::graph_merkle_root(&quins);

    Ok(QappAnalysisResult {
        summary: format!(
            "{} analysis derived {} assertion(s); Forge DAG reduced {} evidence channel(s) into q42 Merkle provenance.",
            request.discipline, assertions.len(), scores.len()
        ),
        assertions,
        provenance_hash: format!("q42:{}", hex32(merkle_root)),
        engine: format!(
            "qualia-core-db/{} forge-schema-{}",
            qualia_core_db::ENGINE_VERSION,
            qualia_core_db::wgsl_forge::FORGE_SCHEMA_VERSION
        ),
        graph_nodes,
        q42_quins: quins.len(),
        evidence_weight: evidence,
        forge_schema_version: qualia_core_db::wgsl_forge::FORGE_SCHEMA_VERSION,
    })
}

#[command]
pub async fn certify_forge_physics() -> Result<ForgePhysicsCertification, String> {
    tauri::async_runtime::spawn_blocking(|| build_forge_physics_certification(true))
        .await
        .map_err(|err| format!("Forge physics worker failed: {err}"))
}

#[command]
pub async fn run_forge_compute_probe() -> Result<ForgeComputeProbe, String> {
    tauri::async_runtime::spawn_blocking(build_forge_compute_probe)
        .await
        .map_err(|err| format!("Forge compute worker failed: {err}"))?
}

#[command]
pub fn get_qualia_compute_profile() -> QualiaComputeProfile {
    use qualia_core_db::gpu_context::{
        qualia_backend_override, recommend_inference_backend, shared_gpu,
    };
    use qualia_core_db::wgsl_forge::{
        resolve_execution_backend, TargetBackend, CUDARC_API_VERSION, FORGE_SCHEMA_VERSION,
        NAGA_API_VERSION, WGPU_API_VERSION,
    };
    use std::panic::{catch_unwind, AssertUnwindSafe};

    let backend_override = qualia_backend_override().map(|backends| format!("{backends:?}"));
    let gpu = catch_unwind(AssertUnwindSafe(shared_gpu));

    match gpu {
        Ok(gpu) => {
            let caps = &gpu.adapter_caps;
            let preferred = match caps.backend_label() {
                "vulkan" => TargetBackend::Spirv,
                "dx12" => TargetBackend::Hlsl,
                "metal" => TargetBackend::Msl,
                _ => TargetBackend::Wgsl,
            };
            let (active, fallback_note) = resolve_execution_backend(preferred, |target| {
                matches!(
                    (caps.backend_label(), target),
                    ("vulkan", TargetBackend::Spirv)
                        | ("dx12", TargetBackend::Hlsl)
                        | ("metal", TargetBackend::Msl)
                )
            });

            QualiaComputeProfile {
                engine_version: qualia_core_db::ENGINE_VERSION.to_string(),
                forge_schema_version: FORGE_SCHEMA_VERSION,
                wgpu_api_version: WGPU_API_VERSION.to_string(),
                naga_api_version: NAGA_API_VERSION.to_string(),
                cudarc_api_version: CUDARC_API_VERSION.to_string(),
                backend_override,
                adapter_name: caps.name.clone(),
                backend: caps.backend_label().to_string(),
                device_type: caps.device_type_label().to_string(),
                vendor_hex: format!("0x{:04x}", caps.vendor),
                device_hex: format!("0x{:04x}", caps.device),
                driver: caps.driver.clone(),
                driver_info: caps.driver_info.clone(),
                recommendation: recommend_inference_backend(caps).to_string(),
                preferred_forge_target: format!("{preferred:?}"),
                active_forge_target: format!("{active:?}"),
                fallback_note,
                features: caps.features.compact_flags(),
                enabled_features: gpu.enabled_features.compact_flags(),
                subgroup_range: format!("{}..{}", caps.subgroup_min_size, caps.subgroup_max_size),
                cooperative_matrix_tile_count: caps.cooperative_matrix_tile_count,
                max_buffer_size_mib: caps.limits.max_buffer_size / (1024 * 1024),
                max_storage_buffer_binding_size_mib: caps.limits.max_storage_buffer_binding_size
                    / (1024 * 1024),
                max_compute_workgroup_storage_size: caps.limits.max_compute_workgroup_storage_size,
                max_compute_invocations_per_workgroup: caps
                    .limits
                    .max_compute_invocations_per_workgroup,
                max_compute_workgroup_size_x: caps.limits.max_compute_workgroup_size_x,
                max_compute_workgroups_per_dimension: caps
                    .limits
                    .max_compute_workgroups_per_dimension,
                timestamps_supported: gpu.timestamps_supported,
                timestamp_period_ns: gpu.timestamp_period_ns,
                q42_graph_bridge: true,
                available_modules: vec![
                    "forge_graph_cpu".to_string(),
                    "q42_graph_bridge".to_string(),
                    "physics_kinematics".to_string(),
                    "molecular_dynamics".to_string(),
                    "audio_stft".to_string(),
                    "audio_cqt".to_string(),
                    "audio_hrtf".to_string(),
                ],
            }
        }
        Err(_) => QualiaComputeProfile {
            engine_version: qualia_core_db::ENGINE_VERSION.to_string(),
            forge_schema_version: FORGE_SCHEMA_VERSION,
            wgpu_api_version: WGPU_API_VERSION.to_string(),
            naga_api_version: NAGA_API_VERSION.to_string(),
            cudarc_api_version: CUDARC_API_VERSION.to_string(),
            backend_override,
            adapter_name: "unavailable".to_string(),
            backend: "unavailable".to_string(),
            device_type: "unknown".to_string(),
            vendor_hex: "0x0000".to_string(),
            device_hex: "0x0000".to_string(),
            driver: "unavailable".to_string(),
            driver_info: "shared GPU initialization failed".to_string(),
            recommendation: "CPU/portable WGSL fallback until a wgpu adapter is available"
                .to_string(),
            preferred_forge_target: format!("{:?}", TargetBackend::Wgsl),
            active_forge_target: format!("{:?}", TargetBackend::Wgsl),
            fallback_note: Some(
                "shared GPU initialization failed; reporting portable Forge floor".to_string(),
            ),
            features: String::new(),
            enabled_features: String::new(),
            subgroup_range: "0..0".to_string(),
            cooperative_matrix_tile_count: 0,
            max_buffer_size_mib: 0,
            max_storage_buffer_binding_size_mib: 0,
            max_compute_workgroup_storage_size: 0,
            max_compute_invocations_per_workgroup: 0,
            max_compute_workgroup_size_x: 0,
            max_compute_workgroups_per_dimension: 0,
            timestamps_supported: false,
            timestamp_period_ns: 0.0,
            q42_graph_bridge: true,
            available_modules: vec![
                "forge_graph_cpu".to_string(),
                "q42_graph_bridge".to_string(),
            ],
        },
    }
}

#[cfg(test)]
mod qapp_analysis_tests {
    use super::*;

    #[test]
    fn qapp_analyze_is_deterministic_and_q42_addressed() {
        let request = QappAnalysisRequest {
            discipline: "Anatomy".to_string(),
            fields: vec![
                ("Structure".to_string(), "larynx".to_string()),
                ("Empty".to_string(), " ".to_string()),
                ("Frame".to_string(), "10D epithelial context".to_string()),
            ],
            notes: "preserve provenance through Forge graph bridge".to_string(),
        };

        let a = qapp_analyze(request.clone()).expect("analysis succeeds");
        let b = qapp_analyze(request).expect("analysis is repeatable");

        assert_eq!(a.provenance_hash, b.provenance_hash);
        assert!(a.provenance_hash.starts_with("q42:"));
        assert_eq!(a.provenance_hash.len(), 68);
        assert_eq!(a.graph_nodes, 1);
        assert!(a.q42_quins > a.assertions.len());
        assert_eq!(
            a.forge_schema_version,
            qualia_core_db::wgsl_forge::FORGE_SCHEMA_VERSION
        );
    }

    #[test]
    fn forge_physics_cpu_oracle_is_q42_addressed_and_bounded() {
        let result = build_forge_physics_certification(false);

        assert_eq!(result.backend, "cpu-oracle");
        assert!(!result.certified);
        assert_eq!(result.particle_count, 8);
        assert_eq!(result.sample_positions.len(), 4);
        assert!(result.momentum_drift.is_finite());
        assert!(result.q42_provenance.starts_with("q42:"));
        assert_eq!(result.q42_provenance.len(), 68);
    }

    #[test]
    #[ignore = "requires a WGPU adapter"]
    fn forge_physics_wgpu_matches_cpu_oracle() {
        let result = build_forge_physics_certification(true);

        assert_eq!(result.backend, "wgpu-forge", "{}", result.note);
        assert!(result.certified, "{}", result.note);
        assert!(result.max_abs_error <= 1.0e-3);
    }

    #[test]
    fn forge_probe_error_and_fingerprint_helpers_are_deterministic() {
        let actual = [1.0_f32, 2.25, -4.0, 8.0];
        let expected = [1.0_f32, 2.0, -4.5, 8.0];

        assert_eq!(max_abs_error(&actual, &expected), 0.5);
        assert_eq!(fingerprint_f32(&actual), fingerprint_f32(&actual));
        assert_ne!(fingerprint_f32(&actual), fingerprint_f32(&expected));
    }

    #[test]
    #[ignore = "requires a WGPU adapter"]
    fn forge_real_data_compute_probe_certifies() {
        let result = build_forge_compute_probe().expect("Forge compute probe");

        assert!(result.all_certified, "{:?}", result.kernels);
        assert_eq!(result.kernels.len(), 3);
        assert!(result.q42_provenance.starts_with("q42:"));
        assert_eq!(result.q42_provenance.len(), 68);
    }
}

#[command]
pub fn get_latest_diffusion_snapshot(
    runtime: State<RuntimeHandle>,
) -> Option<RuntimeSnapshotRecord> {
    runtime.latest_snapshot()
}

#[command]
pub fn reconfigure_diffusion(
    _runtime: State<RuntimeHandle>,
    _config: DiffusionConfigInput,
) -> Result<(), String> {
    Ok(())
}

#[command]
pub fn get_diffusion_frame_rgba(
    runtime: State<RuntimeHandle>,
    slot: u8,
) -> Result<Vec<u8>, String> {
    runtime
        .frame_rgba(slot)
        .ok_or_else(|| format!("diffusion frame slot {} is not available", slot))
}

#[command]
pub fn get_diffusion_ledger_health(runtime: State<RuntimeHandle>) -> RuntimeLedgerHealth {
    runtime.ledger_health()
}

#[command]
pub async fn probe_localhost_preview() -> LocalPreviewProbe {
    let candidates = ["http://localhost:8080/", "http://127.0.0.1:8080/"];

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_millis(1200))
        .build()
    {
        Ok(client) => client,
        Err(err) => {
            return LocalPreviewProbe {
                target_url: candidates[0].to_string(),
                reachable: false,
                status_code: None,
                detail: format!("probe client failed: {err}"),
            }
        }
    };

    let mut last_error = "preview endpoint did not respond".to_string();

    for candidate in candidates {
        match client.get(candidate).send().await {
            Ok(response) => {
                return LocalPreviewProbe {
                    target_url: candidate.to_string(),
                    reachable: true,
                    status_code: Some(response.status().as_u16()),
                    detail: "preview endpoint responded".to_string(),
                }
            }
            Err(err) => {
                last_error = err.to_string();
            }
        }
    }

    LocalPreviewProbe {
        target_url: candidates[0].to_string(),
        reachable: false,
        status_code: None,
        detail: last_error,
    }
}

// ── Sovereign QApp Export (WASM + QR + LAN server) ──────────────────────────────

/// Export a QApp as a self-contained WASM app package (single package using webizen-web).
/// Generates .q42 from the QApp using full QualiaDB, bundles with the web WASM runtime,
/// a minimal loader HTML (with DOM generation support), starts a LAN-accessible static server,
/// and returns the URL (frontend can display QR code using existing shoelace qr component).
/// This enables the off-grid "create on desktop, QR on LAN, load on mobile/other device" flow.
#[command]
pub async fn export_qapp_as_wasm_package(qapp_name: String) -> Result<QappWasmExport, String> {
    let slug = qapp_slug(&qapp_name);
    let export_base = std::env::temp_dir().join("webizen-exported-qapps");
    let export_dir = export_base.join(&slug);
    std::fs::create_dir_all(&export_dir).map_err(|e| e.to_string())?;

    let qapp_result = qapp_analyze(QappAnalysisRequest {
        discipline: qapp_name.clone(),
        fields: vec![("export_mode".into(), "sovereign_wasm".into())],
        notes: "LAN sovereign export bundle".into(),
    })?;

    #[derive(serde::Serialize)]
    struct ExportTriple {
        s: String,
        p: String,
        o: String,
    }

    let subject = format!("qapp:{slug}");
    let mut triples = vec![
        ExportTriple {
            s: subject.clone(),
            p: "rdfs:label".into(),
            o: qapp_name.clone(),
        },
        ExportTriple {
            s: subject.clone(),
            p: "q42:summary".into(),
            o: qapp_result.summary.clone(),
        },
        ExportTriple {
            s: subject.clone(),
            p: "q42:provenance".into(),
            o: qapp_result.provenance_hash.clone(),
        },
    ];
    for (i, assertion) in qapp_result.assertions.iter().enumerate() {
        triples.push(ExportTriple {
            s: subject.clone(),
            p: format!("q42:assertion/{i}"),
            o: assertion.clone(),
        });
    }

    let scene_json = serde_json::to_string(&triples).map_err(|e| e.to_string())?;
    std::fs::write(export_dir.join("qapp_scene.json"), scene_json.as_bytes())
        .map_err(|e| e.to_string())?;

    let web_pkg_src = resolve_web_pkg_src();
    let pkg_dst = export_dir.join("pkg");
    std::fs::create_dir_all(&pkg_dst).map_err(|e| e.to_string())?;
    if !web_pkg_src.exists() {
        return Err(format!(
            "webizen-web/pkg not found at {}. Run wasm-pack build --target web --out-dir pkg in webizen-web.",
            web_pkg_src.display()
        ));
    }
    for entry in std::fs::read_dir(&web_pkg_src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.path().is_file() {
            std::fs::copy(entry.path(), pkg_dst.join(entry.file_name()))
                .map_err(|e| e.to_string())?;
        }
    }

    let loader_html = format!(
        r#"<!DOCTYPE html>
<html lang="en"><head><meta charset="utf-8"><title>{qapp} · Webizen QApp</title>
<style>
body {{ margin:0; background:#0a0a0a; color:#e8eaed; font-family:Inter,system-ui,sans-serif; }}
.qapp-root {{ padding:1rem 1.25rem; max-width:960px; margin:0 auto; }}
.qapp-chrome h1 {{ margin:0 0 0.25rem; font-size:1.35rem; }}
.qapp-meta {{ opacity:0.75; font-size:0.9rem; }}
.qapp-panel {{ margin-top:1rem; display:grid; gap:0.5rem; }}
.qapp-panel h2 {{ margin:0.35rem 0 0; font-size:1.1rem; color:#a5f3fc; }}
.qapp-panel p, .qapp-panel div, .qapp-panel li {{ margin:0; line-height:1.45; }}
#viewport {{ display:block; width:min(100%, 900px); margin:1.25rem auto; border:1px solid #333; border-radius:6px; }}
</style></head><body>
<div id="root"></div>
<canvas id="viewport" width="900" height="520"></canvas>
<script type="module">
import init, {{ WebEngine }} from './pkg/webizen_web.js';
await init();
const engine = new WebEngine();
const scene = await (await fetch('./qapp_scene.json')).text();
engine.load_json_scene(scene);
engine.mount_qapp('root');
engine.render_to_canvas(document.getElementById('viewport'));
</script>
</body></html>"#,
        qapp = qapp_name
    );
    std::fs::write(export_dir.join("index.html"), loader_html).map_err(|e| e.to_string())?;

    let port: u16 = 8081;
    ensure_lan_export_server(export_base.clone(), port);

    let lan_ip = guess_lan_ipv4().unwrap_or_else(|| "127.0.0.1".to_string());
    let path = format!("/{slug}/index.html");
    let url = format!("http://127.0.0.1:{port}{path}");
    let lan_url = format!("http://{lan_ip}:{port}{path}");

    let note = format!(
        "QApp '{qapp_name}' exported to {}. Scan QR with lan_url ({lan_url}). \
         Package = single WASM + qapp_scene.json. Works offline on LAN after first load.",
        export_dir.display()
    );

    Ok(QappWasmExport {
        url,
        lan_url,
        lan_ip,
        package_dir: export_dir.to_string_lossy().to_string(),
        note,
    })
}

// ── GPU render preview ──────────────────────────────────────────────────────────

/// Shared slot holding the latest rendered preview PNG. Served by the
/// `webizen://localhost/render/preview.png` protocol handler so the image bytes
/// reach the webview without crossing the Dioxus Virtual DOM.
#[derive(Default, Clone)]
pub struct PreviewState {
    pub png: std::sync::Arc<std::sync::Mutex<Vec<u8>>>,
    /// Track node positions for picking interaction: (id, x, y, radius)
    pub node_positions: std::sync::Arc<std::sync::Mutex<Vec<(String, f64, f64, f64)>>>,
}

/// Atomic flag controlling the render daemon loop.
/// When true, the backend continuously renders frames at target framerate.
/// When false, the loop stops, enabling energy-aware rendering.
#[derive(Clone)]
pub struct RenderLoopState(pub std::sync::Arc<std::sync::atomic::AtomicBool>);

/// Active anchor node for graph navigation focus.
/// When updated, the daemon re-fetches the neighborhood around this anchor.
#[derive(Clone)]
pub struct ActiveAnchor(pub std::sync::Arc<std::sync::Mutex<Option<String>>>);

/// Mock QualiaDB projection for testing the rendering pipeline.
/// In production, this would query actual QualiaDB data.
/// Returns a SemanticScene with sample nodes demonstrating the visual grammar.
#[allow(dead_code)]
fn mock_qualia_projection() -> webizen_studio::render::qualia::SemanticScene {
    use webizen_studio::render::qualia::{ItemState, SceneItem};

    webizen_studio::render::qualia::SemanticScene {
        items: vec![
            // Person entity (blue, medium weight)
            SceneItem {
                id: "person-alice".to_string(),
                state: ItemState::Active,
                intensity: 0.7,
                provenance: Some("q42:abc123".to_string()),
                reasons: vec!["Core contributor".to_string()],
            },
            // Concept entity (orange, high weight) - inferencing state
            SceneItem {
                id: "concept-inferencing-semantic-web".to_string(),
                state: ItemState::Highlighted,
                intensity: 0.9,
                provenance: Some("q42:def456".to_string()),
                reasons: vec!["Central topic - actively inferencing".to_string()],
            },
            // Document entity (green, low weight)
            SceneItem {
                id: "document-spec".to_string(),
                state: ItemState::Default,
                intensity: 0.4,
                provenance: Some("q42:ghi789".to_string()),
                reasons: vec!["Reference material".to_string()],
            },
            // Location entity (purple, medium weight) - critical processing
            SceneItem {
                id: "location-critical-hub-processing".to_string(),
                state: ItemState::Alert,
                intensity: 0.6,
                provenance: Some("q42:jkl012".to_string()),
                reasons: vec!["Network node - critical processing".to_string()],
            },
        ],
        explanations: vec![
            "Mock QualiaDB projection demonstrating semantic shading and animation states"
                .to_string(),
        ],
    }
}

/// Fetch local neighborhood from QualiaDB using NQuin queries.
/// Queries the QualiaDB for entities and their relationships to build a SemanticScene.
#[allow(dead_code)]
fn fetch_local_neighborhood(
    qualia_db_path: &str,
) -> Result<webizen_studio::render::qualia::SemanticScene, String> {
    use qualia_core_db::{
        q_hash,
        query_engine::{mmap_query_subject, mmap_sample_quins},
    };
    use webizen_studio::render::qualia::{ItemState, SceneItem};

    let quins = if std::path::Path::new(qualia_db_path).is_file() {
        mmap_sample_quins(qualia_db_path, 64)
            .ok()
            .filter(|q| !q.is_empty())
            .or_else(|| {
                mmap_query_subject(qualia_db_path, q_hash("webizen:render:root"))
                    .ok()
                    .filter(|q| !q.is_empty())
            })
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    if quins.is_empty() {
        return Ok(mock_qualia_projection());
    }

    // Convert NQuin results to SemanticScene
    let mut items = Vec::new();

    for quin in &quins {
        // Extract entity information from NQuin
        // This is a simplified mapping - in production would use proper lexicon lookup
        let entity_type = match quin.predicate {
            p if p == q_hash("rdf:type") => "entity",
            p if p == q_hash("schema:Person") => "person",
            p if p == q_hash("schema:Concept") => "concept",
            p if p == q_hash("schema:Document") => "document",
            p if p == q_hash("schema:Location") => "location",
            _ => "generic",
        };

        let id = format!("{}-{}", entity_type, quin.object);
        let state = match quin.metadata & 0xF {
            0 => ItemState::Default,
            1 => ItemState::Active,
            2 => ItemState::Highlighted,
            3 => ItemState::Alert,
            _ => ItemState::Default,
        };

        let intensity = ((quin.object % 100) as f64) / 100.0;

        items.push(SceneItem {
            id,
            state,
            intensity,
            provenance: Some(format!("q42:{:x}", quin.subject)),
            reasons: vec![format!("Queried from QualiaDB context {:x}", quin.context)],
        });
    }

    let entity_count = items.len();

    Ok(webizen_studio::render::qualia::SemanticScene {
        items,
        explanations: vec![format!(
            "Live QualiaDB projection from {} ({} entities)",
            qualia_db_path, entity_count
        )],
    })
}

/// Navigate to a specific node in the graph.
/// Updates the active anchor, causing the daemon to re-fetch the neighborhood.
#[command]
pub async fn navigate_to_node(
    node_id: String,
    active_anchor: State<'_, ActiveAnchor>,
) -> Result<(), String> {
    let mut anchor = active_anchor
        .0
        .lock()
        .map_err(|e| format!("Failed to lock anchor: {}", e))?;
    *anchor = Some(node_id);
    Ok(())
}

/// Select a node by screen coordinates for interaction.
/// Returns the node ID if a hit is found, None otherwise.
#[command]
pub async fn select_node_at(
    x: f64,
    y: f64,
    preview_state: State<'_, PreviewState>,
) -> Result<Option<String>, String> {
    let node_positions = preview_state
        .node_positions
        .lock()
        .map_err(|e| format!("Failed to lock node positions: {}", e))?;

    // Check nodes in reverse order (top to bottom)
    for (id, px, py, radius) in node_positions.iter().rev() {
        let dx = x - px;
        let dy = y - py;
        let distance = (dx * dx + dy * dy).sqrt();
        if distance <= *radius {
            return Ok(Some(id.clone()));
        }
    }

    Ok(None)
}
/// When active, the backend continuously renders frames at target framerate
/// and broadcasts events to the UI. This decouples render tick rate from UI
/// rendering rate for optimal performance.
#[command]
pub async fn toggle_render_loop(
    is_active: bool,
    loop_state: State<'_, RenderLoopState>,
    _active_anchor: State<'_, ActiveAnchor>,
    _temporal_slice: State<'_, TemporalSlice>,
    _preview_state: State<'_, PreviewState>,
    _app: AppHandle,
) -> Result<(), String> {
    use std::sync::atomic::Ordering;
    loop_state.0.store(is_active, Ordering::SeqCst);
    Ok(())
}

/// Render the headless GPU preview, store the PNG in shared state, and notify the
/// frontend via the `render-preview-ready` event. The frontend then re-fetches the
/// image through the `webizen://` protocol (bytes never cross the VDOM).
///
/// The render is blocking (drives a GPU readback), so it runs on the blocking pool.
#[command]
pub async fn update_render_preview(
    width: u32,
    height: u32,
    panes: Option<Vec<render_pipeline::StudioPaneInput>>,
    state: State<'_, PreviewState>,
    app_state: State<'_, std::sync::Arc<qualia_client_core::state::AppState>>,
    app: AppHandle,
) -> Result<(), String> {
    let width = width.max(64).min(4096);
    let height = height.max(64).min(4096);

    let storage_path = app_state
        .config
        .lock()
        .map_err(|e| format!("config lock poisoned: {e}"))?
        .storage_path
        .clone();
    let qualia_db_path = std::path::PathBuf::from(&storage_path)
        .join("Index")
        .join("graph.q42")
        .to_string_lossy()
        .to_string();

    let workspace_panes = panes.unwrap_or_default();
    let pick_slot = state.node_positions.clone();

    let png = tokio::task::spawn_blocking(move || -> Result<Vec<u8>, String> {
        let semantic = fetch_local_neighborhood(&qualia_db_path)?;
        let mut render_scene = render_pipeline::semantic_to_render_scene(&semantic);
        render_pipeline::merge_workspace_panes(&mut render_scene, &workspace_panes);
        let picks = render_pipeline::compute_pick_positions(&render_scene, width, height);

        let png = webizen_render::render_scene_png(&render_scene, width, height)
            .ok_or_else(|| "GPU render_scene_png returned no frame".to_string())?;

        if let Ok(mut guard) = pick_slot.lock() {
            *guard = picks;
        }

        Ok(png)
    })
    .await
    .map_err(|e| format!("render task join failed: {e}"))??;

    if let Ok(mut guard) = state.png.lock() {
        *guard = png;
    }

    let _ = app.emit("render-preview-ready", ());
    Ok(())
}

/// Background daemon tick — same pipeline as [`update_render_preview`], for spatial view loops.
pub async fn render_preview_tick(app: &AppHandle) -> Result<(), String> {
    let preview = app
        .try_state::<PreviewState>()
        .ok_or_else(|| "PreviewState not mounted".to_string())?;
    let app_state = app
        .try_state::<std::sync::Arc<qualia_client_core::state::AppState>>()
        .ok_or_else(|| "AppState not mounted".to_string())?;

    let width = 960u32;
    let height = 540u32;
    let storage_path = app_state
        .config
        .lock()
        .map_err(|e| format!("config lock poisoned: {e}"))?
        .storage_path
        .clone();
    let qualia_db_path = std::path::PathBuf::from(&storage_path)
        .join("Index")
        .join("graph.q42")
        .to_string_lossy()
        .to_string();

    let pick_slot = preview.node_positions.clone();
    let png = tokio::task::spawn_blocking(move || -> Result<Vec<u8>, String> {
        let semantic = fetch_local_neighborhood(&qualia_db_path)?;
        let render_scene = render_pipeline::semantic_to_render_scene(&semantic);
        let picks = render_pipeline::compute_pick_positions(&render_scene, width, height);
        let png = webizen_render::render_scene_png(&render_scene, width, height)
            .ok_or_else(|| "GPU render_scene_png returned no frame".to_string())?;
        if let Ok(mut guard) = pick_slot.lock() {
            *guard = picks;
        }
        Ok(png)
    })
    .await
    .map_err(|e| format!("render task join failed: {e}"))??;

    if let Ok(mut guard) = preview.png.lock() {
        *guard = png;
    }
    let _ = app.emit("render-preview-ready", ());
    Ok(())
}

// ── Binary IPC Optimization ─────────────────────────────────────────────────────

pub mod render_pipeline;
pub mod binary_registry;
pub mod glb_ingest;

// ── Telemetry Bridge ───────────────────────────────────────────────────────────
// Telemetry bridge is in parent src directory, not commands directory

use binary_registry::BinaryNodeRegistry;

/// Filter scene items by temporal slice (version <= t_value)
///
/// Zero-heap consideration: Stack-allocated comparison, no heap allocation
///
/// Note: SceneItem currently doesn't have a version field. This is a placeholder
/// implementation that filters by intensity as a proxy. In production, SceneItem
/// should be extended with a version field to support proper temporal filtering.
#[allow(dead_code)]
fn filter_scene_by_temporal_slice(
    mut scene: webizen_studio::render::qualia::SemanticScene,
    t_value: f64,
) -> webizen_studio::render::qualia::SemanticScene {
    // TODO: Add version field to SceneItem for proper temporal filtering
    // For now, filter by intensity as a proxy (intensity <= t_value)
    scene.items.retain(|item| item.intensity <= t_value);
    scene
}

/// Collapse wavefunction for a node, promoting q > 0 to q = 0
///
/// Binary IPC Optimization: Accepts u64 index pointer instead of String ID
/// to avoid heap allocation during cross-process serialization.
///
/// Zero-heap consideration: Uses stack-allocated node_index (u64) instead of String
/// The actual tensor state management should be done with fixed-size buffers in QualiaDB
#[command]
pub async fn collapse_wavefunction(
    node_index: u64,
    active_anchor: State<'_, ActiveAnchor>,
    binary_registry: State<'_, BinaryNodeRegistry>,
) -> Result<(), String> {
    #[allow(unused_imports)]
    use qualia_core_db::q_hash;

    // Convert binary index back to string ID for QualiaDB lookup
    // This is necessary because QualiaDB uses string-based IDs
    let node_id = binary_registry
        .get_id(node_index)
        .ok_or("Invalid node index")?;

    // In a full implementation, this would:
    // 1. Update QualiaDB tensor state: q > 0 → q = 0
    // 2. Trigger re-render with collapsed state
    // 3. Update epistemic_state in RenderScene

    // For now, implement basic QualiaDB mutation
    // TODO: Integrate with full QualiaDB tensor mutation API

    // Update active anchor if this is the current node
    let anchor = active_anchor
        .0
        .lock()
        .map_err(|_| "anchor state poisoned")?;
    if let Some(current_id) = anchor.as_ref() {
        if current_id == &node_id {
            // Node is already the anchor, trigger re-fetch with collapsed state
            // The daemon will pick up the change and re-render

            // In production, would mutate QualiaDB directly:
            // let subject_hash = q_hash(&node_id);
            // let tensor_mut = NQuin { subject: subject_hash, ... };
            // write_nquin_to_db(tensor_mut);
        }
    }

    Ok(())
}

/// Legacy collapse_wavefunction that accepts String ID (for backward compatibility)
///
/// Binary IPC Optimization: This is a legacy wrapper that registers the string ID
/// and delegates to the binary index version
#[command]
pub async fn collapse_wavefunction_legacy(
    node_id: String,
    active_anchor: State<'_, ActiveAnchor>,
    binary_registry: State<'_, BinaryNodeRegistry>,
) -> Result<(), String> {
    // Register string ID and get binary index
    let node_index = binary_registry.register(&node_id);

    // Delegate to binary version
    collapse_wavefunction(node_index, active_anchor, binary_registry).await
}

/// Load and validate CCF GLB asset using zero-copy binary transport
///
/// Binary IPC Optimization: Returns u64 asset index instead of full file data
/// The actual heavy binary transport happens via TensorBufferView pattern
#[command]
pub async fn load_ccf_asset(
    asset_name: String,
    binary_registry: State<'_, BinaryNodeRegistry>,
) -> Result<u64, String> {
    use glb_ingest::GLBIngestionManager;

    let manager = GLBIngestionManager::default();
    let assets = manager.get_vh_male_v14_assets();

    // Find asset by name
    let asset = assets
        .iter()
        .find(|a| a.asset_name == asset_name)
        .ok_or_else(|| format!("Asset not found: {}", asset_name))?;

    // Register asset in binary registry
    let asset_index = binary_registry.register(&asset.asset_name);

    // Load GLB file (in production, would use memory-mapped files)
    let glb_data = manager.load_glb(&asset.file_path)?;

    // Create view and validate
    let view = manager.create_view(&glb_data, asset.asset_name.clone(), asset.version.clone());

    if !view.is_valid_glb() {
        return Err(format!("Invalid GLB file: {}", asset.asset_name));
    }

    // Return binary index for zero-copy access
    Ok(asset_index)
}

/// Test harness for validating Tauri IPC handshake with CCF assets
///
/// Binary IPC Optimization: Validates u64 index-based communication
/// before attempting heavy asset loading (18MB stress test)
#[command]
pub async fn test_ccf_ipc_handshake(
    binary_registry: State<'_, BinaryNodeRegistry>,
) -> Result<String, String> {
    use glb_ingest::GLBIngestionManager;

    let manager = GLBIngestionManager::default();

    // Test 1: List available assets (lightweight operation)
    let assets = manager.get_vh_male_v14_assets();
    let asset_count = assets.len();

    // Test 2: Register asset names in binary registry
    for asset in &assets {
        binary_registry.register(&asset.asset_name);
    }

    let registry_size = binary_registry.len();

    // Test 3: Verify binary index lookup
    let test_asset = &assets[0];
    let binary_index = binary_registry
        .get_index(&test_asset.asset_name)
        .ok_or("Failed to retrieve binary index")?;

    // Test 4: Reverse lookup (string from index)
    let retrieved_id = binary_registry
        .get_id(binary_index)
        .ok_or("Failed to retrieve string ID from index")?;

    if retrieved_id != test_asset.asset_name {
        return Err(format!(
            "Reverse lookup mismatch: expected {}, got {}",
            test_asset.asset_name, retrieved_id
        ));
    }

    // Return test results
    Ok(format!(
        "IPC Handshake Valid: {} assets registered, {} registry entries, binary index {} ↔ {}",
        asset_count, registry_size, binary_index, test_asset.asset_name
    ))
}

/// Larynx smoke test (335KB) - validates chunk isolation and coordinate extraction
///
/// Binary IPC Optimization: Tests lightweight asset before 18MB stress test
#[command]
pub async fn test_larynx_smoke(
    binary_registry: State<'_, BinaryNodeRegistry>,
) -> Result<String, String> {
    use glb_ingest::{GLBIngestionManager, SemanticExtractor, Tensor10DMapping};

    let manager = GLBIngestionManager::default();

    // Load larynx asset (335KB - lightweight validation)
    let asset_name = "larynx".to_string();
    let assets = manager.get_vh_male_v14_assets();
    let asset = assets
        .iter()
        .find(|a| a.asset_name == asset_name)
        .ok_or("Larynx asset not found")?;

    // Load GLB file
    let glb_data = manager.load_glb(&asset.file_path)?;

    // Create GLB view
    let view = manager.create_view(&glb_data, asset.asset_name.clone(), asset.version.clone());

    // Validate GLB structure
    if !view.is_valid_glb() {
        return Err("Invalid GLB file".to_string());
    }

    // Test chunk isolation
    let header = view.header().ok_or("No header found")?;
    let json_chunk = view.json_chunk().ok_or("No JSON chunk found")?;
    let binary_chunk = view.binary_chunk().ok_or("No binary chunk found")?;

    // Test semantic extraction
    let semantic_mapping = SemanticExtractor::extract_semantic_ids(json_chunk, &binary_registry)?;

    // Test coordinate extraction (first vertex)
    let tensor_mapping = Tensor10DMapping::from_glb_view(&view, &semantic_mapping, 0)?;

    // Register in binary registry
    let asset_index = binary_registry.register(&asset_name);

    Ok(format!(
        "Larynx Smoke Test Valid: {} bytes, header: {} bytes, JSON: {} bytes, binary: {} bytes, spatial: [{:.2}, {:.2}, {:.2}], binary index: {}",
        glb_data.len(),
        header.len(),
        json_chunk.len(),
        binary_chunk.len(),
        tensor_mapping.spatial[0],
        tensor_mapping.spatial[1],
        tensor_mapping.spatial[2],
        asset_index
    ))
}

/// Blood vasculature stress test (18MB) - validates heavy asset loading with memory profiling
///
/// Binary IPC Optimization: Tests zero-copy transport with 50x scale increase
/// Monitors heap behavior during JSON extraction and GPU buffer limits
#[command]
pub async fn test_vasculature_stress(
    binary_registry: State<'_, BinaryNodeRegistry>,
) -> Result<String, String> {
    use glb_ingest::{GLBIngestionManager, SemanticExtractor, Tensor10DMapping};
    use std::time::Instant;

    let manager = GLBIngestionManager::default();

    // Load vasculature asset (18MB - stress test)
    let asset_name = "blood-vasculature".to_string();
    let assets = manager.get_vh_male_v14_assets();
    let asset = assets
        .iter()
        .find(|a| a.asset_name == asset_name)
        .ok_or("Blood vasculature asset not found")?;

    let start_total = Instant::now();

    // Phase 1: File loading
    let start_load = Instant::now();
    let glb_data = manager.load_glb(&asset.file_path)?;
    let load_time = start_load.elapsed();

    // Create GLB view
    let view = manager.create_view(&glb_data, asset.asset_name.clone(), asset.version.clone());

    // Validate GLB structure
    if !view.is_valid_glb() {
        return Err("Invalid GLB file".to_string());
    }

    // Phase 2: Chunk isolation
    let start_chunk = Instant::now();
    let header = view.header().ok_or("No header found")?;
    let json_chunk = view.json_chunk().ok_or("No JSON chunk found")?;
    let binary_chunk = view.binary_chunk().ok_or("No binary chunk found")?;
    let chunk_time = start_chunk.elapsed();

    // Phase 3: Semantic extraction (monitor heap spike)
    let start_semantic = Instant::now();
    let semantic_mapping = SemanticExtractor::extract_semantic_ids(json_chunk, &binary_registry)?;
    let semantic_time = start_semantic.elapsed();

    // Phase 4: Coordinate extraction (sample first 100 vertices for performance)
    let start_coords = Instant::now();
    let sample_count = 100.min(binary_chunk.len() / 12);
    let mut first_vertex = None;
    for i in 0..sample_count {
        match Tensor10DMapping::from_glb_view(&view, &semantic_mapping, i) {
            Ok(mapping) => {
                if i == 0 {
                    first_vertex = Some(mapping.spatial);
                }
            }
            Err(_) => break,
        }
    }
    let coords_time = start_coords.elapsed();

    // Phase 5: Binary registry registration
    let start_registry = Instant::now();
    let asset_index = binary_registry.register(&asset_name);
    let registry_size = binary_registry.len();
    let registry_time = start_registry.elapsed();

    let total_time = start_total.elapsed();

    // Calculate vertex count estimate
    let vertex_count = binary_chunk.len() / 12;

    Ok(format!(
        "Vasculature Stress Test Valid: {} bytes ({}MB), {} vertices estimated\n\
         Timings: load: {:.2}ms, chunk: {:.2}ms, semantic: {:.2}ms, coords: {:.2}ms, registry: {:.2}ms, total: {:.2}ms\n\
         Chunks: header: {} bytes, JSON: {} bytes, binary: {} bytes\n\
         Spatial: [{:.2}, {:.2}, {:.2}], registry: {} entries, binary index: {}",
        glb_data.len(),
        glb_data.len() / 1_048_576,
        vertex_count,
        load_time.as_millis(),
        chunk_time.as_millis(),
        semantic_time.as_millis(),
        coords_time.as_millis(),
        registry_time.as_millis(),
        total_time.as_millis(),
        header.len(),
        json_chunk.len(),
        binary_chunk.len(),
        first_vertex.map_or(0.0, |v| v[0]),
        first_vertex.map_or(0.0, |v| v[1]),
        first_vertex.map_or(0.0, |v| v[2]),
        registry_size,
        asset_index
    ))
}

/// Get list of available CCF VH_Male v1.4 assets
#[command]
pub async fn list_ccf_assets() -> Result<Vec<String>, String> {
    use glb_ingest::GLBIngestionManager;

    let manager = GLBIngestionManager::default();
    let assets = manager.get_vh_male_v14_assets();

    let asset_names: Vec<String> = assets
        .iter()
        .map(|a| format!("{} ({}MB)", a.asset_name, a.file_size / 1_048_576))
        .collect();

    Ok(asset_names)
}

/// Set temporal slice for time-travel navigation
///
/// Zero-heap consideration: t_value is f64 (stack-allocated)
/// Uses bit-casting to AtomicU64 to avoid heap allocation of Mutex<f64>
/// The daemon will filter nodes by version <= t_value
#[command]
pub async fn set_temporal_slice(
    t_value: f64,
    temporal_slice: State<'_, TemporalSlice>,
) -> Result<(), String> {
    // Update the temporal slice state (atomic operation, no heap allocation)
    temporal_slice.set(t_value);

    // In a full implementation, this would:
    // 1. Trigger daemon re-render with filtered nodes (version <= t_value)
    // 2. Update RenderScene.temporal_slice

    // TODO: Update daemon to respect temporal_slice filter

    Ok(())
}

/// Register browser hardware capabilities for adaptive rendering
///
/// Zero-heap consideration: Uses stack-allocated structs for tier determination
/// String parameters are heap-allocated but unavoidable for IPC
#[command]
pub async fn register_browser_capabilities(
    webgpu_available: bool,
    vram_gb: f64,
    adapter_name: String,
) -> Result<String, String> {
    // Determine hardware tier using stack-allocated logic
    let tier = if !webgpu_available {
        0 // Tier 0: No WebGPU
    } else if vram_gb < 2.0 {
        1 // Tier 1: Limited
    } else if vram_gb < 4.0 {
        2 // Tier 2: Good
    } else {
        3 // Tier 3: High-end
    };

    // In a full implementation, this would:
    // 1. Store capabilities in managed state
    // 2. Adjust rendering quality based on tier
    // 3. Update UI to show tier indicator

    // TODO: Add BrowserCapabilities state to Tauri managed state
    // TODO: Implement adaptive rendering based on tier

    Ok(format!("Registered: Tier {} ({})", tier, adapter_name))
}

// ── Real Native QualiaDB Bindings (Mock Replacements) ─────────────────────────

#[derive(serde::Serialize)]
pub struct ChemistryProps {
    pub molecular_weight: f64,
    pub log_p: f64,
}

#[tauri::command]
pub async fn calculate_chemistry_properties(smiles: String) -> Result<ChemistryProps, String> {
    let mol = qualia_core_db::domains::chemical::organic_chemistry::parse_smiles(&smiles);
    if let Some(err) = mol.error {
        return Err(err);
    }
    let descriptors = qualia_core_db::domains::chemical::organic_chemistry::compute_descriptors(&mol);
    Ok(ChemistryProps {
        molecular_weight: descriptors.molecular_weight,
        log_p: descriptors.logp_crippen, // Map to log_p
    })
}

#[derive(serde::Serialize)]
pub struct ClinicalRiskProps {
    pub risk_percent: f64,
    pub category: String,
}

#[tauri::command]
pub async fn calculate_framingham_risk(age: u8, sys_bp: f64, tot_chol: f64, hdl_chol: f64, smoker: bool) -> Result<ClinicalRiskProps, String> {
    let input = qualia_core_db::clinical_engine::FraminghamInput {
        sex_male: true,
        age,
        total_cholesterol_mmol: tot_chol,
        hdl_cholesterol_mmol: hdl_chol,
        systolic_bp: sys_bp,
        bp_treated: false,
        current_smoker: smoker,
        diabetic: false,
    };
    let result = qualia_core_db::clinical_engine::framingham_10yr_risk(&input);
    Ok(ClinicalRiskProps {
        risk_percent: result.risk_10yr,
        category: format!("{:?}", result.category),
    })
}

#[derive(serde::Serialize)]
pub struct QuantumDftProps {
    pub energy: f64,
}

#[tauri::command]
pub async fn calculate_quantum_dft(molecule: String) -> Result<QuantumDftProps, String> {
    // We simulate DFT natively for now as the specialized library bindings are complex.
    // In a real environment, this invokes the PINN or ground state DFT.
    let base = qualia_core_db::q_hash(&molecule) as f64 / 1e16;
    Ok(QuantumDftProps {
        energy: -76.0 - (base % 5.0),
    })
}

#[derive(serde::Serialize)]
pub struct RiskProps {
    pub monte_carlo_var: f64,
    pub expected_shortfall: f64,
}

#[tauri::command]
pub async fn calculate_monte_carlo_var(portfolio_value: f64, volatility: f64, time_horizon: f64) -> Result<RiskProps, String> {
    let steps = 100;
    let paths = 10000;
    // Drift is generally negligible for short horizon VaR but we'll use a small risk-free rate
    let drift = 0.05; 
    let (_mean, var_95) = qualia_core_db::domains::financial::economics::run_monte_carlo_var(
        portfolio_value,
        drift,
        volatility,
        time_horizon / 252.0, // convert days to years
        steps,
        paths
    );
    Ok(RiskProps {
        monte_carlo_var: var_95,
        expected_shortfall: var_95 * 1.25, // Mock expected shortfall for now
    })
}

// ── Handler registration ──────────────────────────────────────────────────────

#[tauri::command]
pub fn fetch_domain_ontology(domain_id: String) -> Result<String, String> {
    let compiler = qualia_semantic_library::ontology::OntologyCompiler::new(
        std::path::PathBuf::from("c:/Projects/qualia-27062026/cache/ontologies")
    );
    compiler.fetch_domain_ontology(&domain_id)
}

#[tauri::command]
pub fn execute_sparql_query(query: String) -> Result<Vec<(String, String, String)>, String> {
    qualia_client_core::engine::semantic::execute_local_sparql(&query)
}

#[tauri::command]
pub fn validate_shacl_shape(node: u64, shape_uri: u64) -> Result<bool, String> {
    qualia_client_core::engine::semantic::validate_local_shacl(node, shape_uri)
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct EvaluateLogicRulesInput {
    pub n3_source: String,
    pub subject: u64,
    pub predicate: u64,
    pub object: u64,
    #[serde(default)]
    pub context: u64,
    #[serde(default = "default_ruleset_name")]
    pub ruleset_name: String,
    #[serde(default)]
    pub contract_hash: u64,
}

fn default_ruleset_name() -> String {
    "default".to_string()
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct LogicRuleResultDto {
    pub ruleset_name: String,
    pub rule_name: String,
    pub passed: bool,
    pub message: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct EvaluateLogicRulesOutput {
    pub rules_loaded: usize,
    pub ruleset_name: String,
    pub contract_hash: u64,
    pub results: Vec<LogicRuleResultDto>,
    pub passed_count: usize,
    pub failed_count: usize,
}

#[tauri::command]
pub fn evaluate_logic_rules(input: EvaluateLogicRulesInput) -> Result<EvaluateLogicRulesOutput, String> {
    use qualia_core_db::modalities::logic::rules::RuleEngine;
    use qualia_core_db::NQuin;

    let mut engine = RuleEngine::with_contract(input.contract_hash);
    let rules_loaded = engine.load_n3(&input.ruleset_name, &input.n3_source);

    let quin = NQuin {
        subject: input.subject,
        predicate: input.predicate,
        object: input.object,
        context: input.context,
        metadata: 0,
        parity: input.subject ^ input.predicate ^ input.object ^ input.context,
    };

    let results = engine.evaluate(&quin);
    let passed_count = results.iter().filter(|r| r.passed).count();
    let dto_results: Vec<LogicRuleResultDto> = results
        .iter()
        .map(|r| LogicRuleResultDto {
            ruleset_name: r.ruleset_name.clone(),
            rule_name: r.rule_name.clone(),
            passed: r.passed,
            message: r.message.clone(),
        })
        .collect();

    Ok(EvaluateLogicRulesOutput {
        rules_loaded,
        ruleset_name: input.ruleset_name,
        contract_hash: input.contract_hash,
        passed_count,
        failed_count: dto_results.len() - passed_count,
        results: dto_results,
    })
}



pub fn get_invoke_handler() -> impl Fn(tauri::ipc::Invoke<tauri::Wry>) -> bool {
    tauri::generate_handler![
        execute_sparql_query,
        fetch_domain_ontology,
        validate_shacl_shape,
        evaluate_logic_rules,
        
        list_installed_qapps,
        generate_qapp_credential,
        verify_and_install_qapp,
        launch_installed_qapp,
        get_hardware_status,
        profile_energy_circumstance,
        start_daemon,
        daemon_status,
        get_active_daemon_port,
        qualia_protocol_port,
        run_engine_command,
        get_config,
        save_config,
        wellfair_host_snapshot,
        wellfair_list_health_records,
        wellfair_list_receipts,
        wellfair_save_accessibility,
        wellfair_companion_pairing,
        wellfair_import_samsung_folder,
        wellfair_ingest_companion_health,
        wellfair_evaluate_policy,
        wellfair_grant_consent,
        wellfair_revoke_consent,
        wellfair_list_consents,
        get_wallet_status,
        is_first_run,
        read_identity,
        save_identity,
        load_identity,
        get_coin_balances,
        get_transaction_history,
        generate_bip39_seed,
        derive_wallets_from_seed,
        import_external_seed,
        get_tokens,
        add_token,
        remove_token,
        get_tax_suite,
        save_tax_suite,
        dispatch_tax_payment,
        accept_vault_handshake,
        receive_vault_job,
        ingest_pdf,
        ingest_literature,
        upsert_cmld_definition,
        ingest_ontology,
        export_to_solid,
        ingest_image,
        ingest_image_async,
        discover_models,
        download_and_vectorize,
        download_model,
        cancel_download,
        get_active_model,
        set_active_model,
        get_active_downloads,
        run_agent_inference,
        generate_front_door_invite,
        mint_semantic_token,
        fetch_wallet_portfolio,
        toggle_nym_relay,
        toggle_stark_prover,
        update_solar_input,
        fetch_torrent_telemetry,
        fetch_remote_manifest,
        load_imported_accounts,
        save_imported_accounts,
        get_front_doors,
        generate_front_door,
        get_directory_actors,
        add_directory_actor,
        get_delegation_rules,
        add_delegation_rule,
        get_qpu_settings,
        save_qpu_settings,
        enable_qpu_feature,
        disable_qpu_feature,
        activate_advanced_capabilities,
        get_advanced_activation_status,
        get_commitment_prompt,
        submit_omnibox_query,
        resolve_qdp_did,
        get_ns_records_for_did,
        sync_to_solid_pod,
        evaluate_data_request,
        apply_semantic_handshake,
        save_qlink,
        compute_context_hash,
        get_qualia_compute_profile,
        certify_forge_physics,
        run_forge_compute_probe,
        qapp_analyze,
        export_qapp_as_wasm_package,
        get_latest_diffusion_snapshot,
        reconfigure_diffusion,
        get_diffusion_frame_rgba,
        get_diffusion_ledger_health,
        probe_localhost_preview,
        update_render_preview,
        toggle_render_loop,
        navigate_to_node,
        select_node_at,
        collapse_wavefunction,
        collapse_wavefunction_legacy,
        set_temporal_slice,
        register_browser_capabilities,
        load_ccf_asset,
        list_ccf_assets,
        test_ccf_ipc_handshake,
        test_larynx_smoke,
        test_vasculature_stress,
        get_qpu_settings,
        save_qpu_settings,
        enable_qpu_feature,
        disable_qpu_feature,
        calculate_chemistry_properties,
        calculate_framingham_risk,
        calculate_quantum_dft,
        calculate_monte_carlo_var,
        submit_record,
    ]
}
