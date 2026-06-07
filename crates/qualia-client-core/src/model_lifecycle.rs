//! Catalog LLM download → GGUF shard map → WAL → lifecycle (in-process).

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use qualia_core_db::{
    gguf_sharder::GGufSharder,
    orchestrator::{ModelLifecycle, NullThermalGovernor, TaskOrchestrator},
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
    static ORCH: OnceLock<Arc<TaskOrchestrator>> = OnceLock::new();
    ORCH.get_or_init(|| Arc::new(TaskOrchestrator::new(Box::new(NullThermalGovernor))))
        .clone()
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
            d.resolved_url().and_then(|u| u.rsplit('/').next().map(|s| s.to_string()))
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
    let profile = model.to_capability_profile_with_projector(
        &path_str,
        mmproj_str.as_deref(),
    );
    write_install_manifest(
        &models,
        model,
        gguf_path,
        mmproj_path,
        profile.profile_id,
        pointer_quins.len().saturating_sub(vision_pointer_quin_count),
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
        pointer_quin_count: pointer_quins.len().saturating_sub(vision_pointer_quin_count),
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

    let mmproj_path = if let (Some(ref vp), Some(vp_url)) =
        (model.vision_projector.as_ref(), model.vision_projector.as_ref().and_then(|d| d.resolved_url()))
    {
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

    finalize_llm_install(
        model,
        &local_path,
        mmproj_path.as_deref(),
        storage_root,
    )
}

pub fn load_install_manifest(storage_root: &Path, model_id: &str) -> Option<InstallManifest> {
    let path = install_manifest_path(&models_dir(storage_root), model_id);
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn activate_model_for_id(model_id: &str, storage_root: &Path) -> Result<ActiveModelRecord, ModelError> {
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

    orchestrator()
        .load_model(manifest.profile_id)
        .map_err(|e| ModelError::Activate(e.to_string()))?;

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

pub fn activate_model(profile_id: u64, storage_root: &Path) -> Result<ActiveModelRecord, ModelError> {
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
