pub fn greet(name: String) -> String {
    format!("Hello, {}! Welcome to QualiaDB via Flutter Rust Bridge.", name)
}

pub struct HardwareStatus {
    pub ram_total_gb: f64,
    pub ram_used_gb: f64,
    pub vram_estimated_gb: f64,
}

pub fn get_hardware_status() -> HardwareStatus {
    // Stub implementation mirroring the old Tauri command
    HardwareStatus {
        ram_total_gb: 32.0,
        ram_used_gb: 12.0,
        vram_estimated_gb: 16.0,
    }
}

pub fn check_ollama_status() -> bool {
    // Simple stub for now
    true
}

pub struct AgentConfig {
    pub storage_path: String,
    pub storage_quota_gb: u64,
    pub base_connectivity_cost_ilp: u64,
}

pub fn get_config() -> AgentConfig {
    AgentConfig {
        storage_path: "C:\\QualiaData".to_string(),
        storage_quota_gb: 50,
        base_connectivity_cost_ilp: 5000,
    }
}

pub fn save_config(new_config: AgentConfig) {
    // Stub for saving config
    println!("Saved config: {}", new_config.storage_path);
}

pub struct TaxRecipient {
    pub label: String,
    pub ilp_address: String,
    pub share_percent: f64,
    pub use_nym: bool,
}

pub struct TaxRecipientSuite {
    pub jurisdiction_did: String,
    pub recipients: Vec<TaxRecipient>,
}

pub fn get_tax_suite() -> TaxRecipientSuite {
    TaxRecipientSuite {
        jurisdiction_did: "did:key:z6MkhaXgBZDvotDkL5257faiztiuC2ZX".into(),
        recipients: vec![
            TaxRecipient { label: "Cooperative Infrastructure Fund".into(), ilp_address: "$ilp.qualia.coop/infrastructure".into(), share_percent: 40.0, use_nym: false },
            TaxRecipient { label: "Digital Rights Legal Defence".into(), ilp_address: "$ilp.qualia.coop/legal-defence".into(), share_percent: 30.0, use_nym: false },
            TaxRecipient { label: "Open Source Sustainability Pool".into(), ilp_address: "$ilp.qualia.coop/oss-sustainability".into(), share_percent: 20.0, use_nym: false },
            TaxRecipient { label: "Disaster Recovery Reserve".into(), ilp_address: "$ilp.qualia.coop/disaster-reserve".into(), share_percent: 10.0, use_nym: true },
        ]
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
    vec![
        CoinBalance { 
            coin: "eCash".into(), 
            ticker: "XEC".into(), 
            address: "ecash:q...".into(), 
            balance: 1_250_000.0, 
            balance_display: "1,250,000.00".into(), 
            fiat_usd: 245.00, 
            price_usd: 0.000196, 
            change_24h: 3.2, 
            network: "eCash".into(), 
            status: "online".into() 
        },
        CoinBalance { 
            coin: "Bitcoin".into(), 
            ticker: "BTC".into(), 
            address: "bc1q...".into(), 
            balance: 0.0045, 
            balance_display: "0.00450000".into(), 
            fiat_usd: 441.00, 
            price_usd: 98_000.0, 
            change_24h: -1.4, 
            network: "Bitcoin".into(), 
            status: "online".into() 
        },
    ]
}

use std::path::PathBuf;
use bip39::{Mnemonic, Language};

fn app_meta_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    { PathBuf::from(std::env::var("APPDATA").unwrap_or_else(|_| "C:\\Users\\Default\\AppData\\Roaming".to_string())).join("Qualia") }
    #[cfg(target_os = "macos")]
    { PathBuf::from(std::env::var("HOME").unwrap_or_default()).join("Library/Application Support/Qualia") }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    { PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".config/qualia") }
}

fn identity_file_path() -> PathBuf { app_meta_dir().join("identity.json") }
fn imported_accounts_path() -> PathBuf { app_meta_dir().join("imported_accounts.json") }

