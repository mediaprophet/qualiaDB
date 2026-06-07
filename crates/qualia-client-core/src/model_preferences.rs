//! User-defined LLM load priority and conditional selection.

use std::path::{Path, PathBuf};

use qualia_core_db::resource_catalog::ResourceCatalog;
use serde::{Deserialize, Serialize};
use sysinfo::System;

use crate::model_lifecycle::{self, models_dir, load_install_manifest, ActiveModelRecord};

const PREFS_FILE: &str = "model_preferences.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ModelTask {
    #[default]
    Always,
    Chat,
    Coding,
    Vision,
    LowRam,
}

impl ModelTask {
    pub fn label(self) -> &'static str {
        match self {
            ModelTask::Always => "Whenever installed",
            ModelTask::Chat => "General chat",
            ModelTask::Coding => "Coding / reasoning",
            ModelTask::Vision => "Vision / multimodal",
            ModelTask::LowRam => "Low RAM only",
        }
    }

    pub fn from_str_lossy(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "chat" => ModelTask::Chat,
            "coding" => ModelTask::Coding,
            "vision" => ModelTask::Vision,
            "low_ram" | "lowram" => ModelTask::LowRam,
            _ => ModelTask::Always,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            ModelTask::Always => "always",
            ModelTask::Chat => "chat",
            ModelTask::Coding => "coding",
            ModelTask::Vision => "vision",
            ModelTask::LowRam => "low_ram",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelLoadCondition {
    #[serde(default = "default_true")]
    pub require_installed: bool,
    #[serde(default)]
    pub task: ModelTask,
    #[serde(default)]
    pub min_ram_gb: Option<f64>,
    #[serde(default = "default_true")]
    pub respect_ram_estimate: bool,
    #[serde(default)]
    pub require_multimodal: bool,
}

impl Default for ModelLoadCondition {
    fn default() -> Self {
        Self {
            require_installed: true,
            task: ModelTask::Always,
            min_ram_gb: None,
            respect_ram_estimate: true,
            require_multimodal: false,
        }
    }
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPreferenceEntry {
    pub model_id: String,
    pub label: String,
    pub priority: u32,
    #[serde(default)]
    pub when: ModelLoadCondition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPreferences {
    #[serde(default)]
    pub auto_select: bool,
    #[serde(default)]
    pub entries: Vec<ModelPreferenceEntry>,
}

impl Default for ModelPreferences {
    fn default() -> Self {
        Self {
            auto_select: true,
            entries: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ResolvedModelPreference {
    pub model_id: String,
    pub label: String,
    pub reason: String,
    pub gguf_path: String,
    pub priority: u32,
    pub task: String,
}

pub fn preferences_path(storage_root: &Path) -> PathBuf {
    storage_root.join("Meta").join(PREFS_FILE)
}

pub fn load_preferences(storage_root: &Path) -> ModelPreferences {
    let path = preferences_path(storage_root);
    let Ok(text) = std::fs::read_to_string(&path) else {
        return ModelPreferences::default();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

pub fn save_preferences(storage_root: &Path, prefs: &ModelPreferences) -> Result<(), String> {
    let path = preferences_path(storage_root);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(prefs).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}

pub fn list_installed_model_ids(storage_root: &Path) -> Vec<String> {
    let models = models_dir(storage_root);
    let Ok(entries) = std::fs::read_dir(&models) else {
        return vec![];
    };
    let mut ids = Vec::new();
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !name.ends_with(".install.json") {
            continue;
        }
        let model_id = name.trim_end_matches(".install.json");
        if load_install_manifest(storage_root, model_id).is_some() {
            ids.push(model_id.to_string());
        }
    }
    ids.sort();
    ids
}

fn system_ram_gb() -> f64 {
    let mut sys = System::new_all();
    sys.refresh_memory();
    sys.total_memory() as f64 / 1024.0 / 1024.0 / 1024.0
}

fn catalog_ram_mb(catalog: &ResourceCatalog, model_id: &str) -> Option<u32> {
    catalog
        .find_llm(model_id)
        .and_then(|m| m.ram_estimate_mb.or(m.size_mb))
}

fn catalog_is_multimodal(catalog: &ResourceCatalog, model_id: &str) -> bool {
    catalog
        .find_llm(model_id)
        .map(|m| m.is_multimodal())
        .unwrap_or(false)
}

fn task_matches(
    condition_task: ModelTask,
    request_task: ModelTask,
    catalog: &ResourceCatalog,
    model_id: &str,
) -> bool {
    if condition_task == ModelTask::Always {
        return true;
    }
    if condition_task != request_task && request_task != ModelTask::Always {
        return false;
    }
    let Some(llm) = catalog.find_llm(model_id) else {
        return condition_task == request_task;
    };
    match condition_task {
        ModelTask::Coding => {
            llm.tags
                .as_ref()
                .map(|t| t.iter().any(|tag| tag == "coding" || tag == "reasoning"))
                .unwrap_or(false)
                || llm
                    .recommended_for
                    .as_ref()
                    .map(|r| r.iter().any(|x| x == "coding"))
                    .unwrap_or(false)
        }
        ModelTask::Vision => catalog_is_multimodal(catalog, model_id),
        ModelTask::LowRam => {
            let ram = catalog_ram_mb(catalog, model_id).unwrap_or(u32::MAX);
            ram <= 2000
                || llm
                    .recommended_for
                    .as_ref()
                    .map(|r| {
                        r.iter()
                            .any(|x| x == "low_ram" || x == "very_low_ram" || x == "edge")
                    })
                    .unwrap_or(false)
        }
        ModelTask::Chat => true,
        ModelTask::Always => true,
    }
}

fn condition_passes(
    entry: &ModelPreferenceEntry,
    request_task: ModelTask,
    ram_gb: f64,
    catalog: &ResourceCatalog,
    storage_root: &Path,
) -> Option<String> {
    let manifest = load_install_manifest(storage_root, &entry.model_id)?;
    if entry.when.require_installed && !Path::new(&manifest.gguf_path).is_file() {
        return None;
    }
    if entry.when.require_multimodal && manifest.modality != "multimodal" {
        return None;
    }
    if let Some(min) = entry.when.min_ram_gb {
        if ram_gb < min {
            return None;
        }
    }
    if entry.when.respect_ram_estimate {
        if let Some(need_mb) = catalog_ram_mb(catalog, &entry.model_id) {
            let need_gb = need_mb as f64 / 1024.0;
            if ram_gb < need_gb * 0.85 {
                return None;
            }
        }
    }
    if !task_matches(entry.when.task, request_task, catalog, &entry.model_id) {
        return None;
    }
    let reason = format!(
        "Priority #{} — {} ({})",
        entry.priority,
        entry.label,
        entry.when.task.label()
    );
    Some(reason)
}

pub fn resolve_preference(
    storage_root: &Path,
    catalog: &ResourceCatalog,
    prefs: &ModelPreferences,
    request_task: ModelTask,
) -> Option<ResolvedModelPreference> {
    if prefs.entries.is_empty() {
        return None;
    }
    let ram_gb = system_ram_gb();
    let mut entries: Vec<_> = prefs.entries.iter().collect();
    entries.sort_by_key(|e| e.priority);

    for entry in entries {
        let Some(reason) = condition_passes(entry, request_task, ram_gb, catalog, storage_root)
        else {
            continue;
        };
        let manifest = load_install_manifest(storage_root, &entry.model_id)?;
        return Some(ResolvedModelPreference {
            model_id: entry.model_id.clone(),
            label: entry.label.clone(),
            reason,
            gguf_path: manifest.gguf_path,
            priority: entry.priority,
            task: entry.when.task.as_str().to_string(),
        });
    }
    None
}

pub fn apply_preference(
    storage_root: &Path,
    catalog: &ResourceCatalog,
    prefs: &ModelPreferences,
    request_task: ModelTask,
) -> Result<ActiveModelRecord, String> {
    let resolved = resolve_preference(storage_root, catalog, prefs, request_task)
        .ok_or_else(|| "No installed model matches your priority rules".to_string())?;
    model_lifecycle::activate_model_for_id(&resolved.model_id, storage_root)
        .map_err(|e| e.to_string())
}

pub fn default_preferences_from_catalog(catalog: &ResourceCatalog) -> ModelPreferences {
    let mut llms: Vec<_> = catalog.llms.iter().collect();
    llms.sort_by_key(|m| m.ram_estimate_mb.or(m.size_mb).unwrap_or(u32::MAX));

    let entries: Vec<ModelPreferenceEntry> = llms
        .iter()
        .take(6)
        .enumerate()
        .map(|(i, m)| {
            let task = if m.is_multimodal() {
                ModelTask::Vision
            } else if m
                .tags
                .as_ref()
                .map(|t| t.iter().any(|tag| tag == "coding"))
                .unwrap_or(false)
            {
                ModelTask::Coding
            } else if m
                .recommended_for
                .as_ref()
                .map(|r| r.iter().any(|x| x == "edge" || x == "low_ram"))
                .unwrap_or(false)
            {
                ModelTask::LowRam
            } else {
                ModelTask::Chat
            };
            ModelPreferenceEntry {
                model_id: m.id.clone(),
                label: m.name.clone(),
                priority: (i as u32) + 1,
                when: ModelLoadCondition {
                    task,
                    ..ModelLoadCondition::default()
                },
            }
        })
        .collect();

    ModelPreferences {
        auto_select: true,
        entries,
    }
}

pub fn ensure_preferences(storage_root: &Path, catalog: &ResourceCatalog) -> ModelPreferences {
    let mut prefs = load_preferences(storage_root);
    if prefs.entries.is_empty() && !catalog.llms.is_empty() {
        prefs = default_preferences_from_catalog(catalog);
        let _ = save_preferences(storage_root, &prefs);
    }
    prefs
}

#[cfg(test)]
mod tests {
    use super::*;
    use qualia_core_db::resource_catalog::ResourceCatalog;

    #[test]
    fn default_preferences_sorted_by_ram() {
        let catalog = qualia_core_db::resource_catalog::load_default()
            .unwrap_or_else(|_| ResourceCatalog::empty());
        let prefs = default_preferences_from_catalog(&catalog);
        assert!(!prefs.entries.is_empty());
        assert!(prefs.auto_select);
        for (i, entry) in prefs.entries.iter().enumerate() {
            assert_eq!(entry.priority, (i as u32) + 1);
        }
    }

    #[test]
    fn task_from_str_maps_coding() {
        assert_eq!(ModelTask::from_str_lossy("coding"), ModelTask::Coding);
        assert_eq!(ModelTask::from_str_lossy("always"), ModelTask::Always);
    }
}
