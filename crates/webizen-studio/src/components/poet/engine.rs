//! Host bridge — WASM talks to Tauri; native studio can call the same commands
//! when compiled inside Webizen Desktop.

use crate::components::settings::host::invoke_json;
use serde::Deserialize;
use serde_json::json;

#[allow(dead_code)]
#[derive(Clone, Deserialize, Default)]
pub struct PoetEvalResult {
    pub ok: bool,
    pub value: String,
    pub diagnostic: Option<String>,
    #[serde(default)]
    pub revision: u64,
    #[serde(default)]
    pub committed: usize,
    #[serde(default)]
    pub honesty: String,
    #[serde(default)]
    pub language: String,
    /// CBOR diagnostic of the result when the host supplied one (hex). Not JSON.
    #[serde(default)]
    pub value_cbor_hex: String,
}

#[allow(dead_code)]
#[derive(Clone, Deserialize, Default)]
pub struct PoetGazetteerResult {
    #[serde(default)]
    pub token_count: usize,
    #[serde(default)]
    pub sentence_count: usize,
    #[serde(default)]
    pub sealed: usize,
    #[serde(default)]
    pub hits: Vec<GazetteerHitDto>,
}

#[allow(dead_code)]
#[derive(Clone, Deserialize, Default)]
pub struct GazetteerHitDto {
    pub surface: String,
    pub iri: String,
    pub kind: String,
}

#[allow(dead_code)]
#[derive(Clone, Deserialize, Default)]
pub struct PoetRenderResult {
    #[serde(default)]
    pub ok: bool,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub honesty: String,
    #[serde(default)]
    pub node_count: usize,
    #[serde(default)]
    pub edge_count: usize,
    #[serde(default)]
    pub face_count: usize,
    pub data_uri: Option<String>,
    pub diagnostic: Option<String>,
}

pub async fn eval(
    source: String,
    as_cell: bool,
    function: Option<String>,
) -> Result<PoetEvalResult, String> {
    invoke_json(
        "poet_eval",
        json!({ "source": source, "as_cell": as_cell, "function": function }),
    )
    .await
}

pub async fn gazetteer(source: String) -> Result<PoetGazetteerResult, String> {
    invoke_json("poet_gazetteer", json!({ "source": source })).await
}

pub async fn render_preview(
    kind: String,
    width: u32,
    height: u32,
) -> Result<PoetRenderResult, String> {
    invoke_json(
        "poet_render_preview",
        json!({ "kind": kind, "width": width, "height": height }),
    )
    .await
}

/// Live ALL_BOUND bind — same id WASM Catalog uses. No Host widen.
///
/// Prefer HTTP `POST {daemon}/invoke` when Native Connected (Capt curl path).
/// Tauri `poet_lexicon_manifest` stays a fallback — Desktop Catalog must not
/// strand on in-process held while `:4242` already gates open.
pub async fn lexicon_manifest(path: String) -> Result<PoetEvalResult, String> {
    let mut bases = Vec::new();
    if let Ok(probe) = daemon_probe().await {
        if probe.reachable && !probe.url.is_empty() {
            bases.push(probe.url);
        }
    }
    bases.push("http://127.0.0.1:4242".into());
    for base in bases {
        if let Ok(http) = http_lexicon_manifest(&base, &path).await {
            return Ok(http);
        }
    }
    invoke_json("poet_lexicon_manifest", json!({ "path": path })).await
}

fn coerce_invoke_value(v: &serde_json::Value) -> String {
    match v.get("value") {
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
        None => String::new(),
    }
}

async fn http_lexicon_manifest(base: &str, path: &str) -> Result<PoetEvalResult, String> {
    let url = format!("{}/invoke", base.trim_end_matches('/'));
    let body = json!({
        "id": "GraphDatabase.lexicon_manifest",
        "args": { "path": path },
    });
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .map_err(|e| e.to_string())?;
    let res = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        return Err(format!("invoke HTTP {}", res.status()));
    }
    let v: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
    Ok(PoetEvalResult {
        ok: v.get("ok").and_then(|x| x.as_bool()).unwrap_or(false),
        value: coerce_invoke_value(&v),
        diagnostic: v
            .get("diagnostic")
            .and_then(|x| x.as_str())
            .map(str::to_string),
        revision: v.get("revision").and_then(|x| x.as_u64()).unwrap_or(0),
        committed: v.get("committed").and_then(|x| x.as_u64()).unwrap_or(0) as usize,
        honesty: v
            .get("honesty")
            .and_then(|x| x.as_str())
            .unwrap_or(if v.get("ok").and_then(|x| x.as_bool()).unwrap_or(false) {
                "live"
            } else {
                "held"
            })
            .to_string(),
        language: String::new(),
        value_cbor_hex: String::new(),
    })
}

/// Live ALL_BOUND bind. Reopen defaults `create: false` so a missing volume stays held.
pub async fn volume_open(path: String, create: bool) -> Result<PoetEvalResult, String> {
    invoke_json(
        "poet_volume_open",
        json!({ "path": path, "create": create }),
    )
    .await
}

/// Live ALL_BOUND bind. Callers must celebrate only when `written > 0`.
pub async fn volume_commit(path: String) -> Result<PoetEvalResult, String> {
    invoke_json("poet_volume_commit", json!({ "path": path })).await
}

pub async fn browse_q42_volume() -> Result<Option<String>, String> {
    invoke_json("open_q42_file_picker", json!({})).await
}

pub async fn start_q42_volume() -> Result<Option<String>, String> {
    invoke_json("save_q42_file_picker", json!({})).await
}

/// HTTP `/health` on local QualiaDB (`127.0.0.1:4242`). Same probe Poet WASM uses.
#[derive(Clone, Deserialize, Default)]
pub struct DaemonProbe {
    #[serde(default)]
    pub reachable: bool,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub port: u16,
    pub engine: Option<String>,
    pub version: Option<String>,
    pub graph_quin_count: Option<usize>,
    #[serde(default)]
    pub honesty: String,
    #[serde(default)]
    pub label: String,
}

pub async fn daemon_probe() -> Result<DaemonProbe, String> {
    invoke_json("poet_daemon_probe", json!({})).await
}
