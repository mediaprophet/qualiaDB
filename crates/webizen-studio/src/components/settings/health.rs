use super::types::AgentQaSnapshot;
use dioxus::prelude::*;

#[component]
pub fn SetupHealthPanel(
    snapshot: Option<AgentQaSnapshot>,
    loading: bool,
    on_refresh: EventHandler<()>,
) -> Element {
    if loading {
        return rsx! { div { style: "{super::EMPTY_CARD}", "Inspecting this Webizen…" } };
    }
    let Some(snapshot) = snapshot else {
        return rsx! {
            div { style: "{super::WARNING_CARD}",
                strong { "Desktop host not available" }
                p { style: "margin:6px 0 12px;font-size:.74rem;", "Setup health uses structured native diagnostics and is unavailable in the public preview." }
                button { style: "{super::SECONDARY_BUTTON}", onclick: move |_| on_refresh.call(()), "Try again" }
            }
        };
    };

    let setup_complete = snapshot
        .setup
        .get("completed")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let model_ready = snapshot.active_model.is_some();
    let mail_running = snapshot
        .mail_receiver
        .get("running")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let cards = [
        (
            "Data home",
            true,
            snapshot.config.storage_path.clone(),
            "Data",
        ),
        (
            "Local model",
            model_ready,
            snapshot
                .active_model
                .clone()
                .unwrap_or_else(|| "Choose and test a model".to_string()),
            "Models",
        ),
        (
            "First-run foundations",
            setup_complete,
            if setup_complete {
                "Reviewed"
            } else {
                "Incomplete"
            }
            .to_string(),
            "Setup",
        ),
        (
            "Mail reception",
            mail_running,
            if mail_running {
                "Receiver running"
            } else {
                "Optional · not running"
            }
            .to_string(),
            "Relations",
        ),
        (
            "Local services",
            !snapshot.daemon_status.is_empty(),
            snapshot.daemon_status.clone(),
            "Services",
        ),
    ];

    rsx! {
        section { style: "display:grid;gap:18px;",
            div { style: "display:flex;align-items:flex-start;justify-content:space-between;gap:16px;flex-wrap:wrap;",
                div {
                    h2 { style: "margin:0;font-size:1.3rem;", "Your Webizen" }
                    p { style: "margin:.35rem 0 0;color:var(--qualia-text-muted);font-size:.8rem;line-height:1.5;",
                        "A living view of what is ready, optional or needs attention."
                    }
                }
                button { style: "{super::SECONDARY_BUTTON}", onclick: move |_| on_refresh.call(()), "Refresh health" }
            }
            div { style: "display:grid;grid-template-columns:repeat(auto-fit,minmax(210px,1fr));gap:12px;",
                for (title, ready, detail, domain) in cards {
                    div { style: if ready { super::SUCCESS_CARD } else { super::WARNING_CARD },
                        div { style: "display:flex;align-items:center;justify-content:space-between;gap:8px;",
                            strong { "{title}" }
                            span { style: "font-size:.64rem;font-weight:800;text-transform:uppercase;letter-spacing:.06em;", if ready { "Ready" } else { "Review" } }
                        }
                        div { style: "margin-top:7px;font-size:.72rem;line-height:1.45;overflow-wrap:anywhere;", "{detail}" }
                        div { style: "margin-top:9px;font-size:.62rem;opacity:.72;", "{domain}" }
                    }
                }
            }
            div { style: "display:grid;grid-template-columns:minmax(0,1.3fr) minmax(260px,.7fr);gap:14px;",
                div { style: "{super::PANEL}",
                    h3 { style: "margin:0 0 12px;font-size:.9rem;", "Needs your attention" }
                    if !model_ready {
                        div { style: "{super::ACTION_ROW}",
                            div {
                                strong { "Choose and test a local model" }
                                p { style: "margin:4px 0 0;color:var(--qualia-text-muted);font-size:.7rem;", "Chat should not promise local replies until a real readiness probe passes." }
                            }
                        }
                    }
                    if !mail_running {
                        div { style: "{super::ACTION_ROW}",
                            div {
                                strong { "Public mail reception is off" }
                                p { style: "margin:4px 0 0;color:var(--qualia-text-muted);font-size:.7rem;", "This is optional. Local and peer conversations remain available." }
                            }
                        }
                    }
                    if model_ready && mail_running {
                        div { style: "{super::EMPTY_CARD}", "No immediate setup repairs detected." }
                    }
                }
                div { style: "{super::PANEL}",
                    h3 { style: "margin:0 0 10px;font-size:.9rem;", "Structured evidence" }
                    dl { style: "margin:0;display:grid;grid-template-columns:auto 1fr;gap:8px 12px;font-size:.7rem;",
                        dt { style: "color:var(--qualia-text-muted);", "Schema" }
                        dd { style: "margin:0;", "{snapshot.schema_version}" }
                        dt { style: "color:var(--qualia-text-muted);", "Captured" }
                        dd { style: "margin:0;", "{snapshot.captured_at_unix}" }
                        dt { style: "color:var(--qualia-text-muted);", "Backend" }
                        dd { style: "margin:0;", "{snapshot.config.inference_backend}" }
                        dt { style: "color:var(--qualia-text-muted);", "Daemon" }
                        dd { style: "margin:0;overflow-wrap:anywhere;", "{snapshot.daemon_status}" }
                    }
                }
            }
        }
    }
}