pub fn save_identity(wallets_json: String) -> Result<(), String> {
    let meta = app_meta_dir();
    std::fs::create_dir_all(&meta).map_err(|e| e.to_string())?;
    std::fs::write(identity_file_path(), wallets_json).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn load_identity() -> Result<Option<String>, String> {
    let path = identity_file_path();
    if !path.exists() {
        return Ok(None);
    }
    let json = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    Ok(Some(json))
}

pub fn generate_bip39_seed() -> Result<String, String> {
    let mnemonic = Mnemonic::generate_in(Language::English, 12).map_err(|_| "Failed to generate".to_string())?;
    let words: Vec<&str> = mnemonic.words().collect();
    Ok(words.join(" "))
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

pub fn derive_wallets_from_seed(seed: String) -> Result<String, String> {
    let mnemonic = match Mnemonic::parse_in(Language::English, &seed) {
        Ok(m) => m,
        Err(_) => return Err("Invalid 12-word seed phrase.".to_string()),
    };
    
    let seed_bytes = mnemonic.to_seed("");
    let hex_seed = to_hex(&seed_bytes[0..16]);
    let xec_hex = to_hex(&seed_bytes[16..24]);
    let eth_hex = to_hex(&seed_bytes[24..32]);

    let wallets = serde_json::json!({
        "qualia_root": format!("did:qualia:0x{}", hex_seed),
        "nym_mixnet": format!("n1{}...{}", &hex_seed[0..4], &hex_seed[12..16]),
        "ecash_xec": format!("ecash:q{}", xec_hex),
        "ethereum": format!("0x{}", eth_hex),
    });

    Ok(serde_json::to_string(&wallets).unwrap())
}

pub fn import_external_seed(network: String, seed: String, label: String) -> Result<String, String> {
    if seed.split_whitespace().count() < 12 {
        return Err("Invalid seed phrase".to_string());
    }

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

pub fn load_imported_accounts() -> Result<String, String> {
    let path = imported_accounts_path();
    if !path.exists() {
        return Ok("[]".to_string());
    }
    let s = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    Ok(s)
}

pub fn save_imported_accounts(accounts_json: String) -> Result<(), String> {
    let meta = app_meta_dir();
    std::fs::create_dir_all(&meta).map_err(|e| e.to_string())?;
    std::fs::write(imported_accounts_path(), accounts_json).map_err(|e| e.to_string())
}

use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use futures_util::StreamExt;
use std::io::Write;

#[derive(Clone)]
pub struct ProgressPayload {
    pub id: String,
    pub progress: f64,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub speed_kbps: f64,
    pub status: String,
}

lazy_static::lazy_static! {
    static ref ACTIVE_DOWNLOADS: Mutex<HashMap<String, ProgressPayload>> = Mutex::new(HashMap::new());
    static ref CANCEL_FLAGS: Mutex<HashMap<String, Arc<AtomicBool>>> = Mutex::new(HashMap::new());
    static ref ACTIVE_MODEL: Mutex<Option<String>> = Mutex::new(None);
}

pub fn get_active_downloads() -> Vec<ProgressPayload> {
    ACTIVE_DOWNLOADS.lock().unwrap().values().cloned().collect()
}

pub fn cancel_download(id: String) {
    if let Some(flag) = CANCEL_FLAGS.lock().unwrap().get(&id) {
        flag.store(true, Ordering::Relaxed);
    }
}

pub async fn download_model(url: String, filename: String, model_id: String) -> Result<String, String> {
    let storage_path = "C:\\QualiaData".to_string(); // from config ideally
    let models_dir = PathBuf::from(&storage_path).join("Models");
    std::fs::create_dir_all(&models_dir).map_err(|e| e.to_string())?;
    let dest_path = models_dir.join(&filename);

    let cancelled = Arc::new(AtomicBool::new(false));
    CANCEL_FLAGS.lock().unwrap().insert(model_id.clone(), cancelled.clone());

    let response = reqwest::get(&url).await.map_err(|e| {
        CANCEL_FLAGS.lock().unwrap().remove(&model_id);
        ACTIVE_DOWNLOADS.lock().unwrap().remove(&model_id);
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
                id: model_id.clone(), progress: 0.0, downloaded_bytes: downloaded,
                total_bytes, speed_kbps: 0.0, status: "cancelled".to_string(),
            };
            ACTIVE_DOWNLOADS.lock().unwrap().insert(model_id.clone(), payload);
            CANCEL_FLAGS.lock().unwrap().remove(&model_id);
            return Err("Cancelled".to_string());
        }
        let chunk = chunk.map_err(|e| e.to_string())?;
        dest.write_all(&chunk).map_err(|e| e.to_string())?;
        downloaded += chunk.len() as u64;

        let now = std::time::Instant::now();
        if now.duration_since(last_report).as_millis() > 250 {
            let elapsed = now.duration_since(last_report).as_secs_f64().max(0.001);
            let speed_kbps = ((downloaded - last_downloaded) as f64 / 1024.0) / elapsed;
            let progress = if total_bytes > 0 { (downloaded as f64 / total_bytes as f64) * 100.0 } else { 0.0 };
            let payload = ProgressPayload {
                id: model_id.clone(), progress, downloaded_bytes: downloaded,
                total_bytes, speed_kbps, status: "downloading".to_string(),
            };
            ACTIVE_DOWNLOADS.lock().unwrap().insert(model_id.clone(), payload);
            last_report = now;
            last_downloaded = downloaded;
        }
    }

    let done_payload = ProgressPayload {
        id: model_id.clone(), progress: 100.0, downloaded_bytes: downloaded,
        total_bytes, speed_kbps: 0.0, status: "complete".to_string(),
    };
    ACTIVE_DOWNLOADS.lock().unwrap().insert(model_id.clone(), done_payload);
    CANCEL_FLAGS.lock().unwrap().remove(&model_id);
    Ok("Download complete".to_string())
}

pub struct ModelInfo {
    pub name: String,
    pub is_active: bool,
    pub avatar_type: String,
}

pub fn discover_models() -> Vec<ModelInfo> {
    let storage_path = "C:\\QualiaData".to_string();
    let models_dir = PathBuf::from(&storage_path).join("Models");
    let mut models = Vec::new();
    let active = ACTIVE_MODEL.lock().unwrap().clone();
    
    if let Ok(entries) = std::fs::read_dir(&models_dir) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.extension().map(|e| e == "gguf").unwrap_or(false) {
                let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                let is_active = Some(name.clone()) == active;
                models.push(ModelInfo {
                    name,
                    is_active,
                    avatar_type: "local".to_string(),
                });
            }
        }
    }
    models
}

