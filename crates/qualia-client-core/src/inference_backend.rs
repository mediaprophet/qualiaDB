//! Persisted inference backend preference (Local / Remote / Hybrid / Ollama).
//!
//! **Local** (GGUF in-process) is the primary Qualia engine.
//! **Ollama** is an explicit opt-in harness for when native inference is not
//! ready or the principal wants a local Ollama endpoint for chat / ETL prep.

use serde::{Deserialize, Serialize};

use crate::chat_agents::AgentBackendKind;
use crate::state::app_meta_dir;

const SETTINGS_FILE: &str = "inference_backend.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InferenceBackendSettings {
    #[serde(default)]
    pub backend: AgentBackendKind,
    /// Remote (Nym / MCP) endpoint label or URL when backend is Remote/Hybrid.
    #[serde(default)]
    pub remote_endpoint: String,

    // ── Optional Ollama harness (ignored unless backend == Ollama) ──────────
    /// e.g. `http://127.0.0.1:11434`
    #[serde(default = "default_ollama_base_url")]
    pub ollama_base_url: String,
    /// Generation model tag (e.g. `llama3.2`, `qwen2.5:7b`).
    #[serde(default = "default_ollama_model")]
    pub ollama_model: String,
    /// Embedding model for ETL / retrieval prep.
    #[serde(default = "default_ollama_embed_model")]
    pub ollama_embed_model: String,
    /// Optional bearer token for hosted/proxied Ollama-compatible APIs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ollama_api_key: Option<String>,
    #[serde(default = "default_ollama_timeout")]
    pub ollama_timeout_secs: u64,
    #[serde(default = "default_ollama_num_ctx")]
    pub ollama_num_ctx: u32,
    #[serde(default = "default_ollama_temperature")]
    pub ollama_temperature: f32,
    /// When true and backend is Hybrid, try Ollama after local fails (not Remote).
    #[serde(default)]
    pub ollama_as_hybrid_fallback: bool,
}

fn default_ollama_base_url() -> String {
    crate::ollama_harness::DEFAULT_OLLAMA_BASE_URL.to_string()
}
fn default_ollama_model() -> String {
    "llama3.2".to_string()
}
fn default_ollama_embed_model() -> String {
    "nomic-embed-text".to_string()
}
fn default_ollama_timeout() -> u64 {
    120
}
fn default_ollama_num_ctx() -> u32 {
    8192
}
fn default_ollama_temperature() -> f32 {
    0.2
}

impl Default for InferenceBackendSettings {
    fn default() -> Self {
        Self {
            backend: AgentBackendKind::Local,
            remote_endpoint: String::new(),
            ollama_base_url: default_ollama_base_url(),
            ollama_model: default_ollama_model(),
            ollama_embed_model: default_ollama_embed_model(),
            ollama_api_key: None,
            ollama_timeout_secs: default_ollama_timeout(),
            ollama_num_ctx: default_ollama_num_ctx(),
            ollama_temperature: default_ollama_temperature(),
            ollama_as_hybrid_fallback: false,
        }
    }
}

impl InferenceBackendSettings {
    /// Mirror `AgentConfig.inference_backend` string into structured settings.
    pub fn apply_agent_config_backend_string(&mut self, s: &str) {
        self.backend = AgentBackendKind::from_str(s);
    }

    pub fn as_agent_config_string(&self) -> String {
        self.backend.as_str().to_string()
    }

    pub fn validate(&self) -> Result<(), String> {
        match self.backend {
            AgentBackendKind::Ollama => {
                if self.ollama_base_url.trim().is_empty() {
                    return Err("Ollama base URL is required when backend is Ollama".into());
                }
                if self.ollama_model.trim().is_empty() {
                    return Err("Ollama model tag is required when backend is Ollama".into());
                }
                if !(self.ollama_base_url.starts_with("http://")
                    || self.ollama_base_url.starts_with("https://"))
                {
                    return Err("Ollama base URL must start with http:// or https://".into());
                }
            }
            AgentBackendKind::Remote => {
                // remote_endpoint optional for now (MCP path may supply later)
            }
            _ => {}
        }
        Ok(())
    }
}

fn settings_path() -> std::path::PathBuf {
    app_meta_dir().join(SETTINGS_FILE)
}

pub fn load_inference_backend_settings() -> InferenceBackendSettings {
    let path = settings_path();
    if !path.is_file() {
        // Seed from AgentConfig.inference_backend if present.
        if let Some(state) = crate::state::APP_STATE.get() {
            if let Ok(cfg) = state.config.lock() {
                let mut s = InferenceBackendSettings::default();
                s.apply_agent_config_backend_string(&cfg.inference_backend);
                return s;
            }
        }
        return InferenceBackendSettings::default();
    }
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

pub fn save_inference_backend_settings(settings: &InferenceBackendSettings) -> Result<(), String> {
    settings.validate()?;
    let path = settings_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())?;

    // Keep AgentConfig.inference_backend in sync for legacy UI that only stores a string.
    if let Some(state) = crate::state::APP_STATE.get() {
        if let Ok(mut cfg) = state.config.lock() {
            cfg.inference_backend = settings.as_agent_config_string();
            if let Ok(json) = serde_json::to_string_pretty(&*cfg) {
                let _ = std::fs::write(crate::state::config_file_path(), json);
            }
        }
    }
    Ok(())
}

pub fn backend_label(settings: &InferenceBackendSettings) -> &'static str {
    match settings.backend {
        AgentBackendKind::Local => "Local GGUF (in-process Qualia)",
        AgentBackendKind::Remote => "Remote (Nym mixnet, consent required)",
        AgentBackendKind::Hybrid => "Hybrid (local first, remote/ollama fallback)",
        AgentBackendKind::Ollama => "Ollama (optional HTTP harness)",
    }
}

/// True when chat should use the Ollama harness instead of LocalLlmAgent.
pub fn use_ollama_harness() -> bool {
    matches!(
        load_inference_backend_settings().backend,
        AgentBackendKind::Ollama
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_local() {
        let settings = InferenceBackendSettings::default();
        assert_eq!(settings.backend, AgentBackendKind::Local);
        assert!(!use_ollama_harness() || settings.backend == AgentBackendKind::Ollama);
    }

    #[test]
    fn ollama_validate_requires_url_and_model() {
        let mut s = InferenceBackendSettings {
            backend: AgentBackendKind::Ollama,
            ollama_base_url: String::new(),
            ollama_model: String::new(),
            ..Default::default()
        };
        assert!(s.validate().is_err());
        s.ollama_base_url = "http://127.0.0.1:11434".into();
        s.ollama_model = "llama3.2".into();
        assert!(s.validate().is_ok());
    }

    #[test]
    fn from_str_roundtrip_ollama() {
        assert_eq!(
            AgentBackendKind::from_str("ollama"),
            AgentBackendKind::Ollama
        );
        assert_eq!(AgentBackendKind::Ollama.as_str(), "ollama");
    }
}
