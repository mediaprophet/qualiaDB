//! Audit & status — policy receipts + journal→quin graph coverage (Phase 2 closeout).

use super::host_client::{fetch_graph_coverage, fetch_receipts};
use super::host_dto::{GraphCoverageDto, ReceiptDto};
use dioxus::prelude::*;

fn cp_prefix(hash: &str) -> String {
    hash.chars().take(12).collect()
}

#[component]
pub fn WellfairAuditPanel() -> Element {
    let mut receipts = use_signal(Vec::<ReceiptDto>::new);
    let mut coverage = use_signal(Vec::<GraphCoverageDto>::new);
    let mut status = use_signal(|| "Loading audit trail…".to_string());

    let reload = move || {
        spawn(async move {
            status.set("Loading…".into());
            let mut msgs = Vec::new();
            match fetch_receipts(24).await {
                Ok(list) => {
                    let n = list.len();
                    receipts.set(list);
                    msgs.push(format!("{n} receipt(s)"));
                }
                Err(e) => msgs.push(format!("receipts error: {e}")),
            }
            match fetch_graph_coverage(64).await {
                Ok(rows) => {
                    let materialized = rows.iter().filter(|r| r.quin_count > 0).count();
                    coverage.set(rows);
                    msgs.push(format!("{materialized} record(s) materialized to quins"));
                }
                Err(e) => msgs.push(format!("coverage error: {e}")),
            }
            status.set(msgs.join(" · "));
        });
    };

    let mut loaded = use_signal(|| false);

    use_effect(move || {
        if loaded() { return; }
        loaded.set(true);
        reload();
    });

    rsx! {
        section {
            aria_label: "WellFair audit and graph coverage",
            style: "padding:0.85rem;border:1px solid var(--qualia-border,#ddd);border-radius:10px;background:var(--qualia-surface,#fafafa);",
            super::shared::DomainChrome { domain: "Instruments", chip: "Audit · provenance · local", show_memory: true }
            div {
                style: "display:flex;align-items:center;justify-content:space-between;gap:0.5rem;margin-bottom:0.5rem;",
                h2 { style: "margin:0;font-size:1rem;", "Audit & provenance" }
                button {
                    style: "padding:0.25rem 0.55rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);background:transparent;font-size:0.75rem;cursor:pointer;",
                    onclick: move |_| reload(),
                    "Refresh"
                }
            }
            p {
                style: "margin:0 0 0.75rem;font-size:0.76rem;color:var(--qualia-text-muted,#666);",
                "{status()}"
            }

            if !coverage.read().is_empty() {
                h3 { style: "margin:0 0 0.35rem;font-size:0.88rem;", "Graph coverage (journal → quins)" }
                div {
                    style: "margin:0 0 0.85rem;max-height:140px;overflow:auto;border:1px solid var(--qualia-border,#eee);border-radius:8px;",
                    table {
                        style: "width:100%;border-collapse:collapse;font-size:0.72rem;",
                        thead {
                            tr {
                                style: "background:var(--qualia-border,#eee);text-align:left;",
                                th { style: "padding:0.35rem 0.45rem;", "Kind" }
                                th { style: "padding:0.35rem 0.45rem;", "Quins" }
                                th { style: "padding:0.35rem 0.45rem;", "Record" }
                            }
                        }
                        tbody {
                            for row in coverage.read().clone() {
                                tr {
                                    key: "{row.record_id}",
                                    td { style: "padding:0.3rem 0.45rem;", "{row.kind}" }
                                    td {
                                        style: "padding:0.3rem 0.45rem;font-weight:600;",
                                        if row.quin_count > 0 { "{row.quin_count}" } else { "—" }
                                    }
                                    td {
                                        style: "padding:0.3rem 0.45rem;font-family:monospace;color:var(--qualia-text-muted,#666);",
                                        "{row.record_id.chars().take(36).collect::<String>()}…"
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if !receipts.read().is_empty() {
                h3 { style: "margin:0 0 0.35rem;font-size:0.88rem;", "Policy receipts" }
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
                                    "checkpoint {cp_prefix(cp)}…"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}