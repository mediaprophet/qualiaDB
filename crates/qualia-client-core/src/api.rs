use crate::state::*;

use crate::qapp_paths::{qapps_dir, resolve_package_manifest_path};
use crate::qapp_registry;
use crate::engine::ingestion;
use crate::engine::llm_offload;
use crate::engine::q42_compiler;
use futures_util::StreamExt;
use qualia_core_db::ilp_dispatcher::{DispatchResult, HttpIlpTransport, IlpDispatcher};
use qualia_core_db::rpc::{route_tax_payment, TaxRecipientSuite};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};
use std::sync::Arc;
use std::time::Duration;
use sysinfo::{Disks, System};
use tokio::time::sleep;
// ── Tauri commands ────────────────────────────────────────────────────────────

pub fn list_installed_qapps() -> Vec<String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let data_dir = state.config.lock().unwrap().storage_path.clone();
    let qapps_dir = qapps_dir(&data_dir);
    let mut qapps = Vec::new();
    if let Ok(entries) = std::fs::read_dir(qapps_dir) {
        for entry in entries.filter_map(Result::ok) {
            if entry.path().is_dir() {
                qapps.push(entry.file_name().to_string_lossy().to_string());
            }
        }
    }
    qapps
}

pub fn generate_qapp_credential(qapp_name: String) -> String {
    format!("did:qualia:qapp:{}:signed_vc", qapp_name)
}

pub fn verify_and_install_qapp(target_path: String) -> Result<String, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let path = std::path::PathBuf::from(&target_path);
    let manifest_path = resolve_package_manifest_path(&path)
        .ok_or_else(|| "qapp.json not found in directory".to_string())?;

    let content = std::fs::read_to_string(&manifest_path).map_err(|e| e.to_string())?;
    let manifest: qapp_registry::QappPackageManifest =
        serde_json::from_str(&content).map_err(|e| format!("Invalid qapp package manifest: {e}"))?;

    let qapp_did = format!(
        "did:qualia:qapp:{}",
        manifest.name.to_lowercase().replace(" ", "-")
    );

    let target = if let Ok(port) = target_path.parse::<u16>() {
        qapp_registry::QappTarget::LocalProxyPort(port)
    } else {
        qapp_registry::QappTarget::LocalDevDirectory(path)
    };

    let registered_qapp = qapp_registry::RegisteredQapp {
        did: qapp_did.clone(),
        manifest,
        target,
    };

    let qapp_id_hash = crate::qapp_manifest::install_qapp_capabilities(&registered_qapp.manifest)
        .map_err(|e| format!("Qapp capability compile failed: {e:?}"))?;

    state.installed_qapps.lock().unwrap().push(registered_qapp);
    save_directory_state();

    Ok(format!("{qapp_did} (hash={qapp_id_hash})"))
}

#[derive(Serialize)]
pub struct WalletStatus {
    lightning_sats: u64,
    ilp_microcents: u64,
    nym_connected: bool,
}

pub fn get_wallet_status() -> WalletStatus {
    WalletStatus {
        lightning_sats: 450000,
        ilp_microcents: 1250000,
        nym_connected: true,
    }
}

pub fn get_config() -> AgentConfig {
    let state = crate::state::APP_STATE.get().unwrap();
    state.config.lock().unwrap().clone()
}

pub fn save_config(new_config: AgentConfig) -> Result<(), String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let disks = Disks::new_with_refreshed_list();
    let path = PathBuf::from(&new_config.storage_path);
    let mut available = u64::MAX;
    for disk in disks.list() {
        if path.starts_with(disk.mount_point()) {
            available = disk.available_space();
            break;
        }
    }
    let margin: u64 = 15 * 1024 * 1024 * 1024;
    let requested = new_config.storage_quota_gb * 1024 * 1024 * 1024;
    if available.saturating_sub(requested) < margin {
        return Err(
            "OS_SAFETY_VIOLATION: Would leave the host OS with less than the 15 GB safety margin."
                .to_string(),
        );
    }
    // Persist config to disk
    let meta = app_meta_dir();
    std::fs::create_dir_all(&meta).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(&new_config).map_err(|e| e.to_string())?;
    std::fs::write(config_file_path(), json).map_err(|e| e.to_string())?;
    // Ensure data directories exist under the new path
    init_data_directories(&new_config.storage_path);
    *state.config.lock().unwrap() = new_config;
    Ok(())
}

// ── QPU Oracle (hidden until chat unlock) ─────────────────────────────────────

pub use crate::qpu_oracle::{QpuChatCommandResult, QpuOracleSettings, QpuOracleSettingsInput};

pub fn get_qpu_settings() -> QpuOracleSettings {
    crate::qpu_oracle::get_qpu_settings()
}

pub fn is_qpu_feature_unlocked() -> bool {
    crate::qpu_oracle::is_qpu_feature_unlocked()
}

pub fn save_qpu_settings(input: QpuOracleSettingsInput) -> Result<QpuOracleSettings, String> {
    crate::qpu_oracle::save_qpu_settings(input)
}

pub fn handle_qpu_chat_command(text: String) -> QpuChatCommandResult {
    crate::qpu_oracle::handle_qpu_chat_command(&text)
}

pub fn handle_engine_chat_command(text: String) -> QpuChatCommandResult {
    crate::qpu_pipeline::handle_engine_chat_command(&text)
}

pub fn profile_energy_circumstance() -> String {
    let mut sys = System::new_all();
    sys.refresh_all();
    let total_mem = sys.total_memory() / 1024 / 1024;
    let used_mem = sys.used_memory() / 1024 / 1024;
    format!(
        "Energy: AC_POWER\nTotal RAM: {} MB\nUsed RAM: {} MB\nSwarm Auth: GRANTED",
        total_mem, used_mem
    )
}

pub fn check_ollama_status() -> bool {
    std::process::Command::new("ollama")
        .arg("-v")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[derive(Serialize)]
pub struct HardwareStatus {
    pub ram_total_gb: f64,
    pub ram_used_gb: f64,
    pub vram_estimated_gb: f64,
}

pub fn get_hardware_status() -> HardwareStatus {
    let mut sys = System::new_all();
    sys.refresh_all();
    HardwareStatus {
        ram_total_gb: sys.total_memory() as f64 / 1024.0 / 1024.0 / 1024.0,
        ram_used_gb: sys.used_memory() as f64 / 1024.0 / 1024.0 / 1024.0,
        // Mock 16GB unified memory for M-Series
        vram_estimated_gb: 16.0,
    }
}

pub async fn download_and_vectorize(
    url: String,
    filename: String,
    item_id: String,
) -> Result<String, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage_path = state.config.lock().unwrap().storage_path.clone();
    let handles = state.download_handles.clone();
    let active_dl = state.active_downloads.clone();

    let index_dir = PathBuf::from(&storage_path).join("Index");
    std::fs::create_dir_all(&index_dir).map_err(|e| e.to_string())?;
    let dest_path = index_dir.join(&filename);

    let cancelled = Arc::new(AtomicBool::new(false));
    handles
        .lock()
        .unwrap()
        .insert(item_id.clone(), cancelled.clone());

    let response = reqwest::get(&url).await.map_err(|e| {
        handles.lock().unwrap().remove(&item_id);
        active_dl.lock().unwrap().remove(&item_id);
        e.to_string()
    })?;
    let total_bytes = response.content_length().unwrap_or(0);
    let mut dest = std::fs::File::create(&dest_path).map_err(|e| e.to_string())?;
    let mut stream = response.bytes_stream();
    let mut downloaded: u64 = 0;
    let mut last_report = std::time::Instant::now();
    let mut last_downloaded: u64 = 0;

    while let Some(chunk) = stream.next().await {
        if cancelled.load(Ordering::Relaxed) {
            let _ = std::fs::remove_file(&dest_path);
            let payload = ProgressPayload {
                id: item_id.clone(),
                progress: 0.0,
                downloaded_bytes: downloaded,
                total_bytes,
                speed_kbps: 0.0,
                status: "cancelled".to_string(),
            };
            let _ = state.download_events.send(payload.clone());
            handles.lock().unwrap().remove(&item_id);
            active_dl.lock().unwrap().remove(&item_id);
            return Err("Cancelled".to_string());
        }
        let chunk = chunk.map_err(|e| e.to_string())?;
        dest.write_all(&chunk).map_err(|e| e.to_string())?;
        downloaded += chunk.len() as u64;

        let now = std::time::Instant::now();
        if now.duration_since(last_report).as_millis() >= 200 {
            let elapsed = now.duration_since(last_report).as_secs_f64().max(0.001);
            let speed_kbps = ((downloaded - last_downloaded) as f64 / 1024.0) / elapsed;
            let progress = if total_bytes > 0 {
                (downloaded as f64 / total_bytes as f64) * 100.0
            } else {
                0.0
            };
            let payload = ProgressPayload {
                id: item_id.clone(),
                progress,
                downloaded_bytes: downloaded,
                total_bytes,
                speed_kbps,
                status: "downloading".to_string(),
            };
            let _ = state.download_events.send(payload.clone());
            active_dl.lock().unwrap().insert(item_id.clone(), payload);
            last_report = now;
            last_downloaded = downloaded;
        }
    }

    let processing_payload = ProgressPayload {
        id: item_id.clone(),
        progress: 100.0,
        downloaded_bytes: downloaded,
        total_bytes,
        speed_kbps: 0.0,
        status: "processing".to_string(),
    };
    let _ = state.download_events.send(processing_payload.clone());
    active_dl
        .lock()
        .unwrap()
        .insert(item_id.clone(), processing_payload);

    let _quin_count = crate::resource_import::ingest_local_rdf(
        &dest_path,
        &item_id,
        Path::new(&storage_path),
        None,
    )
    .map_err(|e| e.to_string())?;

    let done_payload = ProgressPayload {
        id: item_id.clone(),
        progress: 100.0,
        downloaded_bytes: downloaded,
        total_bytes,
        speed_kbps: 0.0,
        status: "complete".to_string(),
    };
    let _ = state.download_events.send(done_payload.clone());
    handles.lock().unwrap().remove(&item_id);
    active_dl.lock().unwrap().remove(&item_id);
    Ok("Download and vectorization complete".to_string())
}

pub async fn download_model(
    url: String,
    filename: String,
    model_id: String,
) -> Result<String, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage_path = state.config.lock().unwrap().storage_path.clone();
    let handles = state.download_handles.clone();
    let active_dl = state.active_downloads.clone();

    let models_dir = PathBuf::from(&storage_path).join("Models");
    std::fs::create_dir_all(&models_dir).map_err(|e| e.to_string())?;
    let dest_path = models_dir.join(&filename);

    let cancelled = Arc::new(AtomicBool::new(false));
    handles
        .lock()
        .unwrap()
        .insert(model_id.clone(), cancelled.clone());

    let response = reqwest::get(&url).await.map_err(|e| {
        handles.lock().unwrap().remove(&model_id);
        active_dl.lock().unwrap().remove(&model_id);
        e.to_string()
    })?;
    let total_bytes = response.content_length().unwrap_or(0);
    let mut dest = std::fs::File::create(&dest_path).map_err(|e| e.to_string())?;
    let mut stream = response.bytes_stream();
    let mut downloaded: u64 = 0;
    let mut last_report = std::time::Instant::now();
    let mut last_downloaded: u64 = 0;

    while let Some(chunk) = stream.next().await {
        if cancelled.load(Ordering::Relaxed) {
            let _ = std::fs::remove_file(&dest_path);
            let payload = ProgressPayload {
                id: model_id.clone(),
                progress: 0.0,
                downloaded_bytes: downloaded,
                total_bytes,
                speed_kbps: 0.0,
                status: "cancelled".to_string(),
            };
            let _ = state.download_events.send(payload.clone());
            handles.lock().unwrap().remove(&model_id);
            active_dl.lock().unwrap().remove(&model_id);
            return Err("Cancelled".to_string());
        }
        let chunk = chunk.map_err(|e| e.to_string())?;
        dest.write_all(&chunk).map_err(|e| e.to_string())?;
        downloaded += chunk.len() as u64;

        let now = std::time::Instant::now();
        if now.duration_since(last_report).as_millis() >= 200 {
            let elapsed = now.duration_since(last_report).as_secs_f64().max(0.001);
            let speed_kbps = ((downloaded - last_downloaded) as f64 / 1024.0) / elapsed;
            let progress = if total_bytes > 0 {
                (downloaded as f64 / total_bytes as f64) * 100.0
            } else {
                0.0
            };
            let payload = ProgressPayload {
                id: model_id.clone(),
                progress,
                downloaded_bytes: downloaded,
                total_bytes,
                speed_kbps,
                status: "downloading".to_string(),
            };
            let _ = state.download_events.send(payload.clone());
            active_dl.lock().unwrap().insert(model_id.clone(), payload);
            last_report = now;
            last_downloaded = downloaded;
        }
    }

    let done_payload = ProgressPayload {
        id: model_id.clone(),
        progress: 100.0,
        downloaded_bytes: downloaded,
        total_bytes,
        speed_kbps: 0.0,
        status: "complete".to_string(),
    };
    let _ = state.download_events.send(done_payload.clone());
    handles.lock().unwrap().remove(&model_id);
    active_dl.lock().unwrap().remove(&model_id);
    Ok(dest_path.to_string_lossy().to_string())
}

