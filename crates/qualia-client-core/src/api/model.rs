//! Active model + lifecycle

#![allow(non_snake_case)]

use super::*;

use crate::state::*;
use futures_util::StreamExt;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};


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

pub fn clear_active_model_record() {
    let _ = std::fs::remove_file(active_model_path());
    let _ = std::fs::remove_file(legacy_active_model_path());
}

pub fn restore_active_model_on_startup() {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let storage_path = Path::new(&storage);

    if let Some(record) = load_active_model_record_from_disk() {
        if Path::new(&record.gguf_path).is_file() {
            log::info!(
                "LLM_LOAD|startup|0.00|Restoring active model {}",
                record.model_id
            );
            match crate::model_lifecycle::activate_model_for_id(&record.model_id, storage_path) {
                Ok(active) => {
                    *state.active_model.lock().unwrap() = Some(active.gguf_path.clone());
                    return;
                }
                Err(err) => {
                    log::error!(
                        "LLM_LOAD|failed|1.00|Startup restore failed for {}: {}",
                        record.model_id,
                        err
                    );
                    *state.active_model.lock().unwrap() = None;
                    clear_active_model_record();
                }
            }
        }
    }

    let catalog = load_workspace_catalog();
    let prefs = crate::model_preferences::ensure_preferences(storage_path, &catalog);
    if prefs.auto_select {
        let _ = try_apply_model_preference("chat");
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
        path.as_ref()
            .map(|gguf| crate::model_lifecycle::ActiveModelRecord {
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
    let storage_path = Path::new(&storage);

    let result = if Path::new(&model_name).is_file() {
        crate::model_lifecycle::finalize_local_gguf(Path::new(&model_name), storage_path)
            .map_err(|e| e.to_string())
    } else {
        let model_id = model_name
            .trim_end_matches(".gguf")
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or(model_name.as_str())
            .to_string();
        crate::model_lifecycle::activate_model_for_id(&model_id, storage_path)
            .map_err(|e| e.to_string())
    };
    let record = match result {
        Ok(record) => record,
        Err(err) => {
            *state.active_model.lock().unwrap() = None;
            clear_active_model_record();
            return Err(err);
        }
    };

    persist_active_model_record(&record)?;
    *state.active_model.lock().unwrap() = Some(record.gguf_path.clone());
    Ok(())
}

/// Evict the resident model from memory without deleting on-disk GGUF files.
pub fn unload_active_model() -> Result<(), String> {
    let state = crate::state::APP_STATE.get().unwrap();
    if let Some(record) = load_active_model_record_from_disk() {
        crate::model_lifecycle::unload_active_model(Some(record.profile_id));
    } else {
        crate::model_lifecycle::unload_active_model(None);
    }
    *state.active_model.lock().unwrap() = None;
    clear_active_model_record();
    Ok(())
}

static MODEL_ACTIVATION_IN_PROGRESS: AtomicBool = AtomicBool::new(false);
static MODEL_ACTIVATION_ERROR: Mutex<Option<String>> = Mutex::new(None);

/// Activate a model on a background thread so the Flutter FRB caller is not blocked.
pub fn set_active_model_async(model_name: String) -> Result<(), String> {
    spawn_model_activation(move || set_active_model(model_name))
}

pub fn try_apply_model_preference_async(task: &str) -> Result<(), String> {
    let task = task.to_string();
    spawn_model_activation(move || try_apply_model_preference(&task))
}

fn spawn_model_activation(work: impl FnOnce() -> Result<(), String> + Send + 'static) -> Result<(), String> {
    if MODEL_ACTIVATION_IN_PROGRESS.load(Ordering::Acquire) {
        return Err("Model activation already in progress".to_string());
    }
    MODEL_ACTIVATION_IN_PROGRESS.store(true, Ordering::Release);
    if let Ok(mut slot) = MODEL_ACTIVATION_ERROR.lock() {
        *slot = None;
    }
    crate::system_telemetry::start_activation_telemetry("Loading model");
    std::thread::Builder::new()
        .name("qualia-model-activate".into())
        .spawn(move || {
            let result = work();
            if let Err(err) = result {
                if let Ok(mut slot) = MODEL_ACTIVATION_ERROR.lock() {
                    *slot = Some(err);
                }
            }
            crate::system_telemetry::stop_activation_telemetry();
            MODEL_ACTIVATION_IN_PROGRESS.store(false, Ordering::Release);
        })
        .map_err(|e| format!("Failed to spawn model activation thread: {e}"))?;
    Ok(())
}

pub fn is_model_activation_in_progress() -> bool {
    MODEL_ACTIVATION_IN_PROGRESS.load(Ordering::Acquire)
}

pub fn take_model_activation_error() -> Option<String> {
    MODEL_ACTIVATION_ERROR.lock().ok()?.take()
}

pub fn get_inference_backend_settings() -> crate::inference_backend::InferenceBackendSettings {
    crate::inference_backend::load_inference_backend_settings()
}

pub fn save_inference_backend_settings(
    settings: crate::inference_backend::InferenceBackendSettings,
) -> Result<(), String> {
    crate::inference_backend::save_inference_backend_settings(&settings)
}

/// Probe the configured Ollama endpoint (tags + reachability).
pub fn probe_ollama_status() -> crate::ollama_harness::OllamaStatus {
    crate::ollama_harness::probe_configured_ollama()
}

pub async fn probe_ollama_status_async() -> crate::ollama_harness::OllamaStatus {
    crate::ollama_harness::probe_configured_ollama_async().await
}

/// List models on the configured Ollama host (empty vec if unreachable).
pub fn list_ollama_models() -> Vec<crate::ollama_harness::OllamaModelInfo> {
    crate::ollama_harness::probe_configured_ollama().models
}

/// One-shot Ollama generate using persisted settings (for smoke / ETL hooks).
pub fn ollama_generate(system: String, prompt: String) -> Result<crate::ollama_harness::OllamaGenerateResult, String> {
    crate::ollama_harness::OllamaHarness::from_loaded_settings().generate(&system, &prompt)
}

pub async fn ollama_generate_async(
    system: String,
    prompt: String,
) -> Result<crate::ollama_harness::OllamaGenerateResult, String> {
    crate::ollama_harness::OllamaHarness::from_loaded_settings()
        .generate_async(&system, &prompt)
        .await
}

pub fn get_model_preferences() -> crate::model_preferences::ModelPreferences {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let catalog = load_workspace_catalog();
    crate::model_preferences::ensure_preferences(Path::new(&storage), &catalog)
}

pub fn save_model_preferences(
    prefs: crate::model_preferences::ModelPreferences,
) -> Result<(), String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    crate::model_preferences::save_preferences(Path::new(&storage), &prefs)
}

pub fn list_installed_llm_ids() -> Vec<String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    crate::model_preferences::list_installed_model_ids(Path::new(&storage))
}

pub fn resolve_model_preference(
    task: &str,
) -> Option<crate::model_preferences::ResolvedModelPreference> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let catalog = load_workspace_catalog();
    let prefs = get_model_preferences();
    let task = crate::model_preferences::ModelTask::from_str_lossy(task);
    crate::model_preferences::resolve_preference(Path::new(&storage), &catalog, &prefs, task)
}

