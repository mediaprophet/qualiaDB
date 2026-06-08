//! Catalog LLM download → GGUF shard map → WAL → lifecycle (in-process).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};

use qualia_core_db::{
    gguf_sharder::GGufSharder,
    llm_agent::{AgentBackend, LocalLlmAgent},
    orchestrator::{ModelLifecycle, NullThermalGovernor, TaskOrchestrator},
    q_hash,
    resource_catalog::{LLMResource, ResourceCatalog},
    wal::WriteAheadLog,
};
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum ModelError {
    NotFound(String),
    NoDownloadUrl(String),
    Download(String),
    Shard(String),
    Wal(String),
    Activate(String),
    Io(std::io::Error),
    Json(serde_json::Error),
}

impl std::fmt::Display for ModelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelError::NotFound(id) => write!(f, "LLM not found in catalog: {id}"),
            ModelError::NoDownloadUrl(id) => write!(f, "No download URL for: {id}"),
            ModelError::Download(e) => write!(f, "Download failed: {e}"),
            ModelError::Shard(e) => write!(f, "GGUF shard map failed: {e}"),
            ModelError::Wal(e) => write!(f, "WAL write failed: {e}"),
            ModelError::Activate(e) => write!(f, "Model activation failed: {e}"),
            ModelError::Io(e) => write!(f, "IO error: {e}"),
            ModelError::Json(e) => write!(f, "JSON error: {e}"),
        }
    }
}

impl From<std::io::Error> for ModelError {
    fn from(e: std::io::Error) -> Self {
        ModelError::Io(e)
    }
}