pub fn cancel_download(id: String) -> Result<(), String> {
    let state = crate::state::APP_STATE.get().unwrap();
    if let Some(flag) = state.download_handles.lock().unwrap().get(&id) {
        flag.store(true, Ordering::Relaxed);
    }
    Ok(())
}

pub fn start_daemon() -> String {
    "Daemon Started".to_string()
}

pub fn daemon_status() -> String {
    let state = crate::state::APP_STATE.get().unwrap();
    if *state.daemon_running.lock().unwrap() {
        "running".to_string()
    } else {
        "stopped".to_string()
    }
}

pub fn get_tax_suite() -> TaxRecipientSuite {
    let state = crate::state::APP_STATE.get().unwrap();
    state.tax_suite.lock().unwrap().clone()
}

pub fn save_tax_suite(suite: TaxRecipientSuite) -> Result<(), String> {
    let state = crate::state::APP_STATE.get().unwrap();
    suite.validate()?;
    let data_dir = state.config.lock().unwrap().storage_path.clone();
    let path = suite_file_path(&data_dir);
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(&suite).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())?;
    *state.tax_suite.lock().unwrap() = suite;
    Ok(())
}

pub fn dispatch_tax_payment(gross_amount_micro_cents: u64) -> Result<DispatchResult, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let suite = state.tax_suite.lock().unwrap().clone();
    let plan = route_tax_payment(gross_amount_micro_cents, &suite)?;
    let disp = IlpDispatcher::new(HttpIlpTransport {
        connector_url: "http://localhost:7770".to_string(),
    });
    Ok(disp.dispatch(&plan))
}

pub fn accept_vault_handshake(did_key: String, _payload: String) -> Result<String, String> {
    println!("[VC-8] Vault handshake from: {}", did_key);
    Ok("HANDSHAKE_SUCCESS".to_string())
}

pub fn receive_vault_job(
    job_id: String,
    task_type: String,
    _data_blob_cbor: Vec<u8>,
) -> Result<String, String> {
    println!("[VC-12] Offload job {} type {}", job_id, task_type);
    if task_type == "LLM_INFERENCE" && check_ollama_status() {
        Ok("INFERENCE_QUEUED".to_string())
    } else {
        Err("UNSUPPORTED_TASK_OR_NO_CAPACITY".to_string())
    }
}

pub async fn ingest_pdf(file_name: String) -> Result<ingestion::IngestionResult, String> {
    let result = ingestion::process_pdf(&file_name)?;
    q42_compiler::compile_to_q42(&file_name, &result.bookmarks)?;
    Ok(result)
}

pub async fn ingest_literature(file_path: String) -> Result<String, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage_path = state.config.lock().unwrap().storage_path.clone();
    let lib_dir = PathBuf::from(&storage_path).join("SemanticLibrary");
    if !lib_dir.exists() {
        std::fs::create_dir_all(&lib_dir).map_err(|e| e.to_string())?;
    }

    let source_path = std::path::Path::new(&file_path);
    let filename = source_path.file_name().unwrap_or_default();
    let dest_path = lib_dir.join(filename);
    std::fs::copy(&source_path, &dest_path).map_err(|e| e.to_string())?;

    let text = pdf_extract::extract_text(&dest_path).map_err(|e| e.to_string())?;
    let preview = if text.len() > 100 {
        &text[0..100]
    } else {
        &text
    };

    Ok(format!(
        "Successfully ingested literature: {}. Generated ontology nodes from preview: '{}...'",
        filename.to_string_lossy(),
        preview.replace("\n", " ")
    ))
}

pub async fn upsert_cmld_definition(term: String, context_did: String) -> Result<String, String> {
    Ok(format!(
        "Successfully mapped '{}' to Context: {}",
        term, context_did
    ))
}

pub async fn ingest_ontology(file_name: String) -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage_path = state.config.lock().unwrap().storage_path.clone();
    let index_dir = PathBuf::from(&storage_path).join("Index");
    let source_path = index_dir.join(&file_name);

    if !source_path.is_file() {
        return Err(format!(
            "Ontology source not found in Index/: {}",
            source_path.display()
        ));
    }

    let ontology_id = source_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(&file_name)
        .to_string();

    let quin_count = crate::resource_import::ingest_local_rdf(
        &source_path,
        &ontology_id,
        Path::new(&storage_path),
        None,
    )
    .map_err(|e| e.to_string())?;

    let q42_path = index_dir.join(format!("{ontology_id}.q42"));

    Ok(serde_json::json!({
        "status": "success",
        "file": file_name,
        "ontology_id": ontology_id,
        "q42_path": q42_path.to_string_lossy(),
        "quin_count": quin_count,
    }))
}

pub async fn import_catalog_ontology(id: String) -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage_path = state.config.lock().unwrap().storage_path.clone();
    let catalog = load_workspace_catalog();

    let result = crate::resource_import::import_catalog_ontology(
        &catalog,
        &id,
        Path::new(&storage_path),
    )
    .await
    .map_err(|e| e.to_string())?;

    qualia_core_db::daemon_graph::init_daemon_graph(&storage_path);

    serde_json::to_value(result).map_err(|e| e.to_string())
}

pub async fn export_to_solid(
    input_q42_path: String,
    output_dir_path: String,
) -> Result<String, String> {
    qualia_core_db::solid_ldp::SolidExporter::export_to_solid_pod(&input_q42_path, &output_dir_path)
        .map(|_| format!("Exported to {}", output_dir_path))
        .map_err(|e| e.to_string())
}

pub async fn ingest_image(file_path: String) -> Result<serde_json::Value, String> {
    ingest_image_typed(file_path, "Generic Asset".to_string()).await
}

pub async fn ingest_image_typed(
    file_path: String,
    typology: String,
) -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let active = load_active_model_record_from_disk();
    let result = crate::vision_ingest::ingest_image_with_active_record(
        Path::new(&storage),
        active,
        Path::new(&file_path),
        &typology,
    )
    .map_err(|e| e.to_string())?;
    serde_json::to_value(result).map_err(|e| e.to_string())
}

pub async fn ingest_image_async(file_path: String, typology: String) -> Result<(), String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let active = load_active_model_record_from_disk();
    tokio::spawn(async move {
        let _ = crate::vision_ingest::ingest_image_with_active_record(
            Path::new(&storage),
            active,
            Path::new(&file_path),
            &typology,
        );
    });
    Ok(())
}

// ── Token registry ───────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone)]
pub struct TokenEntry {
    id: String,
    chain: String,      // "eCash" | "Ethereum" | "Nyx"
    token_type: String, // "ALP" | "SLP" | "ERC-20" | "CW-20"
    contract: String,   // token ID / contract address
    symbol: String,
    name: String,
    balance: String,
    decimals: u8,
    fiat_usd: f64,
}

pub fn tokens_file_path(storage_path: &str) -> PathBuf {
    PathBuf::from(storage_path).join("tokens.json")
}

pub fn default_tokens() -> Vec<TokenEntry> {
    vec![
        TokenEntry {
            id: "alp-lion".into(),
            chain: "eCash".into(),
            token_type: "ALP".into(),
            contract: "alp:0x1A2B3C4D...".into(),
            symbol: "LION".into(),
            name: "Lion Rampant (Heraldry)".into(),
            balance: "1.00".into(),
            decimals: 8,
            fiat_usd: 0.0,
        },
        TokenEntry {
            id: "alp-horus".into(),
            chain: "eCash".into(),
            token_type: "ALP".into(),
            contract: "alp:0x9B4C5D6E...".into(),
            symbol: "HORUS".into(),
            name: "Eye of Horus (Artifact)".into(),
            balance: "50.00".into(),
            decimals: 8,
            fiat_usd: 0.0,
        },
        TokenEntry {
            id: "slp-meme".into(),
            chain: "eCash".into(),
            token_type: "SLP".into(),
            contract: "slp:0x44F1A2B3...".into(),
            symbol: "MEME".into(),
            name: "Early Beta Meme Coin".into(),
            balance: "150000.00".into(),
            decimals: 2,
            fiat_usd: 0.0,
        },
        TokenEntry {
            id: "erc20-usdt".into(),
            chain: "Ethereum".into(),
            token_type: "ERC-20".into(),
            contract: "0xdAC17F958D2ee523a2206206994597C13D831ec7".into(),
            symbol: "USDT".into(),
            name: "Tether USD".into(),
            balance: "250.00".into(),
            decimals: 6,
            fiat_usd: 250.0,
        },
        TokenEntry {
            id: "erc20-usdc".into(),
            chain: "Ethereum".into(),
            token_type: "ERC-20".into(),
            contract: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".into(),
            symbol: "USDC".into(),
            name: "USD Coin".into(),
            balance: "100.00".into(),
            decimals: 6,
            fiat_usd: 100.0,
        },
        TokenEntry {
            id: "erc20-link".into(),
            chain: "Ethereum".into(),
            token_type: "ERC-20".into(),
            contract: "0x514910771AF9Ca656af840dff83E8264EcF986CA".into(),
            symbol: "LINK".into(),
            name: "Chainlink Token".into(),
            balance: "12.50".into(),
            decimals: 18,
            fiat_usd: 162.5,
        },
        TokenEntry {
            id: "cw20-vnym".into(),
            chain: "Nyx".into(),
            token_type: "CW-20".into(),
            contract: "nyx1staking000000000000000000000000000000000000".into(),
            symbol: "vNYM".into(),
            name: "Vested NYM (Staking)".into(),
            balance: "100.00".into(),
            decimals: 6,
            fiat_usd: 2.0,
        },
    ]
}

pub fn load_tokens_from_disk(storage_path: &str) -> Vec<TokenEntry> {
    let path = tokens_file_path(storage_path);
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(default_tokens)
}

pub fn save_tokens_to_disk(storage_path: &str, tokens: &[TokenEntry]) -> Result<(), String> {
    let json = serde_json::to_string_pretty(tokens).map_err(|e| e.to_string())?;
    std::fs::write(tokens_file_path(storage_path), json).map_err(|e| e.to_string())
}

pub fn get_tokens() -> Vec<TokenEntry> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage_path = state.config.lock().unwrap().storage_path.clone();
    load_tokens_from_disk(&storage_path)
}

