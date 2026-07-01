//! Policy receipts — durable audit trail after WAL commits.

use super::host_client::fetch_receipts;
use super::host_dto::ReceiptDto;
use dioxus::prelude::*;

#[component]
pub fn WellfairReceiptsPanel() -> Element {
    let mut receipts = use_signal(Vec::<ReceiptDto>::new);
    let mut status = use_signal(|| String::new());

    let reload = move || {
        spawn(async move {
            match fetch_receipts(16).await {
                Ok(list) => {
                    let n = list.len();
                    receipts.set(list);
                    status.set(if n == 0 {
                        "No receipts yet — ingest health data to generate policy receipts.".into()
                    } else {
                        format!("{n} recent receipt(s).")
                    });
                }
                Err(e) => status.set(format!("Receipts unavailable: {e}")),
            }
        });
    };

    use_effect(move || {
        reload();
    });

    rsx! {
        section {
            aria_label: "WellFair policy receipts",
            style: "padding:0.85rem;border:1px solid var(--qualia-border,#ddd);border-radius:10px;background:var(--qualia-surface,#fafafa);",
            div {
                style: "display:flex;align-items:center;justify-content:space-between;gap:0.5rem;margin-bottom:0.5rem;",
                h2 { style: "margin:0;font-size:1rem;", "Policy receipts" }
                button {
                    style: "padding:0.25rem 0.55rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);background:transparent;font-size:0.75rem;cursor:pointer;",
                    onclick: move |_| reload(),
                    "Refresh"
                }
            }
            if !status().is_empty() {
                p {
                    style: "margin:0 0 0.5rem;font-size:0.76rem;color:var(--qualia-text-muted,#666);",
                    "{status()}"
                }
            }
            if receipts.read().is_empty() {
                rsx! {}
            } else {
                ul {
                    style: "margin:0;padding:0;list-style:none;display:flex;flex-direction:column;gap:0.4rem;",
                    for r in receipts.read().clone() {
                        li {
                            key: "{r.id}",
                            style: "padding:0.45rem 0.55rem;border-radius:6px;border:1px solid var(--qualia-border,#eee);font-size:0.74rem;",
                            div {
                                style: "display:flex;justify-content:space-between;gap:0.5rem;",
                                strong { "{r.decision}" }
                                span { style: "color:var(--qualia-text-muted,#888);", "{r.qapp_id}" }
                            }
                            div { style: "margin-top:0.2rem;color:var(--qualia-text-muted,#666);font-family:monospace;font-size:0.7rem;",
                                "{r.record_id}"
                            }
                            if let Some(cp) = &r.checkpoint_hash {
                                div { style: "margin-top:0.15rem;color:var(--qualia-text-muted,#888);font-size:0.68rem;",
                                    "checkpoint {cp.chars().take(12).collect::<String>()}…"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}