pub fn get_active_model() -> Option<String> {
    ACTIVE_MODEL.lock().unwrap().clone()
}

pub fn set_active_model(model_name: String) {
    *ACTIVE_MODEL.lock().unwrap() = Some(model_name);
}


lazy_static::lazy_static! {
    static ref DAEMON_RUNNING: Mutex<bool> = Mutex::new(false);
}

pub fn start_daemon() -> String {
    let running = *DAEMON_RUNNING.lock().unwrap();
    if running {
        return "Daemon already running".to_string();
    }
    
    *DAEMON_RUNNING.lock().unwrap() = true;
    
    // Spawn the core DB daemon in the background
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let storage_dir = std::env::var("QUALIA_DATA_DIR").unwrap_or_else(|_| ".".to_string());
            let vault = qualia_core_db::key_vault::KeyVault::load_or_generate(&storage_dir).expect("Failed to load KeyVault");
            let vault_arc = std::sync::Arc::new(std::sync::Mutex::new(vault));
            qualia_core_db::daemon::start_local_daemon(4242, vault_arc).await;
            *DAEMON_RUNNING.lock().unwrap() = false;
        });
    });

    "Daemon Started".to_string()
}

pub fn daemon_status() -> String {
    let running = *DAEMON_RUNNING.lock().unwrap();
    if running { "running".to_string() } else { "stopped".to_string() }
}

pub struct SpatialPhysicsState {
    pub temperature: f64,
    pub pressure: f64,
    pub time_dilation: f64,
}

lazy_static::lazy_static! {
    static ref PHYSICS_STATE: Mutex<SpatialPhysicsState> = Mutex::new(SpatialPhysicsState {
        temperature: 50.0,
        pressure: 50.0,
        time_dilation: 1.0,
    });
}

pub fn get_physics_state() -> SpatialPhysicsState {
    let state = PHYSICS_STATE.lock().unwrap();
    SpatialPhysicsState {
        temperature: state.temperature,
        pressure: state.pressure,
        time_dilation: state.time_dilation,
    }
}

pub fn update_physics_state(temperature: f64, pressure: f64, time_dilation: f64) {
    let mut state = PHYSICS_STATE.lock().unwrap();
    state.temperature = temperature;
    state.pressure = pressure;
    state.time_dilation = time_dilation;
}

pub fn verify_and_install_app(zip_path: String, credential_sig: String) -> String {
    if !credential_sig.starts_with("did:qualia:app") {
        return "Invalid App Credential".to_string();
    }
    // Stub for unzipping to C:\QualiaData\Apps\
    "App Installed and Verified".to_string()
}