pub fn add_token(
    chain: String,
    token_type: String,
    contract: String,
    symbol: String,
    name: String,
    decimals: u8,
) -> Result<TokenEntry, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage_path = state.config.lock().unwrap().storage_path.clone();
    let mut tokens = load_tokens_from_disk(&storage_path);

    if tokens
        .iter()
        .any(|t| t.contract.to_lowercase() == contract.to_lowercase() && t.chain == chain)
    {
        return Err("Token already in wallet".to_string());
    }

    let slug: String = contract
        .chars()
        .rev()
        .take(8)
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    let id = format!(
        "{}-{}",
        chain.to_lowercase().replace(' ', "-"),
        slug.to_lowercase()
    );
    let entry = TokenEntry {
        id,
        chain,
        token_type,
        contract,
        symbol,
        name,
        balance: "0.00".into(),
        decimals,
        fiat_usd: 0.0,
    };
    tokens.push(entry.clone());
    save_tokens_to_disk(&storage_path, &tokens)?;
    Ok(entry)
}

pub fn remove_token(id: String) -> Result<(), String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage_path = state.config.lock().unwrap().storage_path.clone();
    let mut tokens = load_tokens_from_disk(&storage_path);
    tokens.retain(|t| t.id != id);
    save_tokens_to_disk(&storage_path, &tokens)
}

// ─────────────────────────────────────────────────────────────────────────────

pub fn read_identity() -> Option<serde_json::Value> {
    std::fs::read_to_string(identity_file_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
}

#[derive(Serialize, Clone)]
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

#[derive(Serialize, Clone)]
pub struct TxRecord {
    txid: String,
    ticker: String,
    direction: String, // "in" | "out"
    amount: String,
    label: String,
    timestamp: String,
    status: String, // "confirmed" | "pending"
    confirmations: u32,
    fee: String,
    counterparty: String,
}

pub fn get_coin_balances() -> Vec<CoinBalance> {
    let id = read_identity();
    let addr = |key: &str, fallback: &str| -> String {
        id.as_ref()
            .and_then(|v| v.get(key))
            .and_then(|v| v.as_str())
            .unwrap_or(fallback)
            .to_string()
    };
    vec![
        CoinBalance {
            coin: "eCash".into(),
            ticker: "XEC".into(),
            address: addr("ecash_xec", "ecash:q… (generate identity first)"),
            balance: 1_250_000.0,
            balance_display: "1,250,000.00".into(),
            fiat_usd: 245.00,
            price_usd: 0.000196,
            change_24h: 3.2,
            network: "eCash".into(),
            status: "online".into(),
        },
        CoinBalance {
            coin: "Bitcoin".into(),
            ticker: "BTC".into(),
            address: "bc1q… (generate identity first)".into(),
            balance: 0.00450000,
            balance_display: "0.00450000".into(),
            fiat_usd: 441.00,
            price_usd: 98_000.0,
            change_24h: -1.4,
            network: "Bitcoin".into(),
            status: "online".into(),
        },
        CoinBalance {
            coin: "Monero".into(),
            ticker: "XMR".into(),
            address: "4… (generate identity first)".into(),
            balance: 4.5,
            balance_display: "4.50000000".into(),
            fiat_usd: 720.00,
            price_usd: 160.0,
            change_24h: 0.8,
            network: "Monero".into(),
            status: "online".into(),
        },
        CoinBalance {
            coin: "Nym".into(),
            ticker: "NYM".into(),
            address: addr("nym_mixnet", "n1… (generate identity first)"),
            balance: 2_400.0,
            balance_display: "2,400.00".into(),
            fiat_usd: 48.00,
            price_usd: 0.02,
            change_24h: -2.1,
            network: "Nyx Chain".into(),
            status: "online".into(),
        },
        CoinBalance {
            coin: "Ethereum".into(),
            ticker: "ETH".into(),
            address: addr("ethereum", "0x… (generate identity first)"),
            balance: 1.42,
            balance_display: "1.42000000".into(),
            fiat_usd: 4_260.00,
            price_usd: 3_000.0,
            change_24h: 1.9,
            network: "Ethereum".into(),
            status: "online".into(),
        },
    ]
}

pub fn get_transaction_history(ticker: String) -> Vec<TxRecord> {
    let all = vec![
        TxRecord {
            txid: "7a9b4f2e1c3d…4c1f".into(),
            ticker: "XEC".into(),
            direction: "out".into(),
            amount: "0.0001".into(),
            label: "Mint ALP Token".into(),
            timestamp: "2026-06-05 14:32".into(),
            status: "confirmed".into(),
            confirmations: 142,
            fee: "0.00001 XEC".into(),
            counterparty: "eCash Burn Address".into(),
        },
        TxRecord {
            txid: "99a1bcd4ef56…bb2c".into(),
            ticker: "NYM".into(),
            direction: "out".into(),
            amount: "100.00".into(),
            label: "Mixnet Staking".into(),
            timestamp: "2026-06-04 09:12".into(),
            status: "confirmed".into(),
            confirmations: 320,
            fee: "0.01 NYM".into(),
            counterparty: "mixGateway1".into(),
        },
        TxRecord {
            txid: "4cc288ab12dc…11a9".into(),
            ticker: "XEC".into(),
            direction: "in".into(),
            amount: "50,000.00".into(),
            label: "Received XEC".into(),
            timestamp: "2026-06-03 17:45".into(),
            status: "confirmed".into(),
            confirmations: 580,
            fee: "".into(),
            counterparty: "ecash:qsender7x…".into(),
        },
        TxRecord {
            txid: "b8f1234abc99…de45".into(),
            ticker: "ETH".into(),
            direction: "out".into(),
            amount: "0.05".into(),
            label: "Smart Contract Interaction".into(),
            timestamp: "2026-06-02 11:20".into(),
            status: "confirmed".into(),
            confirmations: 1280,
            fee: "0.002 ETH".into(),
            counterparty: "0xContract4f2…".into(),
        },
        TxRecord {
            txid: "c2d4567ef890…ab12".into(),
            ticker: "BTC".into(),
            direction: "in".into(),
            amount: "0.00100000".into(),
            label: "Received BTC".into(),
            timestamp: "2026-06-01 08:55".into(),
            status: "confirmed".into(),
            confirmations: 2100,
            fee: "".into(),
            counterparty: "bc1qsender9a…".into(),
        },
        TxRecord {
            txid: "e1f23456789a…cd34".into(),
            ticker: "XEC".into(),
            direction: "out".into(),
            amount: "1,000.00".into(),
            label: "ALP Token Transfer".into(),
            timestamp: "2026-05-31 16:30".into(),
            status: "confirmed".into(),
            confirmations: 3400,
            fee: "0.00001 XEC".into(),
            counterparty: "ecash:qrecipient3b…".into(),
        },
        TxRecord {
            txid: "a9b0c1d2e3f4…5678".into(),
            ticker: "XMR".into(),
            direction: "in".into(),
            amount: "2.00000000".into(),
            label: "Received XMR".into(),
            timestamp: "2026-05-30 14:10".into(),
            status: "confirmed".into(),
            confirmations: 4800,
            fee: "".into(),
            counterparty: "4xmrSender8b…".into(),
        },
        TxRecord {
            txid: "f8e7d6c5b4a3…2109".into(),
            ticker: "NYM".into(),
            direction: "in".into(),
            amount: "500.00".into(),
            label: "Staking Reward".into(),
            timestamp: "2026-05-29 10:00".into(),
            status: "confirmed".into(),
            confirmations: 5200,
            fee: "".into(),
            counterparty: "Nym Gateway Reward".into(),
        },
        TxRecord {
            txid: "1a2b3c4d5e6f…7890".into(),
            ticker: "XEC".into(),
            direction: "in".into(),
            amount: "250,000.00".into(),
            label: "Initial Funding".into(),
            timestamp: "2026-05-25 08:00".into(),
            status: "confirmed".into(),
            confirmations: 9100,
            fee: "".into(),
            counterparty: "ecash:qfunding2a…".into(),
        },
        TxRecord {
            txid: "0f1e2d3c4b5a…6789".into(),
            ticker: "ETH".into(),
            direction: "in".into(),
            amount: "1.42000000".into(),
            label: "ETH Transfer In".into(),
            timestamp: "2026-05-20 12:00".into(),
            status: "confirmed".into(),
            confirmations: 12400,
            fee: "".into(),
            counterparty: "0xSender7c4…".into(),
        },
    ];
    if ticker.is_empty() || ticker == "ALL" {
        all
    } else {
        all.into_iter().filter(|tx| tx.ticker == ticker).collect()
    }
}

pub fn is_first_run() -> bool {
    !config_file_path().exists()
}

pub fn save_identity(wallets: serde_json::Value) -> Result<(), String> {
    let meta = app_meta_dir();
    std::fs::create_dir_all(&meta).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(&wallets).map_err(|e| e.to_string())?;
    std::fs::write(identity_file_path(), json).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn load_identity() -> Result<Option<serde_json::Value>, String> {
    let path = identity_file_path();
    if !path.exists() {
        return Ok(None);
    }
    let json = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let val: serde_json::Value = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    Ok(Some(val))
}

use bip39::{Language, Mnemonic};

pub fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

pub async fn generate_bip39_seed() -> Result<String, String> {
    // Generate a secure, randomized 12-word seed phrase natively
    let mnemonic = Mnemonic::generate_in(Language::English, 12)
        .map_err(|_| "Failed to generate".to_string())?;
    let words: Vec<&str> = mnemonic.words().collect();
    Ok(words.join(" "))
}

pub async fn derive_wallets_from_seed(seed: String) -> Result<serde_json::Value, String> {
    let mnemonic = match Mnemonic::parse_in(Language::English, &seed) {
        Ok(m) => m,
        Err(_) => return Err("Invalid 12-word seed phrase.".to_string()),
    };

    // Deterministically generate keys based on the secure seed
    let seed_bytes = mnemonic.to_seed("");

    // Mock derivation by hex-encoding slices of the master seed
    let hex_seed = to_hex(&seed_bytes[0..16]);
    let xec_hex = to_hex(&seed_bytes[16..24]);
    let eth_hex = to_hex(&seed_bytes[24..32]);
    let nym_hex = to_hex(&seed_bytes[32..40]);

    Ok(serde_json::json!({
        "qualia_root": format!("did:qualia:0x{}", hex_seed),
        "nym_mixnet": format!("n1{}...", nym_hex),
        "ecash_xec": format!("ecash:q{}...", xec_hex),
        "ethereum": format!("0x{}...", eth_hex)
    }))
}

pub async fn generate_front_door_invite() -> Result<String, String> {
    let invite = crate::social_connect::generate_connect_invite(None)?;
    Ok(invite.invite_json)
}

pub async fn mint_semantic_token(_asset_id: String) -> Result<String, String> {
    // Phase 12 Mock: Mint ALP eToken with eMPP / RDF metadata payload
    Ok(format!("alp:0x{:04X}...", 45672_u32))
}

pub async fn fetch_wallet_portfolio() -> Result<serde_json::Value, String> {
    // Phase 13 Mock: Return diverse portfolio of tokens
    Ok(serde_json::json!([
        {
            "name": "Lion Rampant (Heraldry)",
            "tokenId": "alp:0x1A2B...",
            "ticker": "LION",
            "balance": "1.00",
            "rdf": "urn:concept:heraldry",
            "network": "eCash",
            "type": "ALP"
        },
        {
            "name": "Eye of Horus (Artifact)",
            "tokenId": "alp:0x9B4C...",
            "ticker": "HORUS",
            "balance": "50.00",
            "rdf": "urn:concept:egyptian",
            "network": "eCash",
            "type": "ALP"
        },
        {
            "name": "Early Beta Meme Coin",
            "tokenId": "slp:0x44F1...",
            "ticker": "MEME",
            "balance": "150000.00",
            "rdf": "Legacy Metadata",
            "network": "eCash",
            "type": "SLP"
        }
    ]))
}

pub async fn import_external_seed(
    network: String,
    seed: String,
    _label: String,
) -> Result<String, String> {
    // Phase 14 Mock: Multi-Seed Account Import
    // Validate seed format
    if seed.split_whitespace().count() < 12 {
        return Err("Invalid seed phrase".to_string());
    }

    // Hash seed to deterministically generate a mock address based on network
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    format!("{}-{}", seed, network).hash(&mut hasher);
    let mock_hash = format!("{:x}", hasher.finish());

    let address = match network.as_str() {
        "eCash (XEC)" => format!("ecash:q{}...", &mock_hash[0..8]),
        "Bitcoin (BTC)" => format!("bc1q{}...", &mock_hash[0..8]),
        "Nym (NYM) - Nyx Chain" => format!("n1{}...", &mock_hash[0..8]),
        "Monero (XMR)" => format!("4{}...", &mock_hash[0..12]),
        "Ethereum (EVM)" => format!("0x{}...", &mock_hash[0..10]),
        _ => format!("0x{}...", &mock_hash[0..10]),
    };

    Ok(address)
}

pub async fn toggle_nym_relay() -> Result<bool, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let active = &state.nym_relay_active;
    let currently_active = active.load(Ordering::Relaxed);
    let new_state = !currently_active;
    active.store(new_state, Ordering::Relaxed);

    if new_state {
        let active_clone = active.clone();

        // Spawn asynchronous background daemon for packet routing
        tokio::spawn(async move {
            let mut packets_routed = 0;
            let mut packets_dropped = 0;

            while active_clone.load(Ordering::Relaxed) {
                // Simulate network fluctuations and calculate memory backpressure
                // Enforcing a strict 50MB telemetry boundary cap internally
                let packet_load_factor = 1.0 + (packets_routed % 5) as f64 * 0.2;
                let buffer_memory_mb = 12.4 * packet_load_factor;
                let is_congested = buffer_memory_mb > 45.0;

                if is_congested {
                    packets_dropped += 15;
                } else {
                    packets_routed += 42;
                }

                // let _ = window_clone.emit("nym-telemetry", RelayTelemetry {
                //     packets_routed,
                //     packets_dropped,
                //     buffer_memory_mb,
                //     is_congested,
                // });

                sleep(Duration::from_millis(500)).await;
            }
        });
    }
    Ok(new_state)
}

pub async fn toggle_stark_prover() -> Result<bool, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let active = &state.stark_prover_active;
    let currently_active = active.load(Ordering::Relaxed);
    let new_state = !currently_active;
    active.store(new_state, Ordering::Relaxed);

    if new_state {
        let active_clone = active.clone();
        let solar_clone = state.simulated_solar_watts.clone();

        // Spawn asynchronous background daemon for out-of-core proof chunking
        tokio::spawn(async move {
            let mut fragments_paged = 0;

            while active_clone.load(Ordering::Relaxed) {
                let current_solar = solar_clone.load(Ordering::Relaxed);

                // Environmental state evaluation trigger (threshold at 400W)
                if current_solar < 400 {
                    // let _ = window_clone.emit("stark-telemetry", StarkTelemetry {
                    //     status: "Suspended - Awaiting Solar Surplus".to_string(),
                    //     cpu_utilization: 0.0,
                    //     ram_usage_mb: 0.0,
                    //     fragments_paged,
                    // });
                } else {
                    fragments_paged += 8; // Simulate 48-byte Super-Quin paging writes

                    // let _ = window_clone.emit("stark-telemetry", StarkTelemetry {
                    //     status: "Proving Execution Active".to_string(),
                    //     cpu_utilization: 85.4,
                    //     ram_usage_mb: 320.0, // Constrained flat memory footprint
                    //     fragments_paged,
                    // });
                }
                sleep(Duration::from_millis(1000)).await;
            }
        });
    }
    Ok(new_state)
}

