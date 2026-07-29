//! In-app structured diagnostics for human and agent hand-off.

use crate::components::settings::host::invoke_json;
use crate::components::settings::types::{AgentQaModelProbe, AgentQaSnapshot};
use dioxus::prelude::*;

#[component]
pub fn AgentQaPanel() -> Element {
    let mut snapshot = use_signal(|| Option::<AgentQaSnapshot>::None);
    let mut probe = use_signal(|| Option::<AgentQaModelProbe>::None);
    let mut status = use_signal(String::new);
    let mut busy = use_signal(|| false);

    let mut refresh = move || {
        busy.set(true);
        status.set("Collecting structured diagnostics…".to_string());
        spawn(async move {
            match invoke_json::<AgentQaSnapshot>("agent_qa_snapshot", serde_json::json!({})).await {
                Ok(value) => {
                    snapshot.set(Some(value));
                    status.set("Diagnostic snapshot ready.".to_string());
                }
                Err(error) => status.set(format!("Snapshot failed: {error}")),
            }
            busy.set(false);
        });
    };
    use_hook(move || refresh());

    let evidence = snapshot()
        .and_then(|value| serde_json::to_string_pretty(&value).ok())
        .unwrap_or_else(|| "{}".to_string());

    rsx! {
        div { style: "width:100%;height:100%;overflow-y:auto;padding:28px;background:var(--qualia-bg);color:var(--qualia-text);",
            div { style: "max-width:1120px;margin:0 auto;display:grid;gap:16px;",
                header { style: "display:flex;align-items:flex-start;justify-content:space-between;gap:16px;flex-wrap:wrap;",
                    div {
                        div { style: "font-size:.64rem;color:var(--qualia-accent);font-weight:800;letter-spacing:.09em;text-transform:uppercase;", "Assure · Agent QA" }
                        h1 { style: "margin:5px 0 4px;font-size:1.55rem;", "Structured diagnostics" }
                        p { style: "margin:0;color:var(--qualia-text-muted);font-size:.78rem;line-height:1.5;max-width:48rem;",
                            "Machine-readable state and reversible probes. No agent needs to scrape visual labels or terminal decoration to understand this Webizen."
                        }
                    }
                    div { style: "display:flex;gap:8px;",
                        button { style: "{crate::components::settings::SECONDARY_BUTTON}", disabled: busy(), onclick: move |_| refresh(), "Refresh snapshot" }
                        button {
                            style: "{crate::components::settings::PRIMARY_BUTTON}",
                            disabled: busy(),
                            onclick: move |_| {
                                busy.set(true);
                                status.set("Running reversible active-model probe…".to_string());
                                spawn(async move {
                                    match invoke_json::<AgentQaModelProbe>("agent_qa_test_active_model", serde_json::json!({})).await {
                                        Ok(value) => {
                                            status.set(if value.passed { "Model probe passed.".to_string() } else { "Model probe needs attention.".to_string() });
                                            probe.set(Some(value));
                                        }
                                        Err(error) => status.set(format!("Model probe failed: {error}")),
                                    }
                                    busy.set(false);
                                });
                            },
                            "Test active model"
                        }
                    }
                }
                div { role: "status", style: "{crate::components::settings::PANEL}", "{status}" }
                if let Some(value) = snapshot() {
                    div { style: "display:grid;grid-template-columns:repeat(auto-fit,minmax(210px,1fr));gap:12px;",
                        QaTile { title: "Setup", value: if value.setup.get("completed").and_then(serde_json::Value::as_bool).unwrap_or(false) { "Complete".to_string() } else { "Incomplete".to_string() } }
                        QaTile { title: "Active model", value: value.active_model.unwrap_or_else(|| "None".to_string()) }
                        QaTile { title: "Daemon", value: value.daemon_status }
                        QaTile { title: "Evidence schema", value: value.schema_version.to_string() }
                    }
                }
                if let Some(value) = probe() {
                    div { style: if value.passed { crate::components::settings::SUCCESS_CARD } else { crate::components::settings::WARNING_CARD },
                        strong { if value.passed { "Model probe passed" } else { "Model probe failed or was blocked" } }
                        div { style: "margin-top:6px;font-size:.72rem;", "Committed: {value.committed} · Cleanup: {value.cleanup_succeeded} · {value.duration_ms} ms" }
                        if !value.output_sample.is_empty() {
                            pre { style: "white-space:pre-wrap;font-size:.68rem;margin:10px 0 0;", "{value.output_sample}" }
                        }
                    }
                }
                section { style: "{crate::components::settings::PANEL}",
                    div { style: "display:flex;align-items:center;justify-content:space-between;gap:12px;",
                        h2 { style: "margin:0;font-size:.95rem;", "Snapshot JSON" }
                        button {
                            style: "{crate::components::settings::SECONDARY_BUTTON}",
                            onclick: move |_| {
                                #[cfg(target_arch = "wasm32")]
                                if let Some(window) = web_sys::window() {
                                    let clipboard = window.navigator().clipboard();
                                    let _ = clipboard.write_text(&evidence);
                                }
                            },
                            "Copy evidence"
                        }
                    }
                    pre { style: "margin:13px 0 0;padding:13px;border-radius:10px;background:#050a12;color:#cbd5e1;overflow:auto;max-height:460px;font-size:.67rem;line-height:1.5;", "{evidence}" }
                }
            }
        }
    }
}

#[component]
fn QaTile(title: &'static str, value: String) -> Element {
    rsx! {
        div { style: "{crate::components::settings::PANEL}",
            div { style: "font-size:.64rem;color:var(--qualia-text-muted);text-transform:uppercase;letter-spacing:.07em;", "{title}" }
            div { style: "margin-top:7px;font-size:.78rem;font-weight:750;overflow-wrap:anywhere;", "{value}" }
        }
    }
}
