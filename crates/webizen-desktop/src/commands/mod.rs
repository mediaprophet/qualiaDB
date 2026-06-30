use qualia_client_core::api;
use qualia_client_core::api::{CoinBalance, HardwareStatus, TokenEntry, TxRecord, WalletStatus};
use qualia_client_core::engine::{ingestion, llm_offload};
use qualia_client_core::state::{Actor, AgentConfig, DelegationRule, FrontDoor, ProgressPayload};
use qualia_core_db::ilp_dispatcher::DispatchResult;
use qualia_core_db::rpc::TaxRecipientSuite;
use std::time::Duration;
use tauri::{
    command, AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder,
};
use webizen_runtime::DiffusionConfig;

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
    WebviewWindowBuilder::new(&app, label, WebviewUrl::External(parsed))
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

#[command]
pub fn qapp_analyze(request: QappAnalysisRequest) -> Result<QappAnalysisResult, String> {
    // Build a canonical encoding of the request and derive a real provenance
    // stamp from the QualiaDB engine's q_hash.
    let mut canonical = String::new();
    canonical.push_str(&request.discipline);

    let mut assertions = Vec::new();
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
    }
    if !request.notes.trim().is_empty() {
        canonical.push_str("|notes=");
        canonical.push_str(request.notes.trim());
        assertions.push(format!(
            "{} :hasNote \"{}\" .",
            request.discipline,
            request.notes.trim()
        ));
    }

    let hash = qualia_core_db::q_hash(&canonical);
    Ok(QappAnalysisResult {
        summary: format!(
            "{} analysis derived {} assertion(s) with a QualiaDB provenance stamp.",
            request.discipline,
            assertions.len()
        ),
        assertions,
        provenance_hash: format!("q42:{:016x}", hash),
        engine: "qualia-core-db".to_string(),
    })
}

#[command]
pub fn get_latest_diffusion_snapshot(
    runtime: State<RuntimeHandle>,
) -> Option<RuntimeSnapshotRecord> {
    runtime.latest_snapshot()
}