pub fn update_solar_input(watts: u32) {
    let state = crate::state::APP_STATE.get().unwrap();
    state.simulated_solar_watts.store(watts, Ordering::Relaxed);
}

pub async fn fetch_torrent_telemetry() -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    Ok(crate::ontology_workbench::torrent_telemetry(Path::new(&storage)))
}

pub fn sync_workbench_torrent_seeds(storage_path: &str) -> Result<serde_json::Value, String> {
    crate::ontology_workbench::sync_workbench_seeds_to_daemon(Path::new(storage_path))
}

pub async fn workbench_import_ontology_uri(
    uri: String,
    ontology_id: Option<String>,
    domain: Option<String>,
    title: Option<String>,
) -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let result = crate::ontology_workbench::import_from_uri(
        Path::new(&storage),
        uri,
        ontology_id,
        domain,
        title,
    )
    .await?;
    serde_json::to_value(result).map_err(|e| e.to_string())
}

pub fn list_workbench_ontologies() -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let entries = crate::ontology_workbench::list_workbench_entries(Path::new(&storage))?;
    serde_json::to_value(entries).map_err(|e| e.to_string())
}

pub fn set_workbench_torrent_policy(
    ontology_id: String,
    policy_json: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let policy: crate::ontology_workbench::OntologyTorrentPolicy =
        serde_json::from_value(policy_json).map_err(|e| e.to_string())?;
    let updated = crate::ontology_workbench::set_torrent_policy(
        Path::new(&storage),
        &ontology_id,
        policy,
    )?;
    serde_json::to_value(updated).map_err(|e| e.to_string())
}

pub fn set_workbench_seed(ontology_id: String, active: bool) -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let updated =
        crate::ontology_workbench::set_seed_active(Path::new(&storage), &ontology_id, active)?;
    serde_json::to_value(updated).map_err(|e| e.to_string())
}

pub fn get_torrent_bandwidth_policy() -> Result<serde_json::Value, String> {
    let policy = crate::ontology_workbench::load_bandwidth_policy();
    serde_json::to_value(policy).map_err(|e| e.to_string())
}

pub fn set_torrent_bandwidth_policy(policy_json: serde_json::Value) -> Result<serde_json::Value, String> {
    let policy: crate::ontology_workbench::TorrentBandwidthGlobal =
        serde_json::from_value(policy_json).map_err(|e| e.to_string())?;
    crate::ontology_workbench::save_bandwidth_policy(&policy)?;
    serde_json::to_value(policy).map_err(|e| e.to_string())
}

pub fn list_ontology_shares_for_contact(contact_did: String) -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let cards = crate::ontology_workbench::list_share_cards_for_contact(
        Path::new(&storage),
        &contact_did,
    )?;
    serde_json::to_value(cards).map_err(|e| e.to_string())
}

pub fn list_ontology_shares_for_session(session_did: String) -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let cards = crate::ontology_workbench::list_share_cards_for_session(
        Path::new(&storage),
        &session_did,
    )?;
    serde_json::to_value(cards).map_err(|e| e.to_string())
}

pub fn list_chat_session_share_targets() -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let targets = crate::chat_session::list_session_share_targets(Path::new(&storage))
        .map_err(|e| e.to_string())?;
    serde_json::to_value(targets).map_err(|e| e.to_string())
}

pub fn get_chat_session_did(session_id: String) -> Result<String, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    crate::chat_session::get_session_did(Path::new(&storage), &session_id)
        .map_err(|e| e.to_string())
}

pub fn update_chat_contact_categories(
    contact_did: String,
    categories: Vec<String>,
) -> Result<serde_json::Value, String> {
    let contact = crate::social_connect::update_contact_categories(&contact_did, categories)?;
    serde_json::to_value(contact).map_err(|e| e.to_string())
}

pub async fn discover_models() -> Result<Vec<llm_offload::ModelInfo>, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage_path = state.config.lock().unwrap().storage_path.clone();
    let models_dir = PathBuf::from(&storage_path).join("Models");
    let mut models = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&models_dir) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            if path.extension().map(|e| e == "gguf").unwrap_or(false)
                && !name.to_ascii_lowercase().contains("mmproj")
            {
                models.push(llm_offload::ModelInfo {
                    name,
                    is_active: false,
                    avatar_type: "local".to_string(),
                });
            }
        }
    }
    Ok(models)
}

pub async fn run_agent_inference(
    prompt: String,
    model_name: String,
    intent_layout: Vec<f64>,
) -> Result<(), String> {
    tokio::spawn(async move {
        let _ = llm_offload::execute_agent_inference(prompt, model_name, intent_layout).await;
    });
    Ok(())
}

// ── Chat sessions ─────────────────────────────────────────────────────────────

pub fn create_chat_session(title: Option<String>) -> Result<String, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    crate::chat_session::create_session(Path::new(&storage), title, None)
        .map_err(|e| e.to_string())
}

pub fn list_chat_sessions() -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let sessions = crate::chat_session::list_sessions(Path::new(&storage))
        .map_err(|e| e.to_string())?;
    serde_json::to_value(sessions).map_err(|e| e.to_string())
}

pub fn load_chat_session(id: String) -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let session = crate::chat_session::load_session(Path::new(&storage), &id)
        .map_err(|e| e.to_string())?;
    serde_json::to_value(session).map_err(|e| e.to_string())
}

pub fn append_chat_message(session_id: String, role: String, content: String) -> Result<u64, String> {
    append_chat_message_reply(session_id, role, content, None, None)
}

pub fn append_chat_message_reply(
    session_id: String,
    role: String,
    content: String,
    reply_to_fragment: Option<String>,
    branch_type_id: Option<String>,
) -> Result<u64, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let role = crate::chat_session::Role::from_str(&role).map_err(|e| e.to_string())?;
    crate::chat_session::append_message_with_author(
        Path::new(&storage),
        &session_id,
        role,
        &content,
        reply_to_fragment,
        None,
        None,
        None,
        branch_type_id,
    )
    .map_err(|e| e.to_string())
}

pub fn compact_chat_session(session_id: String) -> Result<String, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let path = crate::chat_session::compact_session_to_q42(Path::new(&storage), &session_id)
        .map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().into_owned())
}

pub fn delete_chat_session(session_id: String) -> Result<(), String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    crate::chat_session::delete_session(Path::new(&storage), &session_id)
        .map_err(|e| e.to_string())
}

pub fn rename_chat_session(session_id: String, title: String) -> Result<(), String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    crate::chat_session::rename_session(Path::new(&storage), &session_id, &title)
        .map_err(|e| e.to_string())
}

pub fn get_last_chat_session_id() -> Option<String> {
    crate::chat_session::get_last_session_id()
}

pub fn set_last_chat_session_id(session_id: String) -> Result<(), String> {
    crate::chat_session::set_last_session_id(&session_id).map_err(|e| e.to_string())
}

pub fn compile_session_environment(session_id: String) -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let catalog = load_workspace_catalog();
    let env = crate::context_binding::refresh_session_environment(
        Path::new(&storage),
        &catalog,
        &session_id,
    )
    .map_err(|e| e.to_string())?;
    serde_json::to_value(env).map_err(|e| e.to_string())
}

pub fn update_session_environment(
    session_id: String,
    ontology_ids: Vec<String>,
    prior_session_ids: Vec<String>,
) -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let catalog = load_workspace_catalog();
    let session = crate::chat_session::load_session(Path::new(&storage), &session_id)
        .map_err(|e| e.to_string())?;
    let config = crate::context_binding::ChatEnvironmentConfig {
        session_id: session_id.clone(),
        ontology_ids,
        prior_session_ids,
        session_kind: session.meta.session_kind,
        participants: session.meta.participants.clone(),
    };
    let env = crate::context_binding::compile_chat_environment(
        Path::new(&storage),
        &catalog,
        &config,
    )
    .map_err(|e| e.to_string())?;
    env.save_to_session_dir(Path::new(&storage))
        .map_err(|e| e.to_string())?;
    serde_json::to_value(env).map_err(|e| e.to_string())
}

pub fn get_session_environment(session_id: String) -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let session = crate::chat_session::load_session(Path::new(&storage), &session_id)
        .map_err(|e| e.to_string())?;
    serde_json::to_value(session.environment).map_err(|e| e.to_string())
}

pub fn list_installed_ontology_ids_for_chat() -> Vec<String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    crate::context_binding::list_installed_ontology_ids(Path::new(&storage))
}