pub fn try_apply_model_preference(task: &str) -> Result<(), String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let catalog = load_workspace_catalog();
    let prefs = get_model_preferences();
    let task = crate::model_preferences::ModelTask::from_str_lossy(task);
    let record = match crate::model_preferences::apply_preference(
        Path::new(&storage),
        &catalog,
        &prefs,
        task,
    ) {
        Ok(record) => record,
        Err(err) => {
            *state.active_model.lock().unwrap() = None;
            clear_active_model_record();
            return Err(err);
        }
    };
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
    handles
        .lock()
        .unwrap()
        .insert(id.clone(), cancelled.clone());

    let client = reqwest::Client::new();
    let response = client.get(&url).send().await.map_err(|e| {
        handles.lock().unwrap().remove(&id);
        active_dl.lock().unwrap().remove(&id);
        e.to_string()
    })?;
    let total_bytes = response.content_length().unwrap_or(0);
    let starting_payload = ProgressPayload {
        id: id.clone(),
        progress: 0.0,
        downloaded_bytes: 0,
        total_bytes,
        speed_kbps: 0.0,
        status: "downloading".to_string(),
    };
    let _ = state.download_events.send(starting_payload.clone());
    active_dl
        .lock()
        .unwrap()
        .insert(id.clone(), starting_payload);

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
    active_dl
        .lock()
        .unwrap()
        .insert(id.clone(), processing_payload);

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

    // New installs should be immediately usable in chat (not left at MappedToDisk).
    if let Ok(record) = crate::model_lifecycle::activate_model_for_id(&id, Path::new(&storage_path))
    {
        let _ = persist_active_model_record(&record);
        *state.active_model.lock().unwrap() = Some(record.gguf_path.clone());
    }

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

