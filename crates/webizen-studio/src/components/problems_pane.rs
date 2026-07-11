use dioxus::prelude::*;
use serde::Deserialize;
use std::time::Duration;

use crate::components::qapp_engine::invoke_json;

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct ServiceSnapshot {
    pub id: String,
    pub state: String,
    pub detail: String,
    pub updated_at: String,
    pub heartbeat_at: String,
    pub restart_count: u32,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct OperationSnapshot {
    pub id: String,
    pub kind: String,
    pub label: String,
    pub state: String,
    pub stage: String,
    pub completed_units: u64,
    pub total_units: Option<u64>,
    pub error: Option<String>,
    pub started_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct SupervisorState {
    pub services: Vec<ServiceSnapshot>,
    pub operations: Vec<OperationSnapshot>,
}

#[component]
pub fn ProblemsPane() -> Element {
    let mut state = use_signal(|| SupervisorState::default());
    let mut error = use_signal(|| None::<String>);

    use_effect(move || {
        spawn(async move {
            loop {
                if let Ok(res) = invoke_json("get_supervisor_state", serde_json::json!({})).await {
                    match serde_json::from_value::<SupervisorState>(res) {
                        Ok(data) => {
                            state.set(data);
                            error.set(None);
                        }
                        Err(e) => {
                            error.set(Some(format!("Parse error: {}", e)));
                        }
                    }
                } else {
                    error.set(Some("Failed to fetch supervisor state".to_string()));
                }
                #[cfg(target_arch = "wasm32")]
                {
                    gloo_timers::future::sleep(Duration::from_secs(2)).await;
                }
                #[cfg(not(target_arch = "wasm32"))]
                {
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
            }
        });
    });

    let current_state = state();
    
    let failed_services: Vec<_> = current_state.services.iter()
        .filter(|s| s.state == "failed" || s.state == "degraded")
        .collect();
        
    let failed_operations: Vec<_> = current_state.operations.iter()
        .filter(|o| o.state == "failed")
        .collect();

    let active_operations: Vec<_> = current_state.operations.iter()
        .filter(|o| o.state == "running" || o.state == "queued")
        .collect();

    rsx! {
        div {
            style: "width: 100%; height: 100%; overflow: auto; display: flex; flex-direction: column; padding: 2rem; gap: 2rem; background: var(--qualia-surface);",
            
            div {
                h1 { style: "margin: 0 0 0.5rem; font-size: 1.5rem; color: var(--qualia-text);", "System Problems & Operations" }
                p { style: "margin: 0; color: var(--qualia-text-muted); font-size: 0.9rem;", "Monitor failed services, active tasks, and required human interventions." }
                if let Some(err) = error() {
                    div {
                        style: "margin-top: 1rem; padding: 0.75rem; background: rgba(255,50,50,0.1); border: 1px solid rgba(255,50,50,0.2); border-radius: 8px; color: #ff6b6b;",
                        "Warning: {err}"
                    }
                }
            }

            // Failed Services
            if !failed_services.is_empty() {
                div {
                    style: "display: flex; flex-direction: column; gap: 1rem;",
                    h2 { style: "margin: 0; font-size: 1.2rem; color: #ff6b6b; display: flex; align-items: center; gap: 0.5rem;", 
                        sl-icon { name: "exclamation-triangle" }
                        "Failed Services" 
                    }
                    for s in failed_services {
                        div {
                            key: "{s.id}",
                            style: "background: rgba(255,50,50,0.05); border: 1px solid rgba(255,50,50,0.2); border-radius: 12px; padding: 1rem; display: flex; flex-direction: column; gap: 0.5rem;",
                            div {
                                style: "display: flex; justify-content: space-between; align-items: center;",
                                span { style: "font-weight: 600; color: var(--qualia-text);", "{s.id}" }
                                span { style: "background: rgba(255,50,50,0.1); color: #ff6b6b; padding: 0.2rem 0.5rem; border-radius: 4px; font-size: 0.8rem;", "{s.state}" }
                            }
                            span { style: "color: var(--qualia-text-muted); font-size: 0.9rem;", "{s.detail}" }
                            div {
                                style: "display: flex; gap: 1rem; margin-top: 0.5rem;",
                                button {
                                    style: "padding: 0.4rem 1rem; background: #ff6b6b; color: white; border: none; border-radius: 6px; cursor: pointer; font-weight: 600;",
                                    "Restart Service"
                                }
                                button {
                                    style: "padding: 0.4rem 1rem; background: transparent; border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 6px; cursor: pointer;",
                                    "View Logs"
                                }
                            }
                        }
                    }
                }
            }

            // Failed Operations (Require Human Intervention)
            if !failed_operations.is_empty() {
                div {
                    style: "display: flex; flex-direction: column; gap: 1rem;",
                    h2 { style: "margin: 0; font-size: 1.2rem; color: #ff9e6b; display: flex; align-items: center; gap: 0.5rem;", 
                        sl-icon { name: "exclamation-circle" }
                        "Failed Operations" 
                    }
                    for o in failed_operations {
                        div {
                            key: "{o.id}",
                            style: "background: rgba(255,158,107,0.05); border: 1px solid rgba(255,158,107,0.2); border-radius: 12px; padding: 1rem; display: flex; flex-direction: column; gap: 0.5rem;",
                            div {
                                style: "display: flex; justify-content: space-between; align-items: center;",
                                span { style: "font-weight: 600; color: var(--qualia-text);", "{o.label} ({o.kind})" }
                                span { style: "background: rgba(255,158,107,0.1); color: #ff9e6b; padding: 0.2rem 0.5rem; border-radius: 4px; font-size: 0.8rem;", "{o.state}" }
                            }
                            if let Some(err) = &o.error {
                                span { style: "color: var(--qualia-text-muted); font-size: 0.9rem; font-family: monospace;", "{err}" }
                            }
                            div {
                                style: "display: flex; gap: 1rem; margin-top: 0.5rem;",
                                button {
                                    style: "padding: 0.4rem 1rem; background: #ff9e6b; color: #1a1a1a; border: none; border-radius: 6px; cursor: pointer; font-weight: 600;",
                                    "Retry Operation"
                                }
                                button {
                                    style: "padding: 0.4rem 1rem; background: transparent; border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 6px; cursor: pointer;",
                                    "Dismiss"
                                }
                            }
                        }
                    }
                }
            }

            // Active Operations
            div {
                style: "display: flex; flex-direction: column; gap: 1rem;",
                h2 { style: "margin: 0; font-size: 1.2rem; color: var(--qualia-text); display: flex; align-items: center; gap: 0.5rem;", 
                    sl-icon { name: "activity" }
                    "Active Operations" 
                }
                if active_operations.is_empty() {
                    div {
                        style: "padding: 2rem; border: 1px dashed var(--qualia-border); border-radius: 12px; text-align: center; color: var(--qualia-text-muted);",
                        "No active operations"
                    }
                } else {
                    for o in active_operations {
                        div {
                            key: "{o.id}",
                            style: "background: rgba(255,255,255,0.02); border: 1px solid var(--qualia-border); border-radius: 12px; padding: 1rem; display: flex; flex-direction: column; gap: 0.5rem;",
                            div {
                                style: "display: flex; justify-content: space-between; align-items: center;",
                                span { style: "font-weight: 600; color: var(--qualia-text);", "{o.label}" }
                                span { style: "color: var(--qualia-text-muted); font-size: 0.8rem;", "Stage: {o.stage}" }
                            }
                            if let Some(total) = o.total_units {
                                div {
                                    style: "width: 100%; background: rgba(255,255,255,0.1); border-radius: 4px; height: 4px; overflow: hidden; margin-top: 0.5rem;",
                                    div {
                                        style: "height: 100%; background: var(--qualia-accent); width: {o.completed_units as f64 / total as f64 * 100.0}%; transition: width 0.3s ease;",
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
