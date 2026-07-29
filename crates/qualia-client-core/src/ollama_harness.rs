//! Optional Ollama HTTP harness for inference when the native Qualia GGUF path
//! is unavailable or the principal explicitly selects Ollama.
//!
//! **Not the primary engine.** Qualia remains Local GGUF / wgpu in-process.
//! This harness is opt-in so chat, ETL scaffolding, and later CML/logic gates
//! can still run against a reachable Ollama (or OpenAI-compatible) endpoint
//! while native inference is brought up.
//!
//! Wire format: Ollama REST (`/api/tags`, `/api/generate`, `/api/chat`).
//! Network I/O is confined to this module; callers pass prompts already
//! augmented by Qualia retrieval / ontology routing.

use serde::{Deserialize, Serialize};

use crate::inference_backend::InferenceBackendSettings;

/// Default local Ollama base URL (override via settings or `OLLAMA_HOST`).
pub const DEFAULT_OLLAMA_BASE_URL: &str = "http://127.0.0.1:11434";

/// Operator-visible probe of a configured Ollama endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OllamaStatus {
    pub reachable: bool,
    pub base_url: String,
    pub version_hint: Option<String>,
    pub models: Vec<OllamaModelInfo>,
    pub error: Option<String>,
    pub latency_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OllamaModelInfo {
    pub name: String,
    pub size_bytes: Option<u64>,
    pub parameter_size: Option<String>,
    pub quantization: Option<String>,
    pub family: Option<String>,
}

/// One completed generation (non-streaming).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaGenerateResult {
    pub text: String,
    pub model: String,
    pub total_duration_ns: Option<u64>,
    pub eval_count: Option<u32>,
    pub prompt_eval_count: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct OllamaHarness {
    pub base_url: String,
    pub gen_model: String,
    pub embed_model: String,
    pub api_key: Option<String>,
    pub num_ctx: u32,
    pub timeout_secs: u64,
    pub temperature: f32,
}

impl Default for OllamaHarness {
    fn default() -> Self {
        Self::from_settings(&InferenceBackendSettings::default())
    }
}

