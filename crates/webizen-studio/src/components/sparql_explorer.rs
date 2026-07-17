//! SPARQL Explorer — local knowledge graph query surface.
//!
//! Host: `execute_sparql_query` → `qualia_client_core::engine::semantic::execute_local_sparql`.
//! Honesty: **Partial** — real local SPARQL planner/executor over the resident graph;
//! results are hex slot dumps, not full IRI-resolved SPARQL result sets.

use dioxus::prelude::*;

use crate::components::honesty_chip::{HonestyChip, HonestyLevel};
use crate::components::qapp_engine::invoke_json;

const SAMPLE_SELECT: &str =
    "SELECT ?subject ?predicate ?object\nWHERE {\n  ?subject ?predicate ?object\n}\nLIMIT 10";

const PRESET_LIMITED: &str =
    "SELECT ?s ?p ?o\nWHERE {\n  ?s ?p ?o\n}\nLIMIT 5";

const PRESET_TYPE_PATTERN: &str =
    "SELECT ?s ?type\nWHERE {\n  ?s <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> ?type\n}\nLIMIT 20";

#[derive(Clone, Copy, PartialEq, Eq)]
enum QueryPhase {
    Idle,
    Loading,
    Ok,
    Empty,
    Error,
}

#[component]
pub fn SparqlExplorer() -> Element {
    let mut query = use_signal(|| SAMPLE_SELECT.to_string());
    let mut results = use_signal(|| Vec::<(String, String, String)>::new());
    let mut phase = use_signal(|| QueryPhase::Idle);
    let mut status = use_signal(|| String::new());
    let mut error_detail = use_signal(|| String::new());

    let run_query = move |_| {
        phase.set(QueryPhase::Loading);
        status.set("Executing local SPARQL…".to_string());
        error_detail.set(String::new());
        results.set(Vec::new());

        spawn(async move {
            let args = serde_json::json!({
                "query": query.read().clone(),
            });

            match invoke_json("execute_sparql_query", args).await {
                Ok(res) => match serde_json::from_value::<Vec<(String, String, String)>>(res) {
                    Ok(data) if data.is_empty() => {
                        results.set(Vec::new());
                        phase.set(QueryPhase::Empty);
                        status.set(
                            "Query succeeded — 0 bindings. The resident graph may be empty, or the pattern matched nothing."
                                .to_string(),
                        );
                    }
                    Ok(data) => {
                        let n = data.len();
                        results.set(data);
                        phase.set(QueryPhase::Ok);
                        status.set(format!("{n} binding(s) from local graph (slot hashes)."));
                    }
                    Err(e) => {
                        results.set(Vec::new());
                        phase.set(QueryPhase::Error);
                        status.set("Host returned a value that is not a result table.".to_string());
                        error_detail.set(e.to_string());
                    }
                },
                Err(e) => {
                    results.set(Vec::new());
                    phase.set(QueryPhase::Error);
                    status.set("SPARQL invoke failed.".to_string());
                    error_detail.set(e);
                }
            }
        });
    };

    let current_phase = *phase.read();
    let is_loading = current_phase == QueryPhase::Loading;
    let status_text = status.read().clone();
    let err_text = error_detail.read().clone();
    let rows = results.read().clone();

    rsx! {
        div {
            style: "flex: 1; display: flex; flex-direction: column; gap: 1rem; padding: 2rem; background: rgba(30, 30, 40, 0.6); backdrop-filter: blur(12px); border: 1px solid rgba(255, 255, 255, 0.1); border-radius: 16px; color: var(--qualia-text); min-height: 0; overflow-y: auto;",

            div {
                style: "display: flex; justify-content: space-between; align-items: flex-start; gap: 1rem; flex-wrap: wrap;",
                div {
                    h2 {
                        style: "margin: 0; font-family: 'Inter', sans-serif; font-size: 1.8rem; background: linear-gradient(90deg, #00FF88, #00B8FF); -webkit-background-clip: text; -webkit-text-fill-color: transparent;",
                        "SPARQL Explorer"
                    }
                    p { style: "color: #A0A0B0; margin: 0.4rem 0 0 0;", "Query the local resident knowledge graph (not a remote SPARQL endpoint)." }
                }
                HonestyChip {
                    level: HonestyLevel::Partial,
                    detail: "Local execute_sparql_query · hex slots".to_string(),
                }
            }

            div {
                style: "display: flex; flex-wrap: wrap; gap: 0.5rem; align-items: center;",
                span { style: "font-size: 0.75rem; color: #6b7280; text-transform: uppercase; letter-spacing: 0.06em;", "Presets" }
                button {
                    style: "padding: 0.35rem 0.75rem; background: rgba(0,255,136,0.08); border: 1px solid rgba(0,255,136,0.25); border-radius: 999px; color: #86efac; font-size: 0.8rem; cursor: pointer;",
                    onclick: move |_| query.set(SAMPLE_SELECT.to_string()),
                    "SELECT * LIMIT 10"
                }
                button {
                    style: "padding: 0.35rem 0.75rem; background: rgba(0,184,255,0.08); border: 1px solid rgba(0,184,255,0.25); border-radius: 999px; color: #7dd3fc; font-size: 0.8rem; cursor: pointer;",
                    onclick: move |_| query.set(PRESET_LIMITED.to_string()),
                    "LIMIT 5 sample"
                }
                button {
                    style: "padding: 0.35rem 0.75rem; background: rgba(168,85,247,0.08); border: 1px solid rgba(168,85,247,0.25); border-radius: 999px; color: #d8b4fe; font-size: 0.8rem; cursor: pointer;",
                    onclick: move |_| query.set(PRESET_TYPE_PATTERN.to_string()),
                    "rdf:type pattern"
                }
            }

            textarea {
                style: "width: 100%; height: 200px; padding: 1rem; background: rgba(0, 0, 0, 0.4); border: 1px solid rgba(0, 255, 136, 0.3); border-radius: 8px; color: #E0E0E0; font-family: 'JetBrains Mono', monospace; resize: vertical; transition: all 0.3s ease; box-sizing: border-box;",
                value: "{query}",
                oninput: move |e| query.set(e.value().clone()),
            }

            div {
                style: "display: flex; justify-content: space-between; align-items: center; gap: 1rem; flex-wrap: wrap;",
                span {
                    style: "font-size: 0.85rem; color: #9ca3af; flex: 1; min-width: 12rem;",
                    if status_text.is_empty() {
                        "Run a query against the local graph. Empty graphs return 0 rows (not an error)."
                    } else {
                        "{status_text}"
                    }
                }
                button {
                    style: if is_loading {
                        "padding: 0.8rem 1.5rem; background: linear-gradient(45deg, #00FF88, #00B8FF); border: none; border-radius: 8px; color: #000; font-weight: bold; cursor: pointer; transition: transform 0.2s, box-shadow 0.2s; box-shadow: 0 4px 12px rgba(0, 255, 136, 0.2); opacity: 0.7;"
                    } else {
                        "padding: 0.8rem 1.5rem; background: linear-gradient(45deg, #00FF88, #00B8FF); border: none; border-radius: 8px; color: #000; font-weight: bold; cursor: pointer; transition: transform 0.2s, box-shadow 0.2s; box-shadow: 0 4px 12px rgba(0, 255, 136, 0.2); opacity: 1;"
                    },
                    disabled: is_loading,
                    onclick: run_query,
                    if is_loading {
                        "Executing…"
                    } else {
                        "Run Query"
                    }
                }
            }

            // Error banner — honest host failure, never stuffed into the result table
            if current_phase == QueryPhase::Error {
                div {
                    style: "padding: 1rem 1.1rem; border-radius: 8px; background: rgba(127, 29, 29, 0.35); border: 1px solid rgba(248, 113, 113, 0.45); color: #fecaca;",
                    div { style: "font-weight: 700; margin-bottom: 0.35rem;", "Query failed" }
                    p { style: "margin: 0 0 0.35rem 0; font-size: 0.9rem;", "{status_text}" }
                    if !err_text.is_empty() {
                        pre {
                            style: "margin: 0; white-space: pre-wrap; word-break: break-word; font-family: 'JetBrains Mono', monospace; font-size: 0.8rem; color: #fca5a5;",
                            "{err_text}"
                        }
                    }
                }
            }

            // Empty success
            if current_phase == QueryPhase::Empty {
                div {
                    style: "padding: 2rem 1.25rem; border-radius: 8px; background: rgba(0, 0, 0, 0.25); border: 1px dashed rgba(148, 163, 184, 0.35); text-align: center; color: #94a3b8;",
                    div { style: "font-weight: 600; color: #cbd5e1; margin-bottom: 0.4rem;", "No results" }
                    p { style: "margin: 0; font-size: 0.9rem; line-height: 1.5;",
                        "The query executed successfully but returned zero bindings. Seed the local graph (Talk, ontology import, or daemon ingest) or widen the pattern."
                    }
                }
            }

            // Loading placeholder
            if is_loading {
                div {
                    style: "padding: 1.5rem; border-radius: 8px; background: rgba(0, 0, 0, 0.2); border: 1px solid rgba(255, 255, 255, 0.05); color: #7dd3fc; text-align: center;",
                    "Loading results from host…"
                }
            }

            // Results table
            if current_phase == QueryPhase::Ok && !rows.is_empty() {
                div {
                    style: "margin-top: 0.5rem; background: rgba(0, 0, 0, 0.2); border-radius: 8px; overflow: auto; border: 1px solid rgba(255, 255, 255, 0.05); max-height: 28rem;",
                    table {
                        style: "width: 100%; border-collapse: collapse; text-align: left;",
                        thead {
                            style: "background: rgba(255, 255, 255, 0.05); position: sticky; top: 0;",
                            tr {
                                th { style: "padding: 1rem; border-bottom: 1px solid rgba(255, 255, 255, 0.1); color: #00B8FF;", "Subject" }
                                th { style: "padding: 1rem; border-bottom: 1px solid rgba(255, 255, 255, 0.1); color: #00FF88;", "Predicate" }
                                th { style: "padding: 1rem; border-bottom: 1px solid rgba(255, 255, 255, 0.1); color: #FF00FF;", "Object" }
                            }
                        }
                        tbody {
                            for (s, p, o) in rows.iter() {
                                tr {
                                    style: "border-bottom: 1px solid rgba(255, 255, 255, 0.05);",
                                    td { style: "padding: 1rem; font-family: 'JetBrains Mono', monospace; font-size: 0.85em;", "{s}" }
                                    td { style: "padding: 1rem; font-family: 'JetBrains Mono', monospace; font-size: 0.85em;", "{p}" }
                                    td { style: "padding: 1rem; font-family: 'JetBrains Mono', monospace; font-size: 0.85em;", "{o}" }
                                }
                            }
                        }
                    }
                }
            }

            if current_phase == QueryPhase::Idle {
                div {
                    style: "padding: 1.25rem; border-radius: 8px; background: rgba(0, 0, 0, 0.15); border: 1px solid rgba(255, 255, 255, 0.04); color: #6b7280; font-size: 0.9rem;",
                    "Results appear here after Run Query. Errors stay in the red banner — they are never shown as fake table rows."
                }
            }
        }
    }
}