pub fn run_chat_inference(session_id: String, prompt: String) -> Result<String, String> {
    let result = crate::chat_inference::run_chat_inference_with_options(&session_id, &prompt, None);
    if result.committed {
        Ok(result.text)
    } else {
        Err(result
            .block_reason
            .unwrap_or_else(|| "Inference blocked".to_string()))
    }
}

pub fn run_chat_inference_detailed(
    session_id: String,
    prompt: String,
) -> Result<serde_json::Value, String> {
    let result = crate::chat_inference::run_chat_inference_with_options(&session_id, &prompt, None);
    serde_json::to_value(result).map_err(|e| e.to_string())
}

pub fn cancel_chat_inference() {
    crate::chat_inference::request_cancel_inference();
}

pub fn ensure_chat_session() -> Result<String, String> {
    if let Some(id) = get_last_chat_session_id() {
        let state = crate::state::APP_STATE.get().unwrap();
        let storage = state.config.lock().unwrap().storage_path.clone();
        if crate::chat_session::load_session(Path::new(&storage), &id).is_ok() {
            return Ok(id);
        }
    }
    create_chat_session(None)
}

pub fn create_group_chat_session(
    title: Option<String>,
    participant_dids: Vec<String>,
) -> Result<String, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    crate::chat_session::create_group_session(Path::new(&storage), title, &participant_dids)
        .map_err(|e| e.to_string())
}

pub fn add_chat_participant(session_id: String, participant_did: String) -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let participants = crate::chat_session::add_participant(
        Path::new(&storage),
        &session_id,
        &participant_did,
    )
    .map_err(|e| e.to_string())?;
    serde_json::to_value(participants).map_err(|e| e.to_string())
}

pub fn remove_chat_participant(session_id: String, participant_did: String) -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let participants = crate::chat_session::remove_participant(
        Path::new(&storage),
        &session_id,
        &participant_did,
    )
    .map_err(|e| e.to_string())?;
    serde_json::to_value(participants).map_err(|e| e.to_string())
}

pub fn get_chat_participants(session_id: String) -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let participants =
        crate::chat_session::get_participants(Path::new(&storage), &session_id)
            .map_err(|e| e.to_string())?;
    serde_json::to_value(participants).map_err(|e| e.to_string())
}

pub fn get_local_agent_config(session_id: String) -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let cfg = crate::chat_agents::load_local_agent_config(Path::new(&storage), &session_id)?;
    serde_json::to_value(cfg).map_err(|e| e.to_string())
}

pub fn update_agent_outcome_sharing(
    session_id: String,
    policy_json: String,
) -> Result<serde_json::Value, String> {
    let policy: crate::chat_agents::OutcomeSharingPolicy =
        serde_json::from_str(&policy_json).map_err(|e| e.to_string())?;
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let cfg = crate::chat_agents::update_outcome_sharing(
        Path::new(&storage),
        &session_id,
        policy,
    )?;
    serde_json::to_value(cfg).map_err(|e| e.to_string())
}

pub fn get_default_outcome_sharing(session_id: String) -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let session = crate::chat_session::load_session(Path::new(&storage), &session_id)
        .map_err(|e| e.to_string())?;
    let policy = crate::chat_agents::default_outcome_sharing(session.meta.session_kind);
    serde_json::to_value(policy).map_err(|e| e.to_string())
}

// ── Profile + social connect ───────────────────────────────────────────────────

pub fn get_user_profile() -> Result<serde_json::Value, String> {
    let profile = crate::user_profile::load_profile();
    serde_json::to_value(profile).map_err(|e| e.to_string())
}

pub fn save_user_profile(profile_json: String) -> Result<serde_json::Value, String> {
    let mut profile: crate::user_profile::UserProfile =
        serde_json::from_str(&profile_json).map_err(|e| e.to_string())?;
    profile.public_did = crate::user_profile::resolve_public_did(&profile);
    crate::user_profile::save_profile(&profile)?;
    serde_json::to_value(profile).map_err(|e| e.to_string())
}

pub fn generate_connect_invite(front_door_id: Option<String>) -> Result<serde_json::Value, String> {
    let invite = crate::social_connect::generate_connect_invite(front_door_id)?;
    serde_json::to_value(invite).map_err(|e| e.to_string())
}

pub fn accept_connect_invite(input: String) -> Result<serde_json::Value, String> {
    let contact = crate::social_connect::accept_connect_invite(&input)?;
    serde_json::to_value(contact).map_err(|e| e.to_string())
}

pub fn list_chat_contacts() -> Result<serde_json::Value, String> {
    let contacts = crate::social_connect::list_chat_contacts();
    serde_json::to_value(contacts).map_err(|e| e.to_string())
}

pub fn get_chat_graph(session_id: String) -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let graph = crate::chat_graph::load_graph(Path::new(&storage), &session_id)
        .map_err(|e| e.to_string())?;
    let session = crate::chat_session::load_session(Path::new(&storage), &session_id)
        .map_err(|e| e.to_string())?;
    let reactions =
        crate::chat_ontology::list_reactions(Path::new(&storage), &session_id).unwrap_or_default();
    let branch_types = crate::chat_ontology::list_branch_types(Path::new(&storage));
    serde_json::to_value(serde_json::json!({
        "fragments": graph.fragments,
        "edges": graph.edges,
        "messages": session.messages,
        "reactions": reactions,
        "branch_types": branch_types,
        "wordnet": crate::chat_ontology::resolve_wordnet_q42(Path::new(&storage))
            .map(|p| p.to_string_lossy().to_string()),
    }))
    .map_err(|e| e.to_string())
}

pub fn create_chat_fragment(
    session_id: String,
    message_lamport: u64,
    anchor_start: u32,
    anchor_end: u32,
) -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let session = crate::chat_session::load_session(Path::new(&storage), &session_id)
        .map_err(|e| e.to_string())?;
    let msg = session
        .messages
        .iter()
        .find(|m| m.lamport == message_lamport)
        .ok_or_else(|| format!("message {message_lamport} not found"))?;
    let fragment = crate::chat_graph::create_fragment_from_selection(
        Path::new(&storage),
        &session_id,
        message_lamport,
        &msg.content,
        anchor_start,
        anchor_end,
    )
    .map_err(|e| e.to_string())?;
    serde_json::to_value(fragment).map_err(|e| e.to_string())
}

pub fn sync_chat_relay(session_id: Option<String>) -> Result<u64, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    if let Some(id) = session_id {
        Ok(crate::chat_relay::sync_session_relay(Path::new(&storage), &id)? as u64)
    } else {
        Ok(crate::chat_relay::sync_all_group_sessions()? as u64)
    }
}

pub fn start_chat_relay_poller() {
    crate::chat_relay::start_relay_poller();
}

pub fn list_chat_branch_types() -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let types = crate::chat_ontology::list_branch_types(Path::new(&storage));
    serde_json::to_value(types).map_err(|e| e.to_string())
}

pub fn classify_chat_branch(
    anchor_text: String,
    reply_text: String,
) -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let c = crate::chat_ontology::classify_branch(Path::new(&storage), &anchor_text, &reply_text);
    serde_json::to_value(c).map_err(|e| e.to_string())
}

pub fn toggle_chat_reaction(
    session_id: String,
    message_lamport: u64,
    emoji: String,
) -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let reactions = crate::chat_ontology::toggle_reaction(
        Path::new(&storage),
        &session_id,
        message_lamport,
        &emoji,
    )?;
    serde_json::to_value(reactions).map_err(|e| e.to_string())
}

pub fn list_chat_reactions(session_id: String) -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let reactions = crate::chat_ontology::list_reactions(Path::new(&storage), &session_id)?;
    serde_json::to_value(reactions).map_err(|e| e.to_string())
}

pub fn wordnet_chat_ontology_status() -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let path = crate::chat_ontology::resolve_wordnet_q42(Path::new(&storage));
    Ok(serde_json::json!({
        "available": path.is_some(),
        "q42_path": path.as_ref().map(|p| p.to_string_lossy().to_string()),
        "lex_path": path.as_ref().and_then(|p| crate::chat_ontology::resolve_wordnet_lex(p).map(|l| l.to_string_lossy().to_string())),
    }))
}

pub fn default_chat_file_sharing(session_id: String) -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let session = crate::chat_session::load_session(Path::new(&storage), &session_id)
        .map_err(|e| e.to_string())?;
    let sharing = crate::chat_files::default_sharing_for_session(session.meta.session_kind);
    serde_json::to_value(sharing).map_err(|e| e.to_string())
}

pub fn attach_chat_file(
    session_id: String,
    source_path: String,
    sharing_json: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let sharing: crate::chat_files::ChatFileSharing =
        serde_json::from_value(sharing_json).map_err(|e| e.to_string())?;
    let result = crate::chat_files::attach_chat_file(
        Path::new(&storage),
        &session_id,
        Path::new(&source_path),
        sharing,
    )?;
    serde_json::to_value(result).map_err(|e| e.to_string())
}

pub fn list_chat_files(session_id: String) -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let files = crate::chat_files::list_chat_files(Path::new(&storage), &session_id, None)?;
    serde_json::to_value(files).map_err(|e| e.to_string())
}

pub fn set_chat_file_sharing(
    session_id: String,
    file_id: String,
    sharing_json: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let sharing: crate::chat_files::ChatFileSharing =
        serde_json::from_value(sharing_json).map_err(|e| e.to_string())?;
    let updated = crate::chat_files::set_chat_file_sharing(
        Path::new(&storage),
        &session_id,
        &file_id,
        sharing,
    )?;
    serde_json::to_value(updated).map_err(|e| e.to_string())
}

pub fn get_chat_file_local_path(
    session_id: String,
    file_id: String,
    variant: String,
) -> Result<String, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let path = crate::chat_files::resolve_chat_file_path(
        Path::new(&storage),
        &session_id,
        &file_id,
        &variant,
        None,
    )?;
    Ok(path.to_string_lossy().to_string())
}

pub fn parse_chat_file_preview(source_path: String) -> Result<serde_json::Value, String> {
    let path = Path::new(&source_path);
    if !path.is_file() {
        return Err(format!("File not found: {}", path.display()));
    }
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .and_then(|mut f| std::io::Read::read_to_end(&mut f, &mut bytes))
        .map_err(|e| e.to_string())?;
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("file");
    let parsed = crate::chat_files::parse_document_bytes(name, &bytes);
    serde_json::to_value(parsed).map_err(|e| e.to_string())
}

// ── Active model + lifecycle ───────────────────────────────────────────────────

pub fn active_model_path() -> PathBuf {
    app_meta_dir().join("active_model.json")
}

fn legacy_active_model_path() -> PathBuf {
    app_meta_dir().join("active_model.txt")
}

pub fn load_active_model_record_from_disk() -> Option<crate::model_lifecycle::ActiveModelRecord> {
    let json_path = active_model_path();
    if let Ok(text) = std::fs::read_to_string(&json_path) {
        if let Ok(record) = serde_json::from_str(&text) {
            return Some(record);
        }
    }

    // Migrate legacy bare filename.
    let legacy = std::fs::read_to_string(legacy_active_model_path())
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())?;

    let state = crate::state::APP_STATE.get()?;
    let storage = state.config.lock().unwrap().storage_path.clone();
    let model_id = legacy
        .trim_end_matches(".gguf")
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(&legacy)
        .to_string();

    if let Some(manifest) =
        crate::model_lifecycle::load_install_manifest(Path::new(&storage), &model_id)
    {
        let record = crate::model_lifecycle::ActiveModelRecord {
            model_id: manifest.model_id,
            gguf_path: manifest.gguf_path,
            profile_id: manifest.profile_id,
            quantization: manifest.quantization,
            lifecycle_state: crate::model_lifecycle::lifecycle_label(
                crate::model_lifecycle::get_model_lifecycle_state(),
            )
            .to_string(),
            modality: manifest.modality,
            architecture: manifest.architecture,
            mmproj_path: manifest.mmproj_path,
            context_window: manifest.context_window,
        };
        let _ = persist_active_model_record(&record);
        let _ = std::fs::remove_file(legacy_active_model_path());
        return Some(record);
    }

    None
}