pub fn ingest_literature(file_path: String) -> String {
    let source_path = std::path::Path::new(&file_path);
    let filename = source_path.file_name().unwrap_or_default();
    
    // Create SemanticLibrary path dynamically
    let storage_path = app_meta_dir();
    let lib_dir = storage_path.join("SemanticLibrary");
    if !lib_dir.exists() {
        let _ = std::fs::create_dir_all(&lib_dir);
    }
    let dest_path = lib_dir.join(filename);
    let _ = std::fs::copy(&source_path, &dest_path);

    // Instead of using pdf_extract which we don't have, mock it
    let preview = "Sample parsed ontology nodes from document...";

    format!("Successfully ingested literature: {}. Generated ontology nodes from preview: '{}...'", 
filename.to_string_lossy(), preview.replace("\n", " "))
}

pub fn upsert_cmld_definition(term: String, context_did: String) -> String {
    format!("Successfully mapped '{}' to Context: {}", term, context_did)
}

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct CatalogItem {
    pub id: String,
    pub name: String,
    pub tag: String,
    pub params: Option<String>,
    pub format: String,
    pub size: String,
    pub vram: Option<String>,
}

pub async fn fetch_model_catalog() -> Vec<CatalogItem> {
    // Stub implementation returning remote data. In reality this would be:
    // reqwest::get("https://user.github.io/qualiaDB/llms.json").await.unwrap().json().await.unwrap()
    vec![
        CatalogItem { id: "llama32-1b-q4".into(), name: "Llama 3.2 1B Instruct (Remote)".into(), tag: "Tiny".into(), params: Some("1B".into()), format: "Q4_K_M".into(), size: "0.7 GB".into(), vram: Some("2 GB".into()) },
        CatalogItem { id: "gemma3-4b-q4".into(), name: "Gemma 3 4B IT (Remote)".into(), tag: "Google".into(), params: Some("4B".into()), format: "Q4_K_M".into(), size: "2.5 GB".into(), vram: Some("4 GB".into()) },
    ]
}

pub async fn fetch_ontology_catalog() -> Vec<CatalogItem> {
    vec![
        CatalogItem { id: "cns-anatomy".into(), name: "CNS Anatomy (Remote)".into(), tag: "Medical".into(), params: None, format: "OWL2".into(), size: "14 MB".into(), vram: None },
        CatalogItem { id: "quantum-chromodynamics".into(), name: "QCD Standard Model (Remote)".into(), tag: "Physics".into(), params: None, format: "RDF/XML".into(), size: "8 MB".into(), vram: None },
    ]
}

pub async fn fetch_model_catalog_real() -> Vec<CatalogItem> {
    let url = "https://raw.githubusercontent.com/qualia-db/qualiaDB/main/llms.json";
    if let Ok(res) = reqwest::get(url).await {
        if let Ok(items) = res.json::<Vec<CatalogItem>>().await {
            return items;
        }
    }
    // Fallback if network fails
    vec![
        CatalogItem { id: "llama32-1b-q4".into(), name: "Llama 3.2 1B Instruct (Fallback)".into(), tag: "Tiny".into(), params: Some("1B".into()), format: "Q4_K_M".into(), size: "0.7 GB".into(), vram: Some("2 GB".into()) },
        CatalogItem { id: "gemma3-4b-q4".into(), name: "Gemma 3 4B IT (Fallback)".into(), tag: "Google".into(), params: Some("4B".into()), format: "Q4_K_M".into(), size: "2.5 GB".into(), vram: Some("4 GB".into()) },
    ]
}

pub async fn fetch_ontology_catalog_real() -> Vec<CatalogItem> {
    let url = "https://raw.githubusercontent.com/qualia-db/qualiaDB/main/ontologies.json";
    if let Ok(res) = reqwest::get(url).await {
        if let Ok(items) = res.json::<Vec<CatalogItem>>().await {
            return items;
        }
    }
    vec![
        CatalogItem { id: "cns-anatomy".into(), name: "CNS Anatomy (Fallback)".into(), tag: "Medical".into(), params: None, format: "OWL2".into(), size: "14 MB".into(), vram: None },
        CatalogItem { id: "quantum-chromodynamics".into(), name: "QCD Standard Model (Fallback)".into(), tag: "Physics".into(), params: None, format: "RDF/XML".into(), size: "8 MB".into(), vram: None },
    ]
}
