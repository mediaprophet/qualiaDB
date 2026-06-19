//! QApp ↔ QualiaDB analysis contract.
//!
//! This is the bridge the discipline QApps were missing: a single, typed request/
//! response shape plus one `analyze` entry point that resolves to a **real**
//! QualiaDB call when running inside the Tauri desktop app, and to a deterministic
//! **stub** in the plain-browser demo (so the demo stays fully interactive without
//! a daemon).
//!
//! Wire a discipline up by rendering [`EnginePanel`] at the bottom of its QApp and
//! passing the current field values. The native side is the Tauri command
//! `qapp_analyze` (see `webizen-desktop/src/commands`).

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

/// A discipline analysis request: the field selections plus free-text notes.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AnalysisRequest {
    pub discipline: String,
    pub fields: Vec<(String, String)>,
    pub notes: String,
}

/// The engine's response: a short summary, derived assertions, a provenance hash,
/// and which engine produced it (`qualia-core-db` or `stub`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct AnalysisResult {
    pub summary: String,
    pub assertions: Vec<String>,
    pub provenance_hash: String,
    pub engine: String,
}

/// Resolve an analysis request to the best available engine.
///
/// - Tauri desktop webview → real `qapp_analyze` Tauri command (QualiaDB).
/// - Plain browser / native fallback → deterministic [`stub_analyze`].
pub async fn analyze(req: AnalysisRequest) -> Result<AnalysisResult, String> {
    #[cfg(target_arch = "wasm32")]
    {
        if crate::endpoints::is_native_host() {
            return invoke_native(req).await;
        }
        Ok(stub_analyze(&req))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        Ok(stub_analyze(&req))
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke, catch)]
    async fn tauri_invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
}

#[cfg(target_arch = "wasm32")]
async fn invoke_native(req: AnalysisRequest) -> Result<AnalysisResult, String> {
    let args = serde_wasm_bindgen::to_value(&serde_json::json!({ "request": req }))
        .map_err(|e| e.to_string())?;
    let value = tauri_invoke("qapp_analyze", args)
        .await
        .map_err(|e| format!("qapp_analyze failed: {e:?}"))?;
    serde_wasm_bindgen::from_value(value).map_err(|e| e.to_string())
}

/// Deterministic offline analysis used by the web demo. Produces a stable
/// provenance hash and a few plausible assertions from the supplied fields so the
/// UI behaves identically (minus real graph persistence) to the native path.
pub fn stub_analyze(req: &AnalysisRequest) -> AnalysisResult {
    let mut assertions: Vec<String> = req
        .fields
        .iter()
        .filter(|(_, v)| !v.trim().is_empty())
        .map(|(k, v)| format!("{} :{} \"{}\" .", req.discipline, slug(k), v))
        .collect();
    if !req.notes.trim().is_empty() {
        assertions.push(format!(
            "{} :hasNote \"{}\" .",
            req.discipline,
            truncate(req.notes.trim(), 80)
        ));
    }

    let hash = provenance_hash(req);
    AnalysisResult {
        summary: format!(
            "{} analysis assembled {} assertion(s) for the epistemic graph (offline preview).",
            req.discipline,
            assertions.len()
        ),
        assertions,
        provenance_hash: format!("q42:{hash:016x}"),
        engine: "stub".to_string(),
    }
}

/// FNV-1a 64-bit over the canonical request encoding — stable across runs and
/// platforms, mirroring how the native side derives a `q_hash` provenance stamp.
fn provenance_hash(req: &AnalysisRequest) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    let mut feed = |s: &str| {
        for b in s.as_bytes() {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h ^= 0xff;
        h = h.wrapping_mul(0x100000001b3);
    };
    feed(&req.discipline);
    for (k, v) in &req.fields {
        feed(k);
        feed(v);
    }
    feed(&req.notes);
    h
}

fn slug(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if !out.ends_with('_') {
            out.push('_');
        }
    }
    out.trim_matches('_').to_string()
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut t: String = s.chars().take(max).collect();
        t.push('…');
        t
    }
}

/// Reusable "Commit to QualiaDB" panel. Drop into any discipline QApp, passing the
/// live field values; it calls [`analyze`] and renders the engine's response.
#[component]
pub fn EnginePanel(discipline: String, fields: Vec<(String, String)>, notes: String) -> Element {
    let mut result = use_signal(|| None::<AnalysisResult>);
    let mut error = use_signal(|| None::<String>);
    let mut busy = use_signal(|| false);

    let on_run = move |_| {
        if busy() {
            return;
        }
        let req = AnalysisRequest {
            discipline: discipline.clone(),
            fields: fields.clone(),
            notes: notes.clone(),
        };
        busy.set(true);
        error.set(None);
        spawn(async move {
            match analyze(req).await {
                Ok(res) => {
                    result.set(Some(res));
                }
                Err(e) => {
                    error.set(Some(e));
                    result.set(None);
                }
            }
            busy.set(false);
        });
    };

    rsx! {
        div {
            style: "margin-top: 12px; padding: 12px 16px; border-radius: 8px; border: 1px solid var(--qualia-border); background: var(--qualia-surface); display: flex; flex-direction: column; gap: 10px;",
            div {
                style: "display: flex; align-items: center; justify-content: space-between; gap: 12px;",
                span {
                    style: "font-size: 0.8rem; color: var(--qualia-text-muted);",
                    "QualiaDB epistemic analysis"
                }
                button {
                    disabled: busy(),
                    onclick: on_run,
                    style: "padding: 6px 14px; border-radius: 6px; border: 1px solid var(--qualia-accent); background: var(--qualia-accent); color: white; font-weight: 600; font-size: 0.78rem; cursor: pointer;",
                    if busy() { "Analyzing…" } else { "Commit to QualiaDB" }
                }
            }

            if let Some(err) = error() {
                div {
                    style: "font-size: 0.78rem; color: #f87171;",
                    "Engine error: {err}"
                }
            }

            if let Some(res) = result() {
                div {
                    style: "display: flex; flex-direction: column; gap: 8px;",
                    div { style: "font-size: 0.8rem; color: var(--qualia-text);", "{res.summary}" }
                    div {
                        style: "display: flex; flex-direction: column; gap: 4px;",
                        for a in res.assertions.iter() {
                            div {
                                style: "font-size: 0.72rem; font-family: monospace; color: var(--qualia-text-muted); padding: 4px 8px; border-left: 2px solid var(--qualia-accent); background: var(--qualia-bg);",
                                "{a}"
                            }
                        }
                    }
                    div {
                        style: "display: flex; gap: 12px; font-size: 0.68rem; color: var(--qualia-text-muted);",
                        span { "provenance: {res.provenance_hash}" }
                        span { "engine: {res.engine}" }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_is_deterministic_and_filters_empty_fields() {
        let req = AnalysisRequest {
            discipline: "Anthropology".to_string(),
            fields: vec![
                ("Subfield".to_string(), "Cultural Anthropology".to_string()),
                ("Field Site".to_string(), "".to_string()),
            ],
            notes: "  ".to_string(),
        };
        let a = stub_analyze(&req);
        let b = stub_analyze(&req);
        assert_eq!(a, b, "stub must be deterministic");
        assert_eq!(a.engine, "stub");
        assert_eq!(
            a.assertions.len(),
            1,
            "empty field + blank notes are dropped"
        );
        assert!(a.provenance_hash.starts_with("q42:"));
    }
}