impl OllamaHarness {
    pub fn from_settings(settings: &InferenceBackendSettings) -> Self {
        let base = settings
            .ollama_base_url
            .trim()
            .trim_end_matches('/')
            .to_string();
        let base = if base.is_empty() {
            std::env::var("OLLAMA_HOST").unwrap_or_else(|_| DEFAULT_OLLAMA_BASE_URL.to_string())
        } else {
            base
        };
        let gen = if settings.ollama_model.trim().is_empty() {
            std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "llama3.2".to_string())
        } else {
            settings.ollama_model.trim().to_string()
        };
        let embed = if settings.ollama_embed_model.trim().is_empty() {
            "nomic-embed-text".to_string()
        } else {
            settings.ollama_embed_model.trim().to_string()
        };
        Self {
            base_url: base.trim_end_matches('/').to_string(),
            gen_model: gen,
            embed_model: embed,
            api_key: settings
                .ollama_api_key
                .as_ref()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty()),
            num_ctx: settings.ollama_num_ctx.max(512),
            timeout_secs: settings.ollama_timeout_secs.clamp(5, 3600),
            temperature: settings.ollama_temperature.clamp(0.0, 2.0),
        }
    }

    pub fn from_loaded_settings() -> Self {
        Self::from_settings(&crate::inference_backend::load_inference_backend_settings())
    }

    fn url(&self, path: &str) -> String {
        format!(
            "{}{}",
            self.base_url.trim_end_matches('/'),
            if path.starts_with('/') {
                path.to_string()
            } else {
                format!("/{path}")
            }
        )
    }

    fn apply_auth(
        &self,
        mut req: reqwest::blocking::RequestBuilder,
    ) -> reqwest::blocking::RequestBuilder {
        if let Some(key) = &self.api_key {
            req = req.bearer_auth(key);
        }
        req
    }

    fn apply_auth_async(&self, mut req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Some(key) = &self.api_key {
            req = req.bearer_auth(key);
        }
        req
    }

    fn blocking_client(&self) -> Result<reqwest::blocking::Client, String> {
        reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(self.timeout_secs))
            .build()
            .map_err(|e| format!("ollama client: {e}"))
    }

    async fn async_client(&self) -> Result<reqwest::Client, String> {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(self.timeout_secs))
            .build()
            .map_err(|e| format!("ollama client: {e}"))
    }

    /// GET `/api/tags` — reachability + model list for the options page.
    pub fn probe_status(&self) -> OllamaStatus {
        let t0 = std::time::Instant::now();
        let client = match self.blocking_client() {
            Ok(c) => c,
            Err(e) => {
                return OllamaStatus {
                    reachable: false,
                    base_url: self.base_url.clone(),
                    version_hint: None,
                    models: vec![],
                    error: Some(e),
                    latency_ms: 0,
                };
            }
        };
        let req = self.apply_auth(client.get(self.url("/api/tags")));
        match req.send() {
            Ok(resp) if resp.status().is_success() => {
                let latency_ms = t0.elapsed().as_millis() as u64;
                match resp.json::<TagsResponse>() {
                    Ok(tags) => OllamaStatus {
                        reachable: true,
                        base_url: self.base_url.clone(),
                        version_hint: Some("ollama".into()),
                        models: tags
                            .models
                            .into_iter()
                            .map(|m| OllamaModelInfo {
                                name: m.name,
                                size_bytes: m.size,
                                parameter_size: m
                                    .details
                                    .as_ref()
                                    .and_then(|d| d.parameter_size.clone()),
                                quantization: m
                                    .details
                                    .as_ref()
                                    .and_then(|d| d.quantization_level.clone()),
                                family: m.details.as_ref().and_then(|d| d.family.clone()),
                            })
                            .collect(),
                        error: None,
                        latency_ms,
                    },
                    Err(e) => OllamaStatus {
                        reachable: true,
                        base_url: self.base_url.clone(),
                        version_hint: None,
                        models: vec![],
                        error: Some(format!("parse /api/tags: {e}")),
                        latency_ms,
                    },
                }
            }
            Ok(resp) => OllamaStatus {
                reachable: false,
                base_url: self.base_url.clone(),
                version_hint: None,
                models: vec![],
                error: Some(format!("HTTP {}", resp.status())),
                latency_ms: t0.elapsed().as_millis() as u64,
            },
            Err(e) => OllamaStatus {
                reachable: false,
                base_url: self.base_url.clone(),
                version_hint: None,
                models: vec![],
                error: Some(e.to_string()),
                latency_ms: t0.elapsed().as_millis() as u64,
            },
        }
    }

    /// Async variant of [`Self::probe_status`] for Tauri commands.
    pub async fn probe_status_async(&self) -> OllamaStatus {
        let t0 = std::time::Instant::now();
        let client = match self.async_client().await {
            Ok(c) => c,
            Err(e) => {
                return OllamaStatus {
                    reachable: false,
                    base_url: self.base_url.clone(),
                    version_hint: None,
                    models: vec![],
                    error: Some(e),
                    latency_ms: 0,
                };
            }
        };
        let req = self.apply_auth_async(client.get(self.url("/api/tags")));
        match req.send().await {
            Ok(resp) if resp.status().is_success() => {
                let latency_ms = t0.elapsed().as_millis() as u64;
                match resp.json::<TagsResponse>().await {
                    Ok(tags) => OllamaStatus {
                        reachable: true,
                        base_url: self.base_url.clone(),
                        version_hint: Some("ollama".into()),
                        models: tags
                            .models
                            .into_iter()
                            .map(|m| OllamaModelInfo {
                                name: m.name,
                                size_bytes: m.size,
                                parameter_size: m
                                    .details
                                    .as_ref()
                                    .and_then(|d| d.parameter_size.clone()),
                                quantization: m
                                    .details
                                    .as_ref()
                                    .and_then(|d| d.quantization_level.clone()),
                                family: m.details.as_ref().and_then(|d| d.family.clone()),
                            })
                            .collect(),
                        error: None,
                        latency_ms,
                    },
                    Err(e) => OllamaStatus {
                        reachable: true,
                        base_url: self.base_url.clone(),
                        version_hint: None,
                        models: vec![],
                        error: Some(format!("parse /api/tags: {e}")),
                        latency_ms,
                    },
                }
            }
            Ok(resp) => OllamaStatus {
                reachable: false,
                base_url: self.base_url.clone(),
                version_hint: None,
                models: vec![],
                error: Some(format!("HTTP {}", resp.status())),
                latency_ms: t0.elapsed().as_millis() as u64,
            },
            Err(e) => OllamaStatus {
                reachable: false,
                base_url: self.base_url.clone(),
                version_hint: None,
                models: vec![],
                error: Some(e.to_string()),
                latency_ms: t0.elapsed().as_millis() as u64,
            },
        }
    }

    /// POST `/api/generate` — single-shot completion (blocking; chat/ETL cold path).
    pub fn generate(&self, system: &str, prompt: &str) -> Result<OllamaGenerateResult, String> {
        let client = self.blocking_client()?;
        let body = serde_json::json!({
            "model": self.gen_model,
            "system": system,
            "prompt": prompt,
            "stream": false,
            "options": {
                "temperature": self.temperature,
                "num_ctx": self.num_ctx,
            }
        });
        let req = self.apply_auth(client.post(self.url("/api/generate")).json(&body));
        let resp = req.send().map_err(|e| format!("ollama generate: {e}"))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            return Err(format!("ollama generate HTTP {status}: {body}"));
        }
        let parsed: GenerateResponse = resp
            .json()
            .map_err(|e| format!("ollama generate parse: {e}"))?;
        Ok(OllamaGenerateResult {
            text: parsed.response,
            model: parsed.model.unwrap_or_else(|| self.gen_model.clone()),
            total_duration_ns: parsed.total_duration,
            eval_count: parsed.eval_count,
            prompt_eval_count: parsed.prompt_eval_count,
        })
    }

    /// Async generate for desktop/Tauri.
    pub async fn generate_async(
        &self,
        system: &str,
        prompt: &str,
    ) -> Result<OllamaGenerateResult, String> {
        let client = self.async_client().await?;
        let body = serde_json::json!({
            "model": self.gen_model,
            "system": system,
            "prompt": prompt,
            "stream": false,
            "options": {
                "temperature": self.temperature,
                "num_ctx": self.num_ctx,
            }
        });
        let req = self.apply_auth_async(client.post(self.url("/api/generate")).json(&body));
        let resp = req
            .send()
            .await
            .map_err(|e| format!("ollama generate: {e}"))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("ollama generate HTTP {status}: {body}"));
        }
        let parsed: GenerateResponse = resp
            .json()
            .await
            .map_err(|e| format!("ollama generate parse: {e}"))?;
        Ok(OllamaGenerateResult {
            text: parsed.response,
            model: parsed.model.unwrap_or_else(|| self.gen_model.clone()),
            total_duration_ns: parsed.total_duration,
            eval_count: parsed.eval_count,
            prompt_eval_count: parsed.prompt_eval_count,
        })
    }

    /// POST `/api/chat` multi-turn (system + user messages).
    pub fn chat(&self, system: &str, user: &str) -> Result<OllamaGenerateResult, String> {
        let client = self.blocking_client()?;
        let body = serde_json::json!({
            "model": self.gen_model,
            "stream": false,
            "messages": [
                { "role": "system", "content": system },
                { "role": "user", "content": user },
            ],
            "options": {
                "temperature": self.temperature,
                "num_ctx": self.num_ctx,
            }
        });
        let req = self.apply_auth(client.post(self.url("/api/chat")).json(&body));
        let resp = req.send().map_err(|e| format!("ollama chat: {e}"))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            return Err(format!("ollama chat HTTP {status}: {body}"));
        }
        let parsed: ChatResponse = resp.json().map_err(|e| format!("ollama chat parse: {e}"))?;
        Ok(OllamaGenerateResult {
            text: parsed.message.map(|m| m.content).unwrap_or_default(),
            model: parsed.model.unwrap_or_else(|| self.gen_model.clone()),
            total_duration_ns: parsed.total_duration,
            eval_count: parsed.eval_count,
            prompt_eval_count: parsed.prompt_eval_count,
        })
    }

    /// Embedding vectors via `/api/embeddings` (ETL / retrieval prep).
    pub fn embed(&self, text: &str) -> Result<Vec<f32>, String> {
        let client = self.blocking_client()?;
        let body = serde_json::json!({
            "model": self.embed_model,
            "prompt": text,
        });
        let req = self.apply_auth(client.post(self.url("/api/embeddings")).json(&body));
        let resp = req.send().map_err(|e| format!("ollama embed: {e}"))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            return Err(format!("ollama embed HTTP {status}: {body}"));
        }
        let parsed: EmbeddingResponse = resp
            .json()
            .map_err(|e| format!("ollama embed parse: {e}"))?;
        Ok(parsed.embedding)
    }
}