pub fn load_active_model_from_disk() -> Option<String> {
    load_active_model_record_from_disk().map(|r| r.gguf_path)
}

fn persist_active_model_record(
    record: &crate::model_lifecycle::ActiveModelRecord,
) -> Result<(), String> {
    let meta = app_meta_dir();
    std::fs::create_dir_all(&meta).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(record).map_err(|e| e.to_string())?;
    std::fs::write(active_model_path(), json).map_err(|e| e.to_string())
}

pub fn restore_active_model_on_startup() {
    let Some(record) = load_active_model_record_from_disk() else {
        return;
    };
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    if Path::new(&record.gguf_path).is_file() {
        let _ =
            crate::model_lifecycle::activate_model_for_id(&record.model_id, Path::new(&storage));
        *state.active_model.lock().unwrap() = Some(record.gguf_path.clone());
    }
}

pub fn get_active_model() -> Option<String> {
    let state = crate::state::APP_STATE.get().unwrap();
    state.active_model.lock().unwrap().clone()
}

pub fn get_model_lifecycle_status() -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let path = state.active_model.lock().unwrap().clone();
    let active = load_active_model_record_from_disk().or_else(|| {
        path.as_ref().map(|gguf| crate::model_lifecycle::ActiveModelRecord {
            model_id: gguf
                .rsplit(['/', '\\'])
                .next()
                .unwrap_or(gguf)
                .trim_end_matches(".gguf")
                .to_string(),
            gguf_path: gguf.clone(),
            profile_id: 0,
            quantization: String::new(),
            lifecycle_state: crate::model_lifecycle::lifecycle_label(
                crate::model_lifecycle::get_model_lifecycle_state(),
            )
            .to_string(),
            modality: "text".to_string(),
            architecture: None,
            mmproj_path: None,
            context_window: 4096,
        })
    });
    let status = crate::model_lifecycle::get_model_status(active);
    serde_json::to_value(status).map_err(|e| e.to_string())
}

pub fn set_active_model(model_name: String) -> Result<(), String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let model_id = model_name
        .trim_end_matches(".gguf")
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(model_name.as_str())
        .to_string();

    let record =
        crate::model_lifecycle::activate_model_for_id(&model_id, Path::new(&storage))
            .map_err(|e| e.to_string())?;

    persist_active_model_record(&record)?;
    *state.active_model.lock().unwrap() = Some(record.gguf_path.clone());
    Ok(())
}

pub async fn install_catalog_llm(id: String) -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage_path = state.config.lock().unwrap().storage_path.clone();
    let handles = state.download_handles.clone();
    let active_dl = state.active_downloads.clone();
    let catalog = load_workspace_catalog();

    let model = catalog
        .find_llm(&id)
        .ok_or_else(|| format!("LLM not found in catalog: {id}"))?;
    let url = model
        .download
        .resolved_url()
        .ok_or_else(|| format!("No download URL for: {id}"))?;
    let filename = model
        .download
        .local_filename()
        .unwrap_or_else(|| format!("{id}.gguf"));

    let models_dir = PathBuf::from(&storage_path).join("Models");
    std::fs::create_dir_all(&models_dir).map_err(|e| e.to_string())?;
    let dest_path = models_dir.join(&filename);

    let cancelled = Arc::new(AtomicBool::new(false));
    handles.lock().unwrap().insert(id.clone(), cancelled.clone());

    let client = reqwest::Client::new();
    let response = client.get(&url).send().await.map_err(|e| {
        handles.lock().unwrap().remove(&id);
        active_dl.lock().unwrap().remove(&id);
        e.to_string()
    })?;
    let total_bytes = response.content_length().unwrap_or(0);
    let mut dest = std::fs::File::create(&dest_path).map_err(|e| e.to_string())?;
    let mut stream = response.bytes_stream();
    let mut downloaded: u64 = 0;
    let mut last_report = std::time::Instant::now();
    let mut last_downloaded: u64 = 0;

    while let Some(chunk) = stream.next().await {
        if cancelled.load(Ordering::Relaxed) {
            let _ = std::fs::remove_file(&dest_path);
            let payload = ProgressPayload {
                id: id.clone(),
                progress: 0.0,
                downloaded_bytes: downloaded,
                total_bytes,
                speed_kbps: 0.0,
                status: "cancelled".to_string(),
            };
            let _ = state.download_events.send(payload.clone());
            handles.lock().unwrap().remove(&id);
            active_dl.lock().unwrap().remove(&id);
            return Err("Cancelled".to_string());
        }
        let chunk = chunk.map_err(|e| e.to_string())?;
        dest.write_all(&chunk).map_err(|e| e.to_string())?;
        downloaded += chunk.len() as u64;

        let now = std::time::Instant::now();
        if now.duration_since(last_report).as_millis() >= 200 {
            let elapsed = now.duration_since(last_report).as_secs_f64().max(0.001);
            let speed_kbps = ((downloaded - last_downloaded) as f64 / 1024.0) / elapsed;
            let progress = if total_bytes > 0 {
                (downloaded as f64 / total_bytes as f64) * 100.0
            } else {
                0.0
            };
            let payload = ProgressPayload {
                id: id.clone(),
                progress,
                downloaded_bytes: downloaded,
                total_bytes,
                speed_kbps,
                status: "downloading".to_string(),
            };
            let _ = state.download_events.send(payload.clone());
            active_dl.lock().unwrap().insert(id.clone(), payload);
            last_report = now;
            last_downloaded = downloaded;
        }
    }

    let processing_payload = ProgressPayload {
        id: id.clone(),
        progress: 100.0,
        downloaded_bytes: downloaded,
        total_bytes,
        speed_kbps: 0.0,
        status: "processing".to_string(),
    };
    let _ = state.download_events.send(processing_payload.clone());
    active_dl.lock().unwrap().insert(id.clone(), processing_payload);

    let mut mmproj_path: Option<PathBuf> = None;
    if model.is_multimodal() {
        let vp = model.vision_projector.as_ref().ok_or_else(|| {
            "Multimodal catalog entry missing vision_projector download".to_string()
        })?;
        let vp_url = vp
            .resolved_url()
            .ok_or_else(|| "No download URL for vision projector".to_string())?;
        let vp_name = vp
            .local_filename()
            .unwrap_or_else(|| format!("{id}-mmproj.gguf"));
        let vp_dest = models_dir.join(&vp_name);
        crate::resource_import::stream_download(&vp_url, &vp_dest)
            .await
            .map_err(|e| e.to_string())?;
        mmproj_path = Some(vp_dest);
    }

    let result = crate::model_lifecycle::finalize_llm_install(
        model,
        &dest_path,
        mmproj_path.as_deref(),
        Path::new(&storage_path),
    )
    .map_err(|e| e.to_string())?;

    let done_payload = ProgressPayload {
        id: id.clone(),
        progress: 100.0,
        downloaded_bytes: downloaded,
        total_bytes,
        speed_kbps: 0.0,
        status: "complete".to_string(),
    };
    let _ = state.download_events.send(done_payload.clone());
    handles.lock().unwrap().remove(&id);
    active_dl.lock().unwrap().remove(&id);

    serde_json::to_value(result).map_err(|e| e.to_string())
}

// ── Active downloads (persists across page navigation) ────────────────────────

pub fn get_active_downloads() -> Vec<ProgressPayload> {
    let state = crate::state::APP_STATE.get().unwrap();
    state
        .active_downloads
        .lock()
        .unwrap()
        .values()
        .cloned()
        .collect()
}

// ── Remote manifest fetch ─────────────────────────────────────────────────────

pub async fn fetch_remote_manifest(url: String) -> Result<String, String> {
    reqwest::get(&url)
        .await
        .map_err(|e| format!("Network error: {}", e))?
        .text()
        .await
        .map_err(|e| format!("Response error: {}", e))
}

// ── Imported accounts persistence ────────────────────────────────────────────

pub fn imported_accounts_path() -> PathBuf {
    app_meta_dir().join("imported_accounts.json")
}

pub fn load_imported_accounts() -> Result<serde_json::Value, String> {
    let path = imported_accounts_path();
    if !path.exists() {
        return Ok(serde_json::json!([]));
    }
    let s = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&s).map_err(|e| e.to_string())
}

pub fn save_imported_accounts(accounts: serde_json::Value) -> Result<(), String> {
    let meta = app_meta_dir();
    std::fs::create_dir_all(&meta).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(&accounts).map_err(|e| e.to_string())?;
    std::fs::write(imported_accounts_path(), json).map_err(|e| e.to_string())
}

// ── Daemon port + app session tokens ──────────────────────────────────────────

static ACTIVE_DAEMON_PORT: AtomicU16 = AtomicU16::new(0);

/// Records the loopback port chosen when the graph daemon last started.
pub fn set_active_daemon_port(port: u16) {
    ACTIVE_DAEMON_PORT.store(port, Ordering::SeqCst);
}

/// Returns the active daemon port when known, otherwise the configured default.
pub fn get_active_daemon_port() -> u16 {
    let active = ACTIVE_DAEMON_PORT.load(Ordering::SeqCst);
    if active != 0 {
        return active;
    }
    crate::state::APP_STATE
        .get()
        .map(|state| state.config.lock().unwrap().daemon_port)
        .unwrap_or(4242)
}

/// Issues a signed semantic app token scoped to the installed app's manifest shapes.
pub fn build_anatomy_graph_context_json(
    qapp_name: String,
    user_prompt: String,
    agent_reply: String,
) -> Result<String, String> {
    crate::anatomy_context::build_anatomy_graph_context_json(qapp_name, user_prompt, agent_reply)
}

pub fn build_anatomy_graph_context_json_with_dicom(
    qapp_name: String,
    user_prompt: String,
    agent_reply: String,
    dicom_file_path: Option<String>,
) -> Result<String, String> {
    crate::anatomy_context::build_anatomy_graph_context_json_with_dicom(
        qapp_name,
        user_prompt,
        agent_reply,
        dicom_file_path,
    )
}

pub fn parse_dicom_metadata_json(file_path: String) -> Result<String, String> {
    crate::anatomy_context::parse_dicom_metadata_json(file_path)
}

pub fn build_dicom_overlay_spec_json(file_path: String) -> Result<String, String> {
    crate::anatomy_context::build_dicom_overlay_spec_json(file_path)
}

pub fn submit_dicom_ingest(file_path: String, patient_did_hash: u64) -> Result<u64, String> {
    crate::qapp_api::submit_dicom_ingest(file_path, patient_did_hash)
}

pub fn dicom_ingest_status(job_id: u64) -> u8 {
    crate::qapp_api::dicom_ingest_status(job_id)
}

pub fn execute_dicom_volume_query(
    patient_did_hash: u64,
    series_hash: u64,
) -> Result<Vec<u8>, String> {
    crate::qapp_api::execute_dicom_volume_query(patient_did_hash, series_hash)
}

pub fn issue_qapp_session_token(qapp_name: &str) -> Result<String, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let data_dir = state.config.lock().unwrap().storage_path.clone();
    let qapp_dir = qapps_dir(&data_dir).join(qapp_name);
    if !qapp_dir.exists() {
        return Err(format!("Qapp directory not found: {qapp_name}"));
    }

    let manifest = load_qapp_package_from_dir(&qapp_dir)?;
    let qapp_did = format!(
        "did:qualia:qapp:{}",
        manifest.name.to_lowercase().replace(' ', "-")
    );
    let vault = state.key_vault.lock().unwrap();
    vault.issue_qapp_token(&qapp_did, manifest.required_shapes.clone())
}

// ── Qapp launcher ─────────────────────────────────────────────────────────────

/// Load `qapp.json` for an installed qapp.
pub fn load_installed_qapp_package(qapp_name: &str) -> Result<qapp_registry::QappPackageManifest, String> {
    let state = crate::state::APP_STATE.get().ok_or("APP_STATE not initialized")?;
    let data_dir = state.config.lock().unwrap().storage_path.clone();
    let qapp_dir = qapps_dir(&data_dir).join(qapp_name);
    load_qapp_package_from_dir(&qapp_dir)
}