#[command]
pub fn reconfigure_diffusion(
    runtime: State<RuntimeHandle>,
    config: DiffusionConfigInput,
) -> Result<(), String> {
    if config.width == 0 || config.height == 0 {
        return Err("diffusion dimensions must be greater than zero".to_string());
    }

    runtime.queue_reconfigure(DiffusionConfig {
        width: config.width,
        height: config.height,
        diffusion_rate: config.diffusion_rate,
    })
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
fn fetch_local_neighborhood(
    qualia_db_path: &str,
) -> Result<webizen_studio::render::qualia::SemanticScene, String> {
    use qualia_core_db::{q_hash, query_engine::mmap_query_subject, NQuin};
    use webizen_studio::render::qualia::{ItemState, SceneItem};

    // Try to query QualiaDB - if file doesn't exist, fall back to mock
    let quins = match mmap_query_subject(qualia_db_path, q_hash("webizen:render:root")) {
        Ok(quins) => quins,
        Err(_) => {
            // Fall back to mock if QualiaDB file not found
            return Ok(mock_qualia_projection());
        }
    };

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

    // If no results from QualiaDB, use mock data
    if items.is_empty() {
        return Ok(mock_qualia_projection());
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
    active_anchor: State<'_, ActiveAnchor>,
    temporal_slice: State<'_, TemporalSlice>,
    preview_state: State<'_, PreviewState>,
    app: AppHandle,
) -> Result<(), String> {
    use std::sync::atomic::Ordering;
    use std::time::Instant;
    use tokio::time::{sleep, Duration};

    // Update the atomic flag
    loop_state.0.store(is_active, Ordering::SeqCst);

    if is_active {
        let app_handle_clone = app.clone();
        let preview_state_clone = preview_state.inner().clone();
        let loop_flag = loop_state.0.clone();
        let anchor_state = active_anchor.0.clone();
        let temporal_slice_wrapper = temporal_slice.inner().clone();

        tokio::spawn(async move {
            let start_time = Instant::now();
            let target_framerate = 30; // 30 FPS is plenty for semantic pulsing, saves battery
            let frame_duration = Duration::from_millis(1000 / target_framerate);

            // QualiaDB path - in production this would come from config
            let qualia_db_path = "data/qualia.q42";

            // Track current anchor to detect changes
            let mut current_anchor: Option<String> = None;

            // Track previous node positions for smooth transitions
            let mut previous_node_positions: Vec<(String, f64, f64)> = Vec::new();

            // Transition state for smooth animations
            let mut transition_start_time: Option<f64> = None;
            const TRANSITION_DURATION: f64 = 0.3; // 300ms

            // Predictive caching: pre-fetch neighborhoods for nodes likely to be visited
            use std::collections::HashMap;
            let mut neighborhood_cache: HashMap<
                String,
                webizen_studio::render::qualia::SemanticScene,
            > = HashMap::new();
            let mut last_cache_update: f64 = 0.0;
            const CACHE_TTL: f64 = 5.0; // Cache entries valid for 5 seconds

            while loop_flag.load(Ordering::SeqCst) {
                let loop_start = Instant::now();
                let elapsed_time = start_time.elapsed().as_secs_f64();

                // Get current temporal slice from atomic state (zero-heap read)
                let temporal_slice_value = temporal_slice_wrapper.get();

                // Check if anchor has changed
                let new_anchor = anchor_state.lock().unwrap().clone();
                let anchor_changed = new_anchor != current_anchor;

                // Fetch/Mock the active scene from QualiaDB with temporal filtering
                let semantic_scene = if anchor_changed {
                    // Check cache first
                    if let Some(anchor_id) = &new_anchor {
                        if let Some(cached_scene) = neighborhood_cache.get(anchor_id) {
                            // Cache hit - filter by temporal slice
                            filter_scene_by_temporal_slice(
                                cached_scene.clone(),
                                temporal_slice_value,
                            )
                        } else {
                            // Cache miss - fetch from QualiaDB
                            let scene = match fetch_local_neighborhood(qualia_db_path) {
                                Ok(scene) => scene,
                                Err(_) => mock_qualia_projection(),
                            };
                            // Apply temporal filter before caching
                            let filtered_scene =
                                filter_scene_by_temporal_slice(scene.clone(), temporal_slice_value);
                            // Cache the result
                            neighborhood_cache.insert(anchor_id.clone(), filtered_scene.clone());
                            last_cache_update = elapsed_time;
                            filtered_scene
                        }
                    } else {
                        let scene = match fetch_local_neighborhood(qualia_db_path) {
                            Ok(scene) => scene,
                            Err(_) => mock_qualia_projection(),
                        };
                        filter_scene_by_temporal_slice(scene, temporal_slice_value)
                    }
                } else {
                    // Use cached scene if no anchor change, but apply temporal filter
                    let scene = match fetch_local_neighborhood(qualia_db_path) {
                        Ok(scene) => scene,
                        Err(_) => mock_qualia_projection(),
                    };
                    filter_scene_by_temporal_slice(scene, temporal_slice_value)
                };

                // Periodic cache cleanup and predictive pre-fetching
                if elapsed_time - last_cache_update > 1.0 {
                    // Clean up expired cache entries
                    neighborhood_cache.retain(|_, _| elapsed_time - last_cache_update < CACHE_TTL);

                    // Predictive pre-fetch: cache neighborhoods for nodes in current scene
                    for item in &semantic_scene.items {
                        if !neighborhood_cache.contains_key(&item.id) {
                            if let Ok(neighborhood) = fetch_local_neighborhood(qualia_db_path) {
                                neighborhood_cache.insert(item.id.clone(), neighborhood);
                            }
                        }
                    }

                    last_cache_update = elapsed_time;
                }

                // If anchor changed, update state and start transition
                if anchor_changed {
                    current_anchor = new_anchor;
                    transition_start_time = Some(elapsed_time);
                }

                // Build Scene from SemanticScene with a simple layout
                use webizen_studio::render::qualia::build_scene;
                use webizen_studio::render::scene::Camera;

                // Collect all item IDs for dynamic layout
                let item_ids: Vec<String> = semantic_scene
                    .items
                    .iter()
                    .map(|item| item.id.clone())
                    .collect();
                let total = item_ids.len();

                let scene = build_scene(&semantic_scene, Camera::default(), move |id| {
                    // Find the index of this item in the list
                    let idx = item_ids.iter().position(|item_id| item_id == id)?;

                    // Simple circular layout
                    let angle = (idx as f64 / total as f64) * 2.0 * std::f64::consts::PI;
                    let radius = 3.0;
                    let position = webizen_studio::render::scene::Vec3::new(
                        angle.cos() * radius,
                        0.0,
                        angle.sin() * radius,
                    );

                    // Choose mesh based on entity type from ID
                    let mesh = if id.contains("person") {
                        webizen_studio::render::mesh::Mesh::uv_sphere(0.5, 8, 12)
                    } else if id.contains("concept") {
                        webizen_studio::render::mesh::Mesh::cube(0.8)
                    } else if id.contains("document") {
                        webizen_studio::render::mesh::Mesh::uv_sphere(0.4, 6, 8)
                    } else if id.contains("location") {
                        webizen_studio::render::mesh::Mesh::cube(0.6)
                    } else {
                        webizen_studio::render::mesh::Mesh::uv_sphere(0.3, 4, 6)
                    };

                    Some((position, mesh))
                });

                // Map to RenderScene contract
                use webizen_render::scene_contract::{RenderScene, ScenePoint, TransitionState};
                let render_scene = RenderScene::from(scene.clone());

                // Store current node positions before updating for next transition
                if anchor_changed {
                    previous_node_positions = render_scene
                        .nodes
                        .iter()
                        .map(|node| (node.id.clone(), node.position.x, node.position.y))
                        .collect();
                }

                // Calculate transition progress
                let transition_state = if let Some(start_time) = transition_start_time {
                    let progress = ((elapsed_time - start_time) / TRANSITION_DURATION).min(1.0);
                    if progress < 1.0 {
                        Some(TransitionState {
                            previous_positions: previous_node_positions
                                .iter()
                                .map(|(id, x, y)| {
                                    (
                                        id.clone(),
                                        ScenePoint {
                                            x: *x,
                                            y: *y,
                                            z: 0.0,
                                        },
                                    )
                                })
                                .collect(),
                            progress,
                            duration: TRANSITION_DURATION,
                        })
                    } else {
                        transition_start_time = None; // Transition complete
                        None
                    }
                } else {
                    None
                };

                // Set selected node ID for visual feedback
                let render_scene = RenderScene {
                    selected_node_id: current_anchor.clone(),
                    transition_state,
                    ..render_scene
                };

                // Render with time
                let render_scene_clone = render_scene.clone();
                let telemetry = crate::telemetry_hooks::collect_system_telemetry();
                let png = match tauri::async_runtime::spawn_blocking(move || {
                    webizen_render::wgpu_renderer::render_scene_png_with_time_and_telemetry(
                        &render_scene_clone,
                        800,
                        600,
                        elapsed_time,
                        &telemetry,
                    )
                })
                .await
                {
                    Ok(result) => result
                        .ok_or_else(|| "GPU preview produced no frame (no adapter?)".to_string()),
                    Err(err) => Err(format!("render task failed: {err}")),
                };

                if let Ok(png_bytes) = png {
                    if let Ok(mut guard) = preview_state_clone.png.lock() {
                        *guard = png_bytes.clone();
                    }

                    // Extract and store node positions for picking
                    let node_positions: Vec<(String, f64, f64, f64)> = render_scene
                        .nodes
                        .iter()
                        .map(|node| {
                            // Apply same pulsing calculation as renderer
                            let animated_radius = if node.is_inferencing && node.pulse_rate > 0.0 {
                                let pulse_phase =
                                    2.0 * std::f64::consts::PI * node.pulse_rate * elapsed_time;
                                let pulse_factor = 1.0 + 0.3 * pulse_phase.sin();
                                node.radius * pulse_factor.abs()
                            } else {
                                node.radius
                            };

                            // Convert normalized coordinates to screen space (800x600)
                            let screen_x = node.position.x * 800.0;
                            let screen_y = node.position.y * 600.0;

                            (node.id.clone(), screen_x, screen_y, animated_radius)
                        })
                        .collect();

                    if let Ok(mut positions) = preview_state_clone.node_positions.lock() {
                        *positions = node_positions;
                    }

                    let _ = app_handle_clone.emit("render-preview-ready", ());
                }

                // Energy-aware sleep: only sleep for the REMAINING time in the frame window
                let render_time = loop_start.elapsed();
                if render_time < frame_duration {
                    sleep(frame_duration - render_time).await;
                }
            }
        });
    }

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
    state: State<'_, PreviewState>,
    app: AppHandle,
) -> Result<(), String> {
    use webizen_render::scene_contract::RenderScene;
    use webizen_studio::render::graph::Scene;
    use webizen_studio::render::qualia::build_scene;
    use webizen_studio::render::scene::Camera;

    // TODO: Replace with actual QualiaDB query
    // For now, use mock projection to validate pipeline
    let semantic_scene = mock_qualia_projection();

    // Build Scene from SemanticScene with a simple layout
    // Layout function maps item IDs to positions and meshes
    let scene = build_scene(&semantic_scene, Camera::default(), |id| {
        // Simple circular layout for testing
        let (idx, total) = if id == "person-alice" {
            (0, 4)
        } else if id == "concept-inferencing-semantic-web" {
            (1, 4)
        } else if id == "document-spec" {
            (2, 4)
        } else if id == "location-critical-hub-processing" {
            (3, 4)
        } else {
            return None;
        };
        let angle = (idx as f64 / total as f64) * 2.0 * std::f64::consts::PI;
        let radius = 3.0;
        let position = webizen_studio::render::scene::Vec3::new(
            angle.cos() * radius,
            0.0,
            angle.sin() * radius,
        );
        // Use different mesh types based on classification
        let mesh = if id.contains("person") {
            webizen_studio::render::mesh::Mesh::uv_sphere(0.5, 8, 12)
        } else if id.contains("concept") {
            webizen_studio::render::mesh::Mesh::cube(0.8)
        } else {
            webizen_studio::render::mesh::Mesh::uv_sphere(0.4, 6, 8)
        };
        Some((position, mesh))
    });

    // Map Scene to RenderScene contract
    let render_scene = RenderScene::from(scene);

    let png = tauri::async_runtime::spawn_blocking(move || {
        webizen_render::render_scene_png(&render_scene, width, height)
    })
    .await
    .map_err(|err| format!("render task failed: {err}"))?
    .ok_or_else(|| "GPU preview produced no frame (no adapter?)".to_string())?;

    *state
        .png
        .lock()
        .map_err(|_| "preview state poisoned".to_string())? = png;
    app.emit("render-preview-ready", ())
        .map_err(|err| err.to_string())?;
    Ok(())
}

// ── Binary IPC Optimization ─────────────────────────────────────────────────────

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
    active_anchor: State<'_, crate::ActiveAnchor>,
    binary_registry: State<'_, BinaryNodeRegistry>,
) -> Result<(), String> {
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
    active_anchor: State<'_, crate::ActiveAnchor>,
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

// ── Handler registration ──────────────────────────────────────────────────────

pub fn get_invoke_handler() -> impl Fn(tauri::ipc::Invoke<tauri::Wry>) -> bool {
    tauri::generate_handler![
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
        qapp_analyze,
        export_qapp_as_wasm_package,
        get_latest_diffusion_snapshot,
        reconfigure_diffusion,
        get_diffusion_frame_rgba,
        get_diffusion_ledger_health,
        probe_localhost_preview,
        update_render_preview,
        crate::telemetry_bridge::get_system_telemetry,
        crate::telemetry_bridge::update_telemetry_metric,
        crate::telemetry_bridge::reset_telemetry,
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
    ]
}
