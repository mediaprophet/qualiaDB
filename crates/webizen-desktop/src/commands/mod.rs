use qualia_client_core::api;
use qualia_client_core::api::{CoinBalance, HardwareStatus, TokenEntry, TxRecord, WalletStatus};
use qualia_client_core::engine::{ingestion, llm_offload};
use qualia_client_core::state::{Actor, AgentConfig, DelegationRule, FrontDoor, ProgressPayload};
use qualia_core_db::ilp_dispatcher::DispatchResult;
use qualia_core_db::rpc::TaxRecipientSuite;
use tauri::{command, AppHandle, Manager, WindowBuilder, WindowUrl};

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

    if let Some(window) = app.get_window(&label) {
        window.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }

    let parsed = url
        .parse()
        .map_err(|e| format!("Invalid launch URL '{url}': {e}"))?;
    WindowBuilder::new(&app, label, WindowUrl::External(parsed))
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
pub fn save_qpu_settings(input: qualia_client_core::qpu_oracle::QpuOracleSettingsInput) -> Result<qualia_client_core::qpu_oracle::QpuOracleSettings, String> {
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
// ── Handler registration ──────────────────────────────────────────────────────

pub fn get_invoke_handler() -> impl Fn(tauri::Invoke) {
    tauri::generate_handler![
        list_installed_qapps,
        generate_qapp_credential,
        verify_and_install_qapp,
        launch_installed_qapp,
        get_hardware_status,
        profile_energy_circumstance,
        start_daemon,
        daemon_status,
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
    ]
}
