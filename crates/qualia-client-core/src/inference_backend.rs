//! Persisted inference backend preference (Local / Remote / Hybrid).

use serde::{Deserialize, Serialize};

use crate::chat_agents::AgentBackendKind;
use crate::state::app_meta_dir;

const SETTINGS_FILE: &str = "inference_backend.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InferenceBackendSettings {
    #[serde(default)]
    pub backend: AgentBackendKind,
    #[serde(default)]
    pub remote_endpoint: String,
}

impl Default for InferenceBackendSettings {
    fn default() -> Self {
        Self {
            backend: AgentBackendKind::Local,
            remote_endpoint: String::new(),
        }
    }
}

fn settings_path() -> std::path::PathBuf {
    app_meta_dir().join(SETTINGS_FILE)
}

pub fn load_inference_backend_settings() -> InferenceBackendSettings {
    let path = settings_path();
    if !path.is_file() {
        return InferenceBackendSettings::default();
    }
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

pub fn save_inference_backend_settings(settings: &InferenceBackendSettings) -> Result<(), String> {
    let path = settings_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}

pub fn backend_label(settings: &InferenceBackendSettings) -> &'static str {
    match settings.backend {
        AgentBackendKind::Local => "Local GGUF (in-process)",
        AgentBackendKind::Remote => "Remote (Nym mixnet, consent required)",
        AgentBackendKind::Hybrid => "Hybrid (local first, remote fallback)",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_local() {
        let settings = InferenceBackendSettings::default();
        assert_eq!(settings.backend, AgentBackendKind::Local);
    }
}
