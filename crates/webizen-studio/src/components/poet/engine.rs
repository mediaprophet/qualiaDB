//! Host bridge — WASM talks to Tauri; native studio can call the same commands
//! when compiled inside Webizen Desktop.

use crate::components::settings::host::invoke_json;
use serde::Deserialize;
use serde_json::json;

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

#[derive(Clone, Deserialize, Default)]
pub struct GazetteerHitDto {
    pub surface: String,
    pub iri: String,
    pub kind: String,
}

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
