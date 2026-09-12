//! Host bridge — Catalog Open pack is daemon-first (`:4242/invoke`).
//! Tauri / in-process binds are fallbacks. No Host widen.

use super::graph_daemon::{
    daemon_http_probe, daemon_lexicon_manifest, daemon_lexicon_manifest_wait,
};
use crate::components::settings::host::invoke_json;
use serde::Deserialize;
use serde_json::json;
use webizen_studio::lexicon_catalog::{resolve_lexicon_path, HELD_WHY};

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

/// Live ALL_BOUND bind — daemon-first `:4242/invoke`, then Desktop host,
/// then in-process `GraphDatabase.lexicon_manifest`. Never "unavailable".
pub async fn lexicon_manifest(path: String) -> Result<PoetEvalResult, String> {
    let resolved = resolve_lexicon_path(path.trim());
    if let Ok(from_daemon) = daemon_lexicon_manifest(&resolved).await {
        return Ok(from_daemon);
    }
    match invoke_json("poet_lexicon_manifest", json!({ "path": resolved.clone() })).await {
        Ok(result) => Ok(result),
        Err(err) => {
            if let Ok(from_daemon) = daemon_lexicon_manifest_wait(&resolved).await {
                return Ok(from_daemon);
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = err;
                return Ok(in_process_lexicon_manifest(&resolved));
            }
            #[cfg(target_arch = "wasm32")]
            {
                let folded = err.to_ascii_lowercase();
                if folded.contains("unavailable") || folded.contains("broken") {
                    Err(HELD_WHY.to_string())
                } else {
                    Err(err)
                }
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn in_process_lexicon_manifest(path: &str) -> PoetEvalResult {
    use qualia_core_db::poet_host::{format_value, PoetSnapshot};
    use vibe::Value;
    let mut snap = PoetSnapshot::from_daemon();
    let mut rec = std::collections::BTreeMap::new();
    rec.insert("path".into(), Value::String(path.into()));
    match snap.invoke_id(
        webizen_studio::lexicon_catalog::INVOKE_ID,
        Value::Record(rec),
    ) {
        Ok(v) => PoetEvalResult {
            ok: true,
            value: format_value(&v),
            diagnostic: None,
            revision: snap.revision,
            committed: snap.visible_count(),
            honesty: "live".into(),
            language: vibe::LANGUAGE_VERSION.to_string(),
            value_cbor_hex: String::new(),
        },
        Err(e) => PoetEvalResult {
            ok: false,
            value: String::new(),
            diagnostic: Some(e.to_json()),
            revision: snap.revision,
            committed: snap.visible_count(),
            honesty: "held".into(),
            language: vibe::LANGUAGE_VERSION.to_string(),
            value_cbor_hex: String::new(),
        },
    }
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
    if let Some(probe) = daemon_http_probe().await {
        return Ok(probe);
    }
    match invoke_json("poet_daemon_probe", json!({})).await {
        Ok(probe) => Ok(probe),
        Err(err) => {
            if let Some(probe) = daemon_http_probe().await {
                return Ok(probe);
            }
            let folded = err.to_ascii_lowercase();
            if folded.contains("unavailable") || folded.contains("broken") {
                Err(HELD_WHY.to_string())
            } else {
                Err(err)
            }
        }
    }
}
