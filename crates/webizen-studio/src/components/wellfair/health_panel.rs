//! Health observations — journal projection from durable WAL commits.

use super::host_client::{fetch_health_records, fetch_host_snapshot};
use super::host_dto::HealthRecordDto;
use super::shared::DomainChrome;
use dioxus::prelude::*;

fn format_unix(ts: u32) -> String {
    if ts == 0 {
        return "—".into();
    }
    // Human-readable without chrono dependency in studio.
    format!("{ts}")
}

fn kind_label(kind: &str) -> &str {
    match kind {
        "weight" => "Weight",
        "steps" => "Steps",
        "sleep" => "Sleep",
        "heart_rate" => "Heart rate",
        _ => "Record",
    }
}

#[component]
pub fn WellfairHealthPanel() -> Element {
    let mut records = use_signal(Vec::<HealthRecordDto>::new);
    let mut status = use_signal(|| "Loading health records…".to_string());
    let mut record_count = use_signal(|| 0u32);
    let mut graph_count = use_signal(|| 0u32);
    let mut checkpoint = use_signal(|| None::<String>);

    let reload = move || {
        spawn(async move {
            status.set("Loading…".into());
            let snap = fetch_host_snapshot().await;
            record_count.set(snap.health_record_count);
            graph_count.set(snap.graph_quin_count);
            checkpoint.set(snap.last_checkpoint_prefix.clone());
            match fetch_health_records(64).await {
                Ok(list) => {
                    let n = list.len();
                    records.set(list);
                    status.set(if n == 0 {
                        "No health records yet. Sync from your phone via Tools.".into()
                    } else {
                        format!("{n} recent record(s) from vault journal.")
                    });
                }
                Err(e) => status.set(format!("Could not load records: {e}")),
            }
        });
    };

    let mut loaded = use_signal(|| false);

    use_effect(move || {
        if loaded() { return; }
        loaded.set(true);
        reload();
    });

    let grouped: Vec<(String, Vec<HealthRecordDto>)> = {
        let list = records.read().clone();
        let kinds = ["weight", "steps", "sleep", "heart_rate", "record"];
        kinds
            .iter()
            .filter_map(|k| {
                let subset: Vec<_> = list.iter().filter(|r| r.kind == *k).cloned().collect();
                if subset.is_empty() {
                    None
                } else {
                    Some(((*k).to_string(), subset))
                }
            })
            .collect()
    };

    rsx! {
        section {
            aria_label: "WellFair health observations",
            style: "padding:0.85rem;border:1px solid var(--qualia-border,#ddd);border-radius:10px;background:var(--qualia-surface,#fafafa);",
            DomainChrome { domain: "Care", chip: "Body · local vault", show_memory: true }
            div {
                style: "display:flex;flex-wrap:wrap;align-items:center;justify-content:space-between;gap:0.5rem;margin-bottom:0.75rem;",
                h2 { style: "margin:0;font-size:1rem;", "Health — observations" }
                div {
                    style: "display:flex;gap:0.5rem;align-items:center;font-size:0.78rem;color:var(--qualia-text-muted,#666);",
                    span { "{record_count()} journal · {graph_count()} graph quins" }
                    if let Some(cp) = checkpoint() {
                        span { "· checkpoint {cp}…" }
                    }
                    button {
                        style: "padding:0.25rem 0.55rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);background:transparent;font-size:0.75rem;cursor:pointer;",
                        onclick: move |_| reload(),
                        "Refresh"
                    }
                }
            }
            p {
                style: "margin:0 0 0.75rem;font-size:0.78rem;color:var(--qualia-text-muted,#666);",
                "{status()}"
            }
            if grouped.is_empty() && !records.read().is_empty() {
                HealthRecordTable { rows: records.read().clone() }
            } else {
                for (kind, rows) in grouped {
                    div {
                        key: "{kind}",
                        style: "margin-bottom:0.85rem;",
                        h3 {
                            style: "margin:0 0 0.35rem;font-size:0.88rem;",
                            "{kind_label(&kind)} ({rows.len()})"
                        }
                        HealthRecordTable { rows: rows.clone() }
                    }
                }
            }
        }
    }
}

#[component]
fn HealthRecordTable(rows: Vec<HealthRecordDto>) -> Element {
    rsx! {
        div {
            style: "overflow-x:auto;",
            table {
                style: "width:100%;border-collapse:collapse;font-size:0.76rem;",
                thead {
                    tr {
                        style: "text-align:left;border-bottom:1px solid var(--qualia-border,#ddd);",
                        th { style: "padding:0.35rem 0.5rem;", "When" }
                        th { style: "padding:0.35rem 0.5rem;", "Source" }
                        th { style: "padding:0.35rem 0.5rem;", "Evidence" }
                        th { style: "padding:0.35rem 0.5rem;", "Hash" }
                    }
                }
                tbody {
                    for row in rows {
                        tr {
                            key: "{row.id}",
                            style: "border-bottom:1px solid var(--qualia-border,#eee);",
                            td { style: "padding:0.35rem 0.5rem;white-space:nowrap;", "{format_unix(row.asserted_time_unix)}" }
                            td { style: "padding:0.35rem 0.5rem;", "{row.source}" }
                            td { style: "padding:0.35rem 0.5rem;", "{row.evidence_type}" }
                            td {
                                style: "padding:0.35rem 0.5rem;font-family:monospace;font-size:0.72rem;",
                                if let Some(h) = &row.blob_hash {
                                    "{h.chars().take(8).collect::<String>()}…"
                                } else {
                                    "—"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}