impl From<serde_json::Error> for ModelError {
    fn from(e: serde_json::Error) -> Self {
        ModelError::Json(e)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveModelRecord {
    pub model_id: String,
    pub gguf_path: String,
    pub profile_id: u64,
    pub quantization: String,
    pub lifecycle_state: String,
    #[serde(default)]
    pub modality: String,
    #[serde(default)]
    pub architecture: Option<String>,
    #[serde(default)]
    pub mmproj_path: Option<String>,
    #[serde(default)]
    pub context_window: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelInstallResult {
    pub model_id: String,
    pub gguf_path: String,
    pub profile_id: u64,
    pub pointer_quin_count: usize,
    pub vision_pointer_quin_count: usize,
    pub lifecycle_state: String,
    pub wal_path: String,
    pub modality: String,
    pub mmproj_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelStatus {
    pub active: Option<ActiveModelRecord>,
    pub lifecycle_state: String,
    pub profile_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallManifest {
    pub model_id: String,
    pub gguf_path: String,
    pub profile_id: u64,
    pub quantization: String,
    pub pointer_quin_count: usize,
    pub vision_pointer_quin_count: usize,
    pub installed_at: u64,
    pub wal_path: String,
    pub modality: String,
    pub architecture: Option<String>,
    pub mmproj_path: Option<String>,
    pub context_window: u32,
}

fn orchestrator() -> Arc<TaskOrchestrator> {
    task_orchestrator()
}

/// Shared orchestrator for chat inference and model lifecycle transitions.
pub fn task_orchestrator() -> Arc<TaskOrchestrator> {
    static ORCH: OnceLock<Arc<TaskOrchestrator>> = OnceLock::new();
    ORCH.get_or_init(|| Arc::new(TaskOrchestrator::new(Box::new(NullThermalGovernor))))
        .clone()
}

/// UI RAM ceiling for LLM arena pressure (matches 0.0.9 Flutter plan).
pub const MEMORY_FLOOR_MB: u32 = 512;

static LLM_MEMORY_BYTES: AtomicU64 = AtomicU64::new(0);
static KV_CACHE_USED_MB: AtomicU32 = AtomicU32::new(0);

pub fn record_llm_memory_bytes(bytes: u64) {
    LLM_MEMORY_BYTES.store(bytes, Ordering::Relaxed);
}

pub fn record_llm_memory_sample(bytes: u64) {
    if bytes == 0 {
        return;
    }
    let mut current = LLM_MEMORY_BYTES.load(Ordering::Relaxed);
    while bytes > current {
        match LLM_MEMORY_BYTES.compare_exchange(current, bytes, Ordering::Relaxed, Ordering::Relaxed)
        {
            Ok(_) => return,
            Err(observed) => current = observed,
        }
    }
}

pub fn get_llm_memory_bytes() -> u64 {
    LLM_MEMORY_BYTES.load(Ordering::Relaxed)
}

pub fn record_kv_cache_used_mb(megabytes: u32) {
    KV_CACHE_USED_MB.store(megabytes, Ordering::Relaxed);
}

pub fn get_kv_cache_used_mb() -> u32 {
    KV_CACHE_USED_MB.load(Ordering::Relaxed)
}

pub fn get_thermal_state_label() -> &'static str {
    orchestrator().thermal_state_label()
}

pub fn models_dir(storage_root: &Path) -> PathBuf {
    storage_root.join("Models")
}

pub fn lifecycle_label(state: ModelLifecycle) -> &'static str {
    match state {
        ModelLifecycle::Discovered => "Discovered",
        ModelLifecycle::MappedToDisk => "MappedToDisk",
        ModelLifecycle::StreamingVRAM => "StreamingVRAM",
        ModelLifecycle::Active => "Active",
        ModelLifecycle::Scrubbing => "Scrubbing",
    }
}

fn probe_and_activate_model(
    agent: &LocalLlmAgent,
    profile_id: u64,
    model_label: &str,
    gguf_path: &str,
) -> Result<(), ModelError> {
    let orch = orchestrator();
    if let Some(resident) = orch.resident_model_id() {
        if resident != profile_id {
            log::info!(
                "LLM_LOAD|unload-start|0.01|Evicting resident model 0x{resident:016x} before loading {}",
                model_label
            );
            orch.evict_model(resident);
            let wait_started = std::time::Instant::now();
            while orch.scrubbing_lock.load(Ordering::Acquire) {
                if wait_started.elapsed() > std::time::Duration::from_secs(5) {
                    log::error!(
                        "LLM_LOAD|failed|1.00|Timed out waiting for prior model eviction"
                    );
                    return Err(ModelError::Activate(
                        "Timed out waiting for prior model eviction".to_string(),
                    ));
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            record_llm_memory_bytes(0);
            record_kv_cache_used_mb(0);
            log::info!(
                "LLM_LOAD|unload-done|0.03|Previous resident model scrubbed from memory"
            );
        }
    }
    let mut sys = sysinfo::System::new_all();
    sys.refresh_memory();
    let ram_total_gib = sys.total_memory() as f64 / 1024.0 / 1024.0 / 1024.0;
    let ram_used_gib = sys.used_memory() as f64 / 1024.0 / 1024.0 / 1024.0;
    let ram_free_gib = (ram_total_gib - ram_used_gib).max(0.0);
    {
        let mut state = orch.current_model_state.lock().unwrap();
        *state = ModelLifecycle::MappedToDisk;
        *state = ModelLifecycle::StreamingVRAM;
    }
    log::info!("LLM_LOAD|prepare|0.02|Preparing model {}", model_label);
    log::info!(
        "LLM_LOAD|ram-check|0.04|System RAM {:.1}/{:.1} GiB used; {:.1} GiB available",
        ram_used_gib,
        ram_total_gib,
        ram_free_gib
    );
    match qualia_core_db::gguf_bridge::probe_gguf_runtime(gguf_path) {
        Ok(report) => {
            let kv_cache_mb = (report.kv_cache_bytes / (1024 * 1024)).min(u32::MAX as u64) as u32;
            record_llm_memory_bytes(report.mapped_bytes);
            record_kv_cache_used_mb(kv_cache_mb);
            if let Err(err) = orch.load_model(agent, profile_id) {
                log::error!("LLM_LOAD|failed|1.00|Activation failed: {}", err);
                return Err(ModelError::Activate(err.to_string()));
            }
            orch.register_resident_model(profile_id, report.mapped_bytes + report.kv_cache_bytes);
            log::info!(
                "LLM_LOAD|placement|0.96|Model mapped in system RAM ({:.2} GiB) and KV cache reserved in VRAM/system memory ({} MiB)",
                report.mapped_bytes as f64 / (1024.0 * 1024.0 * 1024.0),
                kv_cache_mb
            );
            log::info!(
                "LLM_LOAD|active|1.00|Model ready (mapped {:.2} GiB, kv {} MiB, backend {})",
                report.mapped_bytes as f64 / (1024.0 * 1024.0 * 1024.0),
                kv_cache_mb,
                if report.directml_enabled {
                    "DirectML+wgpu"
                } else {
                    "wgpu"
                }
            );
        }
        Err(err) => {
            record_llm_memory_bytes(0);
            record_kv_cache_used_mb(0);
            let mut state = orch.current_model_state.lock().unwrap();
            *state = ModelLifecycle::MappedToDisk;
            log::error!("LLM_LOAD|failed|1.00|Activation failed: {}", err);
            return Err(ModelError::Activate(err));
        }
    }
    Ok(())
}

pub fn unload_active_model(profile_id: Option<u64>) {
    let orch = orchestrator();
    let resident = profile_id.or_else(|| orch.resident_model_id());
    if let Some(model_id) = resident {
        log::info!(
            "LLM_LOAD|unload-start|0.00|Unloading resident model 0x{model_id:016x}"
        );
        orch.evict_model(model_id);
        let wait_started = std::time::Instant::now();
        while orch.scrubbing_lock.load(Ordering::Acquire) {
            if wait_started.elapsed() > std::time::Duration::from_secs(5) {
                log::warn!("LLM_LOAD|failed|1.00|Timed out waiting for model scrub");
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        log::info!("LLM_LOAD|unload-done|1.00|Model memory scrub complete");
    }
    record_llm_memory_bytes(0);
    record_kv_cache_used_mb(0);
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn llm_filename(model: &LLMResource) -> String {
    model
        .download
        .local_filename()
        .unwrap_or_else(|| format!("{}.gguf", model.id))
}

fn install_manifest_path(models_dir: &Path, model_id: &str) -> PathBuf {
    models_dir.join(format!("{model_id}.install.json"))
}

fn projector_filename(model: &LLMResource) -> Option<String> {
    model.vision_projector.as_ref().and_then(|d| {
        d.local_filename().or_else(|| {
            d.resolved_url()
                .and_then(|u| u.rsplit('/').next().map(|s| s.to_string()))
        })
    })
}

fn write_install_manifest(
    models_dir: &Path,
    model: &LLMResource,
    gguf_path: &Path,
    mmproj_path: Option<&Path>,
    profile_id: u64,
    pointer_quin_count: usize,
    vision_pointer_quin_count: usize,
    wal_path: &Path,
) -> Result<(), ModelError> {
    let modality = if model.is_multimodal() {
        "multimodal".to_string()
    } else {
        "text".to_string()
    };
    let manifest = InstallManifest {
        model_id: model.id.clone(),
        gguf_path: gguf_path.to_string_lossy().into_owned(),
        profile_id,
        quantization: model
            .quantization
            .clone()
            .unwrap_or_else(|| "Q4_K_M".to_string()),
        pointer_quin_count,
        vision_pointer_quin_count,
        installed_at: unix_now(),
        wal_path: wal_path.to_string_lossy().into_owned(),
        modality: modality.clone(),
        architecture: model.architecture.clone(),
        mmproj_path: mmproj_path.map(|p| p.to_string_lossy().into_owned()),
        context_window: model.effective_context_window(),
    };
    let json = serde_json::to_string_pretty(&manifest)?;
    std::fs::write(install_manifest_path(models_dir, &model.id), json)?;
    Ok(())
}

pub fn finalize_llm_install(
    model: &LLMResource,
    gguf_path: &Path,
    mmproj_path: Option<&Path>,
    storage_root: &Path,
) -> Result<ModelInstallResult, ModelError> {
    let models = models_dir(storage_root);
    std::fs::create_dir_all(&models)?;

    if !gguf_path.is_file() {
        return Err(ModelError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("GGUF not found: {}", gguf_path.display()),
        )));
    }

    if model.is_multimodal() && mmproj_path.is_none() {
        return Err(ModelError::Shard(
            "Multimodal model requires vision projector (mmproj) GGUF".to_string(),
        ));
    }

    let path_str = gguf_path.to_string_lossy().into_owned();
    let sharder = GGufSharder::new(path_str.clone());
    let mut pointer_quins = sharder.generate_bidx_pointer_map();
    let mut vision_pointer_quin_count = 0usize;

    if let Some(mmproj) = mmproj_path {
        if !mmproj.is_file() {
            return Err(ModelError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("mmproj not found: {}", mmproj.display()),
            )));
        }
        let mmproj_str = mmproj.to_string_lossy().into_owned();
        let vision_sharder = GGufSharder::new(mmproj_str);
        let vision_quins = vision_sharder.generate_bidx_pointer_map();
        vision_pointer_quin_count = vision_quins.len();
        pointer_quins.extend(vision_quins);
    }

    let wal_path = models.join("models.wal");
    let mut wal = WriteAheadLog::open(&wal_path)
        .map_err(|e| ModelError::Wal(format!("Cannot open {}: {}", wal_path.display(), e)))?;

    let timestamp = unix_now();
    let prov = model.provenance_quin(timestamp, &path_str);
    wal.append_mutation(&prov)
        .map_err(|e| ModelError::Wal(e.to_string()))?;

    if let Some(src_quin) = model.source_url_quin() {
        wal.append_mutation(&src_quin)
            .map_err(|e| ModelError::Wal(e.to_string()))?;
    }

    for q in &pointer_quins {
        wal.append_mutation(q)
            .map_err(|e| ModelError::Wal(e.to_string()))?;
    }

    for q in &model.to_quins() {
        wal.append_mutation(q)
            .map_err(|e| ModelError::Wal(e.to_string()))?;
    }

    let mmproj_str = mmproj_path.map(|p| p.to_string_lossy().into_owned());
    let profile = model.to_capability_profile_with_projector(&path_str, mmproj_str.as_deref());
    write_install_manifest(
        &models,
        model,
        gguf_path,
        mmproj_path,
        profile.profile_id,
        pointer_quins
            .len()
            .saturating_sub(vision_pointer_quin_count),
        vision_pointer_quin_count,
        &wal_path,
    )?;

    {
        let orch = orchestrator();
        let mut state = orch.current_model_state.lock().unwrap();
        *state = ModelLifecycle::MappedToDisk;
    }

    let modality = if model.is_multimodal() {
        "multimodal".to_string()
    } else {
        "text".to_string()
    };

    Ok(ModelInstallResult {
        model_id: model.id.clone(),
        gguf_path: path_str,
        profile_id: profile.profile_id,
        pointer_quin_count: pointer_quins
            .len()
            .saturating_sub(vision_pointer_quin_count),
        vision_pointer_quin_count,
        lifecycle_state: lifecycle_label(ModelLifecycle::MappedToDisk).to_string(),
        wal_path: wal_path.to_string_lossy().into_owned(),
        modality,
        mmproj_path: mmproj_str,
    })
}

pub async fn install_catalog_llm(
    catalog: &ResourceCatalog,
    id: &str,
    storage_root: &Path,
) -> Result<ModelInstallResult, ModelError> {
    let model = catalog
        .find_llm(id)
        .ok_or_else(|| ModelError::NotFound(id.to_string()))?;

    let url = model
        .download
        .resolved_url()
        .ok_or_else(|| ModelError::NoDownloadUrl(id.to_string()))?;

    let models = models_dir(storage_root);
    std::fs::create_dir_all(&models)?;

    let filename = llm_filename(model);
    let local_path = models.join(&filename);

    crate::resource_import::stream_download(&url, &local_path)
        .await
        .map_err(ModelError::Download)?;

    let mmproj_path = if let (Some(ref vp), Some(vp_url)) = (
        model.vision_projector.as_ref(),
        model
            .vision_projector
            .as_ref()
            .and_then(|d| d.resolved_url()),
    ) {
        let vp_name = projector_filename(model).unwrap_or_else(|| "mmproj.gguf".to_string());
        let vp_path = models.join(&vp_name);
        crate::resource_import::stream_download(&vp_url, &vp_path)
            .await
            .map_err(ModelError::Download)?;
        let _ = vp;
        Some(vp_path)
    } else {
        None
    };

    finalize_llm_install(model, &local_path, mmproj_path.as_deref(), storage_root)
}

pub fn load_install_manifest(storage_root: &Path, model_id: &str) -> Option<InstallManifest> {
    let path = install_manifest_path(&models_dir(storage_root), model_id);
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

fn sanitize_local_model_id(stem: &str) -> String {
    let cleaned: String = stem
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if cleaned.is_empty() {
        "local-model".to_string()
    } else {
        cleaned
    }
}

/// Register and activate a user-selected `.gguf` file at an arbitrary path.
///
/// Writes `{model_id}.install.json` under `{storage_root}/Models/` (path is not copied),
/// maps GGUF tensor pointers into the WAL, and transitions lifecycle → `Active`.
pub fn finalize_local_gguf(
    gguf_path: &Path,
    storage_root: &Path,
) -> Result<ActiveModelRecord, ModelError> {
    if !gguf_path.is_file() {
        return Err(ModelError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("GGUF not found: {}", gguf_path.display()),
        )));
    }

    let models = models_dir(storage_root);
    std::fs::create_dir_all(&models)?;

    let stem = gguf_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("local-model");
    let model_id = sanitize_local_model_id(stem);
    let path_str = gguf_path.to_string_lossy().into_owned();
    let profile_id = q_hash(&format!("profile:local:{model_id}"));

    let sharder = GGufSharder::new(path_str.clone());
    let pointer_quins = sharder.generate_bidx_pointer_map();
    let pointer_quin_count = pointer_quins.len();

    let wal_path = models.join("models.wal");
    let mut wal = WriteAheadLog::open(&wal_path)
        .map_err(|e| ModelError::Wal(format!("Cannot open {}: {}", wal_path.display(), e)))?;

    let subject = q_hash(&format!("local-gguf:{model_id}"));
    let predicate = q_hash("prov:wasDerivedFrom");
    let object = q_hash(&path_str);
    let context = q_hash("ctx:local-gguf");
    let prov = qualia_core_db::QualiaQuin {
        subject,
        predicate,
        object,
        context,
        metadata: unix_now(),
        parity: subject ^ predicate ^ object ^ context,
    };
    wal.append_mutation(&prov)
        .map_err(|e| ModelError::Wal(e.to_string()))?;

    for quin in pointer_quins {
        wal.append_mutation(&quin)
            .map_err(|e| ModelError::Wal(e.to_string()))?;
    }

    let manifest = InstallManifest {
        model_id: model_id.clone(),
        gguf_path: path_str.clone(),
        profile_id,
        quantization: "local".to_string(),
        pointer_quin_count,
        vision_pointer_quin_count: 0,
        installed_at: unix_now(),
        wal_path: wal_path.to_string_lossy().into_owned(),
        modality: "text".to_string(),
        architecture: None,
        mmproj_path: None,
        context_window: 4096,
    };
    let json = serde_json::to_string_pretty(&manifest)?;
    std::fs::write(install_manifest_path(&models, &model_id), json)?;

    let agent = LocalLlmAgent::with_local_backend(
        format!("did:qualia:local-gguf:{profile_id}"),
        AgentBackend::Local {
            model_path: path_str.clone(),
            context_window: 4096,
            quantization: "local".to_string(),
            vision_projector_path: None,
            modality: "text".to_string(),
            architecture: None,
        },
    );
    probe_and_activate_model(&agent, profile_id, &model_id, &path_str)?;

    let lifecycle = *orchestrator().current_model_state.lock().unwrap();
    Ok(ActiveModelRecord {
        model_id,
        gguf_path: path_str,
        profile_id,
        quantization: "local".to_string(),
        lifecycle_state: lifecycle_label(lifecycle).to_string(),
        modality: "text".to_string(),
        architecture: None,
        mmproj_path: None,
        context_window: 4096,
    })
}

pub fn activate_model_for_id(
    model_id: &str,
    storage_root: &Path,
) -> Result<ActiveModelRecord, ModelError> {
    let manifest = load_install_manifest(storage_root, model_id).ok_or_else(|| {
        ModelError::Activate(format!(
            "No install manifest for `{model_id}` — download the model in LLM Hub first"
        ))
    })?;

    if !Path::new(&manifest.gguf_path).is_file() {
        return Err(ModelError::Activate(format!(
            "GGUF missing at {}",
            manifest.gguf_path
        )));
    }

    if manifest.modality == "multimodal" {
        let mmproj = manifest.mmproj_path.as_deref().ok_or_else(|| {
            ModelError::Activate("Multimodal manifest missing mmproj_path".to_string())
        })?;
        if !Path::new(mmproj).is_file() {
            return Err(ModelError::Activate(format!(
                "Vision projector missing at {mmproj}"
            )));
        }
    }

    let agent = LocalLlmAgent::with_local_backend(
        format!("did:qualia:profile:{}", manifest.profile_id),
        AgentBackend::Local {
            model_path: manifest.gguf_path.clone(),
            context_window: manifest.context_window,
            quantization: manifest.quantization.clone(),
            vision_projector_path: manifest.mmproj_path.clone(),
            modality: manifest.modality.clone(),
            architecture: manifest.architecture.clone(),
        },
    );
    probe_and_activate_model(
        &agent,
        manifest.profile_id,
        &manifest.model_id,
        &manifest.gguf_path,
    )?;

    let lifecycle = *orchestrator().current_model_state.lock().unwrap();
    let record = ActiveModelRecord {
        model_id: manifest.model_id,
        gguf_path: manifest.gguf_path,
        profile_id: manifest.profile_id,
        quantization: manifest.quantization,
        lifecycle_state: lifecycle_label(lifecycle).to_string(),
        modality: manifest.modality,
        architecture: manifest.architecture,
        mmproj_path: manifest.mmproj_path,
        context_window: manifest.context_window,
    };

    Ok(record)
}

pub fn activate_model(
    profile_id: u64,
    storage_root: &Path,
) -> Result<ActiveModelRecord, ModelError> {
    let models = models_dir(storage_root);
    let entries = std::fs::read_dir(&models).map_err(ModelError::Io)?;
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        if !path
            .file_name()
            .map(|n| n.to_string_lossy().contains(".install.json"))
            .unwrap_or(false)
        {
            continue;
        }
        if let Ok(text) = std::fs::read_to_string(&path) {
            if let Ok(manifest) = serde_json::from_str::<InstallManifest>(&text) {
                if manifest.profile_id == profile_id {
                    return activate_model_for_id(&manifest.model_id, storage_root);
                }
            }
        }
    }
    Err(ModelError::Activate(format!(
        "No installed model with profile_id 0x{profile_id:016x}"
    )))
}

pub fn get_model_lifecycle_state() -> ModelLifecycle {
    *orchestrator().current_model_state.lock().unwrap()
}

pub fn get_model_status(active: Option<ActiveModelRecord>) -> ModelStatus {
    let lifecycle = get_model_lifecycle_state();
    ModelStatus {
        profile_id: active.as_ref().map(|r| r.profile_id),
        active,
        lifecycle_state: lifecycle_label(lifecycle).to_string(),
    }
}