pub(crate) fn load_qapp_package_from_dir(qapp_dir: &Path) -> Result<qapp_registry::QappPackageManifest, String> {
    let manifest_path = resolve_package_manifest_path(qapp_dir)
        .ok_or_else(|| format!("qapp.json not found in {}", qapp_dir.display()))?;
    let content = std::fs::read_to_string(&manifest_path).map_err(|e| e.to_string())?;
    serde_json::from_str::<qapp_registry::QappPackageManifest>(&content)
        .map_err(|e| format!("Invalid qapp package manifest: {e}"))
}

fn resolve_entrypoint_path(
    manifest: &qapp_registry::QappPackageManifest,
    entrypoint: Option<&str>,
) -> String {
    let named_entrypoints = manifest.x_qualia.as_ref().map(|ext| &ext.entrypoints);

    match entrypoint {
        Some(requested) if !requested.trim().is_empty() => named_entrypoints
            .and_then(|map| map.get(requested))
            .cloned()
            .unwrap_or_else(|| requested.to_string()),
        _ => named_entrypoints
            .and_then(|map| map.get("web"))
            .cloned()
            .unwrap_or_else(|| "index.html".to_string()),
    }
}

fn split_asset_and_hash(relative_path: &str) -> (String, Option<String>) {
    let mut parts = relative_path.splitn(2, '#');
    let asset = parts.next().unwrap_or("").trim().trim_start_matches('/');
    let hash = parts
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let asset_path = if asset.is_empty() {
        "index.html".to_string()
    } else {
        asset.to_string()
    };
    (asset_path, hash.map(str::to_string))
}

fn encode_query_component(input: &str) -> String {
    let mut encoded = String::with_capacity(input.len());
    for byte in input.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(char::from(byte));
        } else {
            encoded.push('%');
            encoded.push_str(&format!("{:02X}", byte));
        }
    }
    encoded
}

fn append_launch_context(
    mut base_url: String,
    source: Option<String>,
    surface: Option<String>,
    payload_json: Option<String>,
    qapp_name: Option<&str>,
) -> String {
    let mut params = Vec::new();

    if let Some(source) = source.filter(|value| !value.trim().is_empty()) {
        params.push(format!(
            "qualia_source={}",
            encode_query_component(source.trim())
        ));
    }

    if let Some(surface) = surface.filter(|value| !value.trim().is_empty()) {
        params.push(format!(
            "qualia_surface={}",
            encode_query_component(surface.trim())
        ));
    }

    if let Some(payload_json) = payload_json.filter(|value| !value.trim().is_empty()) {
        params.push(format!(
            "qualia_payload={}",
            encode_query_component(payload_json.trim())
        ));
    }

    if let Some(qapp_name) = qapp_name.filter(|value| !value.trim().is_empty()) {
        if let Ok(token) = issue_qapp_session_token(qapp_name.trim()) {
            params.push(format!(
                "qualia_token={}",
                encode_query_component(&token)
            ));
        }
        let port = get_active_daemon_port();
        if port > 0 {
            params.push(format!("qualia_daemon_port={port}"));
        }
        params.push(format!(
            "qualia_qapp={}",
            encode_query_component(qapp_name.trim())
        ));
    }

    if !params.is_empty() {
        base_url.push(if base_url.contains('?') { '&' } else { '?' });
        base_url.push_str(&params.join("&"));
    }

    base_url
}

fn append_hash_fragment(mut base_url: String, hash_fragment: Option<String>) -> String {
    if let Some(hash_fragment) = hash_fragment {
        base_url.push('#');
        base_url.push_str(&hash_fragment);
    }
    base_url
}

#[derive(Serialize)]
struct SparqlEndpointProbe {
    target: String,
    resolved_endpoint: String,
    reachable: bool,
    status_code: Option<u16>,
    detail: String,
    federation_supported: Option<bool>,
}

#[derive(Serialize)]
struct AppRequirementCheck {
    kind: String,
    id: String,
    required: bool,
    status: String,
    detail: String,
}

#[derive(Serialize)]
struct QappReadinessReport {
    qapp_name: String,
    ready: bool,
    summary: String,
    blocking_issues: usize,
    optional_warnings: usize,
    checks: Vec<AppRequirementCheck>,
}

pub fn load_workspace_catalog() -> qualia_core_db::resource_catalog::ResourceCatalog {
    qualia_core_db::resource_catalog::load_default()
        .unwrap_or_else(|_| qualia_core_db::resource_catalog::ResourceCatalog::empty())
}

fn catalog_has_llm(
    catalog: &qualia_core_db::resource_catalog::ResourceCatalog,
    model: &str,
) -> bool {
    if catalog.find_llm(model).is_some() {
        return true;
    }
    let target = normalize_resource_key(model);
    catalog.llms.iter().any(|entry| {
        normalize_resource_key(&entry.id) == target
            || entry
                .download
                .local_filename()
                .map(|file| normalize_resource_key(&file) == target)
                .unwrap_or(false)
    })
}

fn catalog_has_ontology(
    catalog: &qualia_core_db::resource_catalog::ResourceCatalog,
    ontology: &str,
) -> bool {
    if catalog.find_ontology(ontology).is_some() {
        return true;
    }
    let target = normalize_resource_key(ontology);
    catalog.ontologies.iter().any(|entry| {
        normalize_resource_key(&entry.id) == target
    })
}

fn normalize_resource_key(value: &str) -> String {
    value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

fn directory_contains_requirement(dir: &Path, requirement: &str) -> bool {
    let target = normalize_resource_key(requirement);
    std::fs::read_dir(dir)
        .ok()
        .into_iter()
        .flat_map(|entries| entries.filter_map(Result::ok))
        .any(|entry| {
            let file_name = entry.file_name();
            let candidate = normalize_resource_key(&file_name.to_string_lossy());
            candidate.contains(&target) || target.contains(&candidate)
        })
}

fn collect_matching_files(dir: &Path, requirement: &str) -> Vec<PathBuf> {
    let target = normalize_resource_key(requirement);
    std::fs::read_dir(dir)
        .ok()
        .into_iter()
        .flat_map(|entries| entries.filter_map(Result::ok))
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| {
            let file_name = path
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_default();
            let candidate = normalize_resource_key(&file_name);
            candidate.contains(&target) || target.contains(&candidate)
        })
        .collect()
}

fn resolve_sparql_endpoint_from_catalog(
    endpoint_or_id: &str,
) -> Result<(String, Option<bool>), String> {
    if endpoint_or_id.starts_with("http://") || endpoint_or_id.starts_with("https://") {
        return Ok((endpoint_or_id.to_string(), None));
    }

    let catalog = load_workspace_catalog();
    catalog
        .find_sparql(endpoint_or_id)
        .or_else(|| {
            let target = normalize_resource_key(endpoint_or_id);
            catalog
                .sparql_endpoints
                .iter()
                .find(|entry| normalize_resource_key(&entry.id) == target)
        })
        .map(|entry| (entry.endpoint.clone(), entry.federation_supported))
        .ok_or_else(|| format!("Unknown SPARQL endpoint id: {}", endpoint_or_id))
}

fn evaluate_capability_requirement(
    requirement: &qapp_registry::QappLaunchRequirement,
    daemon_running: bool,
) -> AppRequirementCheck {
    let (status, detail) = match requirement.capability.as_str() {
        "qualia.localDaemon.health" | "qualia.localDaemon.query" => {
            if daemon_running {
                ("ready", "Local Qualia daemon is running.")
            } else {
                ("missing", "Local Qualia daemon is not currently running.")
            }
        }
        "qualia.wasm.execute_ntriples_query"
        | "qualia.wasm.compile_query_to_json"
        | "qualia.wasm.validate_shacl_constraint" => (
            "declared",
            "WASM capability is manifest-declared but not actively verified by the desktop host.",
        ),
        "qualia.flutter.chatRepresentationLaunch" => (
            "ready",
            "Flutter desktop host can launch an app with chat representation context.",
        ),
        _ => (
            "declared",
            "Capability is declared in the manifest but not yet actively checked by the desktop host.",
        ),
    };

    AppRequirementCheck {
        kind: "capability".to_string(),
        id: requirement.capability.clone(),
        required: requirement.required,
        status: status.to_string(),
        detail: detail.to_string(),
    }
}

pub fn inspect_installed_qapp_readiness(qapp_name: String) -> Result<String, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let data_dir = state.config.lock().unwrap().storage_path.clone();
    let qapp_dir = qapps_dir(&data_dir).join(&qapp_name);
    if !qapp_dir.exists() {
        return Err(format!("Qapp directory not found: {qapp_name}"));
    }

    let manifest = load_qapp_package_from_dir(&qapp_dir)?;
    let extension = manifest.x_qualia.clone().unwrap_or_default();
    let daemon_running = *state.daemon_running.lock().unwrap();

    let models_dir = PathBuf::from(&data_dir).join("Models");
    let index_dir = PathBuf::from(&data_dir).join("Index");
    let library_dir = PathBuf::from(&data_dir).join("SemanticLibrary");

    let catalog = load_workspace_catalog();

    let mut checks = Vec::new();

    for requirement in &extension.requires {
        checks.push(evaluate_capability_requirement(requirement, daemon_running));
    }

    if extension.local_daemon.is_some() {
        checks.push(AppRequirementCheck {
            kind: "daemon".to_string(),
            id: "local-daemon".to_string(),
            required: false,
            status: if daemon_running { "ready" } else { "inactive" }.to_string(),
            detail: if daemon_running {
                "Local Qualia daemon is available for app integrations.".to_string()
            } else {
                "App declares local daemon integration, but the daemon is not running.".to_string()
            },
        });
    }

    for ontology in &extension.required_ontologies {
        let in_catalog = catalog_has_ontology(&catalog, ontology);
        let installed = directory_contains_requirement(&index_dir, ontology)
            || directory_contains_requirement(&library_dir, ontology);
        let status = if installed { "ready" } else { "missing" };
        let detail = if installed {
            format!(
                "Ontology `{}` appears to be present in local Qualia storage.",
                ontology
            )
        } else if in_catalog {
            format!(
                "Ontology `{}` is known in the bundled resource catalog but is not installed locally.",
                ontology
            )
        } else {
            format!(
                "Ontology `{}` is required by the app but is not installed and was not found in the bundled catalog.",
                ontology
            )
        };
        checks.push(AppRequirementCheck {
            kind: "ontology".to_string(),
            id: ontology.clone(),
            required: true,
            status: status.to_string(),
            detail,
        });
    }

    for model in &extension.required_models {
        let in_catalog = catalog_has_llm(&catalog, model);
        let installed = directory_contains_requirement(&models_dir, model);
        let status = if installed { "ready" } else { "missing" };
        let detail = if installed {
            format!(
                "Model `{}` appears to be present in the local Models directory.",
                model
            )
        } else if in_catalog {
            format!(
                "Model `{}` is known in the bundled model catalog but is not downloaded locally.",
                model
            )
        } else {
            format!(
                "Model `{}` is required by the app but is not present and was not found in the bundled model catalog.",
                model
            )
        };
        checks.push(AppRequirementCheck {
            kind: "model".to_string(),
            id: model.clone(),
            required: true,
            status: status.to_string(),
            detail,
        });
    }

    for endpoint in &extension.optional_remote_endpoints {
        let match_entry = catalog
            .find_sparql(endpoint)
            .or_else(|| {
                catalog.sparql_endpoints.iter().find(|entry| {
                    entry.endpoint == *endpoint
                        || normalize_resource_key(&entry.id) == normalize_resource_key(endpoint)
                })
            });
        let (status, detail) = if let Some(entry) = match_entry {
            let federation_note = match entry.federation_supported {
                Some(true) => " Federation is advertised as supported.",
                Some(false) => " Federation is not advertised as supported.",
                None => "",
            };
            (
                "cataloged",
                format!(
                    "Endpoint `{}` is known to the bundled SPARQL catalog at {}.{}",
                    endpoint, entry.endpoint, federation_note
                ),
            )
        } else if endpoint.starts_with("http://") || endpoint.starts_with("https://") {
            (
                "declared",
                format!(
                    "Endpoint `{}` is declared directly in the manifest. The desktop host does not currently verify live reachability.",
                    endpoint
                ),
            )
        } else {
            (
                "missing",
                format!(
                    "Endpoint `{}` is not present in the bundled SPARQL catalog and is not an explicit URL.",
                    endpoint
                ),
            )
        };
        checks.push(AppRequirementCheck {
            kind: "sparql-endpoint".to_string(),
            id: endpoint.clone(),
            required: false,
            status: status.to_string(),
            detail,
        });
    }

    let blocking_issues = checks
        .iter()
        .filter(|check| check.required && check.status != "ready")
        .count();
    let optional_warnings = checks
        .iter()
        .filter(|check| {
            !check.required && !matches!(check.status.as_str(), "ready" | "cataloged" | "declared")
        })
        .count();
    let ready = blocking_issues == 0;
    let summary = if ready {
        format!(
            "`{}` is ready to launch with {} optional warnings.",
            qapp_name, optional_warnings
        )
    } else {
        format!(
            "`{}` is missing {} required resources or capabilities.",
            qapp_name, blocking_issues
        )
    };

    let report = QappReadinessReport {
        qapp_name,
        ready,
        summary,
        blocking_issues,
        optional_warnings,
        checks,
    };
    serde_json::to_string(&report).map_err(|e| e.to_string())
}

