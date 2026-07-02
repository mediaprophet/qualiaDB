//! Sync inbox status — read-only view of quarantined/validated inbound operations (Phase 5).

use super::host_client::{fetch_sync_inbox, SyncInboxRecordDto};
use dioxus::prelude::*;

#[derive(Clone, Debug, Default)]
struct SyncUi {
    status: String,
    records: Vec<SyncInboxRecordDto>,
}

fn outcome_color(state: &str) -> &'static str {
    match state {
        "validated" => "#2a7a3f",
        "duplicate" => "#8a6d1d",
        "rejected" => "#b5341f",
        _ => "#666",
    }
}

#[component]
pub fn WellfairSyncPanel() -> Element {
    let mut ui = use_signal(SyncUi::default);

    let reload = move || {
        spawn(async move {
            match fetch_sync_inbox(64).await {
                Ok(records) => ui.write().records = records,
                Err(e) => ui.write().status = format!("Inbox unavailable: {e}"),
            }
        });
    };

    use_effect(move || {
        reload();
    });

    rsx! {
        section {
            aria_label: "WellFair sync inbox",
            style: "padding:0.85rem;border:1px solid var(--qualia-border,#ddd);border-radius:10px;background:var(--qualia-surface,#fafafa);margin-bottom:0.85rem;",
            h2 { style: "margin:0 0 0.5rem;font-size:1rem;", "Sync inbox" }
            p {
                style: "margin:0 0 0.75rem;font-size:0.74rem;color:var(--qualia-text-muted,#666);",
                "Inbound sync operations are quarantined and validated fail-closed. Replays are recorded as duplicates and never applied twice; Sanctuary-classified frames are rejected."
            }
            if !ui().status.is_empty() {
                p { style: "margin:0 0 0.5rem;font-size:0.76rem;", "{ui().status}" }
            }

            if ui().records.is_empty() {
                p {
                    style: "margin:0;font-size:0.74rem;color:var(--qualia-text-muted,#888);",
                    "No inbound operations."
                }
            } else {
                ul {
                    style: "margin:0;padding:0;list-style:none;display:flex;flex-direction:column;gap:0.35rem;",
                    for rec in ui().records.clone() {
                        li {
                            key: "{rec.operation.operation_id}",
                            style: "padding:0.4rem 0.5rem;border:1px solid var(--qualia-border,#eee);border-radius:6px;font-size:0.74rem;display:flex;justify-content:space-between;gap:0.5rem;",
                            span {
                                strong { "{rec.operation.kind}" }
                                span { style: "margin-left:0.35rem;color:var(--qualia-text-muted,#888);",
                                    "lamport {rec.operation.lamport} · {rec.operation.sensitivity}"
                                }
                            }
                            span {
                                style: format!("font-weight:700;color:{};", outcome_color(&rec.outcome.state)),
                                "{rec.outcome.state}"
                                if let Some(reason) = &rec.outcome.reason {
                                    span { style: "font-weight:400;color:var(--qualia-text-muted,#888);margin-left:0.3rem;",
                                        "({reason})"
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
