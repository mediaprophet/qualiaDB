//! Ollama HTTP backend — an external LLM reached over its REST API.
//!
//! Uses blocking `reqwest` (this is a batch CLI tool, not an async service).
//! `base_url`, model names and an optional bearer token are all configurable so
//! the same backend also targets a hosted Ollama or a compatible proxy.

use serde::Deserialize;

use super::LlmBackend;

/// Configuration for the Ollama backend.
#[derive(Debug, Clone)]
pub struct OllamaConfig {
    /// e.g. `http://127.0.0.1:11434`.
    pub base_url: String,
    /// Embedding model (e.g. `nomic-embed-text`).
    pub embed_model: String,
    /// Generation model (e.g. `gemma4:e4b`).
    pub gen_model: String,
    /// Optional bearer token for hosted/proxied endpoints.
    pub api_key: Option<String>,
    /// Context window for generation.
    pub num_ctx: u32,
    pub timeout_secs: u64,
}

impl Default for OllamaConfig {
    fn default() -> Self {
        OllamaConfig {
            base_url: std::env::var("OLLAMA_HOST")
                .unwrap_or_else(|_| "http://127.0.0.1:11434".into()),
            // Qwen3-Embedding (June 2025) tops the MTEB retrieval leaderboards;
            // the 0.6B variant has 32k context and 1024-dim output — the best
            // quality-per-size for a large technical corpus. Override via
            // OllamaConfig / --embed-model.
            embed_model: "qwen3-embedding:0.6b".into(),
            gen_model: "gemma4:e4b".into(),
            api_key: std::env::var("QSL_LLM_API_KEY").ok(),
            num_ctx: 16384,
            timeout_secs: 600,
        }
    }
}

pub struct OllamaBackend {
    cfg: OllamaConfig,
    client: reqwest::blocking::Client,
    embed_dim: std::cell::Cell<u32>,
}

impl OllamaBackend {
    pub fn new(cfg: OllamaConfig) -> anyhow::Result<Self> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(cfg.timeout_secs))
            .build()?;
        Ok(OllamaBackend {
            cfg,
            client,
            embed_dim: std::cell::Cell::new(0),
        })
    }

    fn post_json(
        &self,
        path: &str,
        body: serde_json::Value,
    ) -> anyhow::Result<reqwest::blocking::Response> {
        let mut req = self
            .client
            .post(format!(
                "{}{}",
                self.cfg.base_url.trim_end_matches('/'),
                path
            ))
            .json(&body);
        if let Some(key) = &self.cfg.api_key {
            req = req.bearer_auth(key);
        }
        let resp = req.send()?;
        if !resp.status().is_success() {
            anyhow::bail!("ollama {} -> HTTP {}", path, resp.status());
        }
        Ok(resp)
    }
}

#[derive(Deserialize)]
struct EmbeddingResp {
    embedding: Vec<f32>,
}

#[derive(Deserialize)]
struct GenerateResp {
    response: String,
}

impl LlmBackend for OllamaBackend {
    fn embed(&self, texts: &[String]) -> anyhow::Result<Vec<Vec<f32>>> {
        let mut out = Vec::with_capacity(texts.len());
        // /api/embeddings is one prompt per call — reliable across versions.
        for t in texts {
            let body = serde_json::json!({ "model": self.cfg.embed_model, "prompt": t });
            let resp = self.post_json("/api/embeddings", body)?;
            let parsed: EmbeddingResp = resp.json()?;
            if self.embed_dim.get() == 0 {
                self.embed_dim.set(parsed.embedding.len() as u32);
            }
            out.push(parsed.embedding);
        }
        Ok(out)
    }

    fn generate(&self, system: &str, prompt: &str) -> anyhow::Result<String> {
        let body = serde_json::json!({
            "model": self.cfg.gen_model,
            "system": system,
            "prompt": prompt,
            "stream": false,
            "options": { "temperature": 0.1, "num_ctx": self.cfg.num_ctx }
        });
        let resp = self.post_json("/api/generate", body)?;
        let parsed: GenerateResp = resp.json()?;
        Ok(parsed.response)
    }

    fn embed_dim(&self) -> u32 {
        self.embed_dim.get()
    }

    fn model_id(&self) -> String {
        format!("ollama/{}", self.cfg.embed_model)
    }
}