pub fn list_installed_ontology_artifacts() -> Vec<String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let data_dir = state.config.lock().unwrap().storage_path.clone();
    let dirs = [
        PathBuf::from(&data_dir).join("Index"),
        PathBuf::from(&data_dir).join("SemanticLibrary"),
    ];
    let mut artifacts = Vec::new();

    for dir in dirs {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                if path.is_file() {
                    artifacts.push(entry.file_name().to_string_lossy().to_string());
                }
            }
        }
    }

    artifacts.sort();
    artifacts.dedup();
    artifacts
}

pub fn remove_installed_ontology(ontology_id: String) -> Result<String, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let data_dir = state.config.lock().unwrap().storage_path.clone();
    let dirs = [
        PathBuf::from(&data_dir).join("Index"),
        PathBuf::from(&data_dir).join("SemanticLibrary"),
    ];
    let mut removed = 0usize;

    for dir in dirs {
        for path in collect_matching_files(&dir, &ontology_id) {
            std::fs::remove_file(&path)
                .map_err(|e| format!("Failed to remove {}: {}", path.display(), e))?;
            removed += 1;
        }
    }

    if removed == 0 {
        return Err(format!(
            "No installed ontology artifacts matched `{}`.",
            ontology_id
        ));
    }

    Ok(format!(
        "Removed {} ontology artifact(s) for `{}`.",
        removed, ontology_id
    ))
}

pub fn remove_installed_model(model_id: String) -> Result<String, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let data_dir = state.config.lock().unwrap().storage_path.clone();
    let models_dir = PathBuf::from(&data_dir).join("Models");
    let matches = collect_matching_files(&models_dir, &model_id);
    let mut removed_names = Vec::new();

    for path in matches {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let is_gguf = path
            .extension()
            .map(|ext| ext.to_string_lossy().eq_ignore_ascii_case("gguf"))
            .unwrap_or(false);
        let is_install = name.ends_with(".install.json");
        if is_gguf || is_install {
            std::fs::remove_file(&path)
                .map_err(|e| format!("Failed to remove {}: {}", path.display(), e))?;
            if is_gguf {
                removed_names.push(name);
            }
        }
    }

    if removed_names.is_empty() {
        return Err(format!("No installed model matched `{}`.", model_id));
    }

    {
        let mut active_model = state.active_model.lock().unwrap();
        if let Some(current) = active_model.clone() {
            let normalized_current = normalize_resource_key(&current);
            if removed_names
                .iter()
                .any(|name| normalized_current.contains(&normalize_resource_key(name)))
            {
                *active_model = None;
                let _ = std::fs::remove_file(active_model_path());
                let _ = std::fs::remove_file(legacy_active_model_path());
            }
        }
    }

    Ok(format!(
        "Removed {} model file(s): {}",
        removed_names.len(),
        removed_names.join(", ")
    ))
}

pub fn test_sparql_endpoint(endpoint_or_id: String) -> Result<String, String> {
    let (endpoint, federation_supported) =
        resolve_sparql_endpoint_from_catalog(&endpoint_or_id)?;
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(8))
        .build()
        .map_err(|e| format!("SPARQL probe client error: {}", e))?;

    let response = client
        .get(&endpoint)
        .header("Accept", "application/sparql-results+json, application/json;q=0.9, */*;q=0.1")
        .send();

    let probe = match response {
        Ok(response) => {
            let status = response.status();
            let reachable = status.is_success()
                || status.is_redirection()
                || matches!(status.as_u16(), 400 | 401 | 403 | 405 | 406);
            SparqlEndpointProbe {
                target: endpoint_or_id,
                resolved_endpoint: endpoint,
                reachable,
                status_code: Some(status.as_u16()),
                detail: format!("Endpoint responded with HTTP {}.", status.as_u16()),
                federation_supported,
            }
        }
        Err(err) => SparqlEndpointProbe {
            target: endpoint_or_id,
            resolved_endpoint: endpoint,
            reachable: false,
            status_code: None,
            detail: format!("Endpoint probe failed: {}", err),
            federation_supported,
        },
    };

    serde_json::to_string(&probe).map_err(|e| e.to_string())
}

/// Returns the URL that should be opened in the system browser for a qapp.
/// Looks up by directory name inside `{storage_path}/Qapps/`.
pub fn launch_installed_qapp(qapp_name: String) -> Result<String, String> {
    launch_installed_qapp_with_context(qapp_name.clone(), None, None, None, None)
}

pub fn launch_installed_qapp_with_context(
    qapp_name: String,
    entrypoint: Option<String>,
    surface: Option<String>,
    payload_json: Option<String>,
    source: Option<String>,
) -> Result<String, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let data_dir = state.config.lock().unwrap().storage_path.clone();
    let qapp_dir = qapps_dir(&data_dir).join(&qapp_name);

    if !qapp_dir.exists() {
        return Err(format!("Qapp directory not found: {qapp_name}"));
    }

    let manifest = load_qapp_package_from_dir(&qapp_dir)?;
    let resolved_entrypoint = resolve_entrypoint_path(&manifest, entrypoint.as_deref());
    let (asset_path, hash_fragment) = split_asset_and_hash(&resolved_entrypoint);
    let asset_file = qapp_dir.join(&asset_path);

    let base_url = if let Some(port) = manifest.dev_port {
        let trimmed = asset_path.trim_start_matches('/');
        if trimmed.is_empty() || trimmed == "index.html" {
            format!("http://localhost:{}", port)
        } else {
            format!("http://localhost:{}/{}", port, trimmed)
        }
    } else {
        if !asset_file.exists() {
            return Err(format!("{} not found in {}", asset_path, qapp_dir.display()));
        }

        if crate::qapps_protocol::qualia_protocol_port() != 0 {
            crate::qapps_protocol::qualia_qapp_asset_url(&qapp_name, &asset_path)
                .unwrap_or_else(|_| format!("file:///{}", asset_file.display()).replace('\\', "/"))
        } else {
            format!("file:///{}", asset_file.display()).replace('\\', "/")
        }
    };

    let base_url = append_launch_context(base_url, source, surface, payload_json, Some(&qapp_name));
    Ok(append_hash_fragment(base_url, hash_fragment))
}

// ── Dashboard engine command ───────────────────────────────────────────────────

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

// ─────────────────────────────────────────────────────────────────────────────
// Agent Directory & Delegation Manager
// ─────────────────────────────────────────────────────────────────────────────

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
        routing_hints: vec!["nym:mixnet:sydney1".to_string()],
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

pub fn add_directory_actor(mut actor: Actor) -> Result<(), String> {
    let state = crate::state::APP_STATE.get().unwrap();
    if actor.routing_hints.is_empty() {
        actor.routing_hints.push("nym:mixnet:global".to_string());
    }
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

// ── Qualia protocol + desktop updater ─────────────────────────────────────────

pub fn seed_bundled_qapps() -> Result<Vec<String>, String> {
    crate::bundled_qapps::seed_bundled_qapps()
}

pub fn installed_qapp_version(qapp_name: &str) -> Result<Option<String>, String> {
    let state = crate::state::APP_STATE.get().ok_or("APP_STATE not initialized")?;
    let storage = state.config.lock().map_err(|e| e.to_string())?.storage_path.clone();
    Ok(crate::bundled_qapps::installed_qapp_version(
        Path::new(&storage),
        qapp_name,
    ))
}

pub fn check_qapp_update(qapp_name: String) -> Result<String, String> {
    let state = crate::state::APP_STATE.get().ok_or("APP_STATE not initialized")?;
    let storage = state.config.lock().map_err(|e| e.to_string())?.storage_path.clone();
    let status = crate::bundled_qapps::check_bundled_qapp_update(&qapp_name, Path::new(&storage));
    serde_json::to_string(&status).map_err(|e| e.to_string())
}

pub fn check_qapp_update_from_path(qapp_name: String, source_path: String) -> Result<String, String> {
    let state = crate::state::APP_STATE.get().ok_or("APP_STATE not initialized")?;
    let storage = state.config.lock().map_err(|e| e.to_string())?.storage_path.clone();
    let status = crate::bundled_qapps::check_qapp_update_from_source(
        &qapp_name,
        Path::new(&storage),
        Path::new(&source_path),
    )?;
    serde_json::to_string(&status).map_err(|e| e.to_string())
}

pub fn list_qapp_update_offers() -> Result<String, String> {
    let state = crate::state::APP_STATE.get().ok_or("APP_STATE not initialized")?;
    let storage = state.config.lock().map_err(|e| e.to_string())?.storage_path.clone();
    let offers = crate::bundled_qapps::list_bundled_qapp_updates(Path::new(&storage));
    serde_json::to_string(&offers).map_err(|e| e.to_string())
}

pub fn apply_qapp_update(qapp_name: String) -> Result<String, String> {
    let state = crate::state::APP_STATE.get().ok_or("APP_STATE not initialized")?;
    let storage = state.config.lock().map_err(|e| e.to_string())?.storage_path.clone();
    crate::bundled_qapps::apply_bundled_qapp_update(Path::new(&storage), &qapp_name)
}

pub fn apply_qapp_update_from_path(qapp_name: String, source_path: String) -> Result<String, String> {
    let state = crate::state::APP_STATE.get().ok_or("APP_STATE not initialized")?;
    let storage = state.config.lock().map_err(|e| e.to_string())?.storage_path.clone();
    crate::bundled_qapps::upgrade_qapp_from_source(
        Path::new(&storage),
        &qapp_name,
        Path::new(&source_path),
    )
}

pub fn start_qualia_protocol() -> Result<u16, String> {
    crate::qapps_protocol::start_qualia_protocol()
}

pub fn qualia_protocol_port() -> u16 {
    crate::qapps_protocol::qualia_protocol_port()
}

pub async fn download_and_install_update(url: String) -> Result<(), String> {
    crate::update_installer::download_and_install_update(url).await
}

pub fn register_qualia_uri_handler(exe_path: String) -> Result<(), String> {
    crate::qapps_protocol::register_qualia_uri_handler(&exe_path)
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
    let s = crate::prerequisites::check_prerequisites();
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
    crate::prerequisites::configure_webview2_runtime()
}

pub async fn install_prerequisite(kind: String) -> Result<(), String> {
    crate::prerequisites::install_prerequisite(kind).await
}

// ── main ──────────────────────────────────────────────────────────────────────