/// Convenience: probe using currently persisted settings.
pub fn probe_configured_ollama() -> OllamaStatus {
    OllamaHarness::from_loaded_settings().probe_status()
}

pub async fn probe_configured_ollama_async() -> OllamaStatus {
    OllamaHarness::from_loaded_settings()
        .probe_status_async()
        .await
}

// ── Wire types ────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct TagsResponse {
    #[serde(default)]
    models: Vec<TagModel>,
}

#[derive(Debug, Deserialize)]
struct TagModel {
    name: String,
    #[serde(default)]
    size: Option<u64>,
    #[serde(default)]
    details: Option<TagDetails>,
}

#[derive(Debug, Deserialize)]
struct TagDetails {
    #[serde(default)]
    parameter_size: Option<String>,
    #[serde(default)]
    quantization_level: Option<String>,
    #[serde(default)]
    family: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GenerateResponse {
    response: String,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    total_duration: Option<u64>,
    #[serde(default)]
    eval_count: Option<u32>,
    #[serde(default)]
    prompt_eval_count: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    #[serde(default)]
    message: Option<ChatMessage>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    total_duration: Option<u64>,
    #[serde(default)]
    eval_count: Option<u32>,
    #[serde(default)]
    prompt_eval_count: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct ChatMessage {
    #[serde(default)]
    content: String,
}

#[derive(Debug, Deserialize)]
struct EmbeddingResponse {
    embedding: Vec<f32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chat_agents::AgentBackendKind;
    use crate::inference_backend::InferenceBackendSettings;

    #[test]
    fn harness_from_settings_defaults() {
        let s = InferenceBackendSettings {
            backend: AgentBackendKind::Ollama,
            ollama_base_url: "http://127.0.0.1:11434/".into(),
            ollama_model: "qwen2.5:7b".into(),
            ollama_embed_model: "nomic-embed-text".into(),
            ollama_timeout_secs: 120,
            ollama_num_ctx: 8192,
            ollama_temperature: 0.2,
            ..Default::default()
        };
        let h = OllamaHarness::from_settings(&s);
        assert_eq!(h.base_url, "http://127.0.0.1:11434");
        assert_eq!(h.gen_model, "qwen2.5:7b");
        assert_eq!(h.num_ctx, 8192);
    }

    #[test]
    fn unreachable_probe_is_fail_closed() {
        let h = OllamaHarness {
            base_url: "http://127.0.0.1:1".into(),
            gen_model: "x".into(),
            embed_model: "y".into(),
            api_key: None,
            num_ctx: 2048,
            timeout_secs: 1,
            temperature: 0.1,
        };
        let st = h.probe_status();
        assert!(!st.reachable);
        assert!(st.error.is_some());
    }
}
