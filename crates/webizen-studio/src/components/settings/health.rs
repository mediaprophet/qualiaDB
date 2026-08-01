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
            "Local foundation",
        ),
        (
            "Local model",
            model_ready,
            snapshot
                .active_model
                .clone()
                .unwrap_or_else(|| "Choose and test a model".to_string()),
            "Local foundation",
        ),
        (
            "First-run foundations",
            setup_complete,
            if setup_complete {
                "Local foundations reviewed"
            } else {
                "Incomplete — finish local foundations"
            }
            .to_string(),
            "Local foundation",
        ),
        (
            "Person / apparatus IDs",
            setup_complete,
            if setup_complete {
                "Person principal ≠ this machine; multi-device fleet ready".to_string()
            } else {
                "Minted on open / finish setup".to_string()
            },
            "Local foundation",
        ),
        (
            "People & reachability",
            false,
            "Progressive · set up in Relations when peers connect".to_string(),
            "Progressive",
        ),
        (
            "Mail reception",
            mail_running,
            if mail_running {
                "Receiver running"
            } else {
                "Progressive · optional · not required to start"
            }
            .to_string(),
            "Progressive",
        ),
        (
            "Local services",
            !snapshot.daemon_status.is_empty(),
            snapshot.daemon_status.clone(),
            "Local foundation",
        ),
    ];

    rsx! {
        section { style: "display:grid;gap:18px;",
            div { style: "display:flex;align-items:flex-start;justify-content:space-between;gap:16px;flex-wrap:wrap;",
                div {
                    h2 { style: "margin:0;font-size:1.3rem;", "Your Webizen" }
                    p { style: "margin:.35rem 0 0;color:var(--qualia-text-muted);font-size:.8rem;line-height:1.5;",
                        "Living health: local foundations first, then progressive relational and network paths when they make sense."
                    }
                }
                button { style: "{super::SECONDARY_BUTTON}", onclick: move |_| on_refresh.call(()), "Refresh health" }
            }
            div { style: "display:grid;grid-template-columns:repeat(auto-fit,minmax(210px,1fr));gap:12px;",
                for (title, ready, detail, domain) in cards {
                    div { style: if ready { super::SUCCESS_CARD } else { super::WARNING_CARD },
                        div { style: "display:flex;align-items:center;justify-content:space-between;gap:8px;",
                            strong { "{title}" }
                            span { style: "font-size:.64rem;font-weight:800;text-transform:uppercase;letter-spacing:.06em;",
                                if ready { "Ready" } else if domain == "Progressive" { "Later" } else { "Review" }
                            }
                        }
                        div { style: "margin-top:7px;font-size:.72rem;line-height:1.45;overflow-wrap:anywhere;", "{detail}" }
                        div { style: "margin-top:9px;font-size:.62rem;opacity:.72;", "{domain}" }
                    }
                }
            }
            div { style: "display:grid;grid-template-columns:minmax(0,1.3fr) minmax(260px,.7fr);gap:14px;",
                div { style: "{super::PANEL}",
                    h3 { style: "margin:0 0 12px;font-size:.9rem;", "Needs your attention" }
                    if !setup_complete {
                        div { style: "{super::ACTION_ROW}",
                            div {
                                strong { "Finish local foundations" }
                                p { style: "margin:4px 0 0;color:var(--qualia-text-muted);font-size:.7rem;", "Data home, device, a local instrument and how you want to be known. Relational setup waits until after open." }
                            }
                        }
                    }
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
                                strong { "Mail and public reachability are progressive" }
                                p { style: "margin:4px 0 0;color:var(--qualia-text-muted);font-size:.7rem;", "Not required to start. Configure in Relations when you have a domain or peers." }
                            }
                        }
                    }
                    if setup_complete && model_ready {
                        div { style: "{super::EMPTY_CARD}", "Local foundations look fine. Extend people, reachability and backups when you need them." }
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
