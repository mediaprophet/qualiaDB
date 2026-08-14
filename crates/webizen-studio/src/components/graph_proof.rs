//! Desktop surface for the bounded N-Triples → Q42 graph proof.

use dioxus::prelude::*;
use serde::Deserialize;

use crate::components::qapp_engine::invoke_json;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Ready,
    Running,
    Proven,
    Different,
    BlankNodeCaution,
    Error,
}

#[derive(Clone, Debug, Deserialize)]
struct Report {
    source_records: u64,
    q42_records: u64,
    source_unique_records: u64,
    q42_unique_records: u64,
    missing_from_q42: u64,
    unexpected_in_q42: u64,
    first_missing: Option<Vec<u64>>,
    first_unexpected: Option<Vec<u64>>,
    source_skipped_lines: u64,
    source_contains_blank_nodes: bool,
    rdf_isomorphism: String,
}

fn limit(value: &str, name: &str) -> Result<u64, String> {
    match value.trim().parse::<u64>() {
        Ok(value) if value > 0 => Ok(value),
        _ => Err(format!("{name} must be a positive whole number")),
    }
}

fn count(value: u64) -> String {
    let raw = value.to_string();
    let mut out = String::with_capacity(raw.len() + raw.len() / 3);
    for (index, character) in raw.chars().enumerate() {
        if index > 0 && (raw.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(character);
    }
    out
}

fn witness(record: &Option<Vec<u64>>) -> String {
    record
        .as_ref()
        .map(|fields| {
            fields
                .iter()
                .map(|field| format!("0x{field:016X}"))
                .collect::<Vec<_>>()
                .join(" · ")
        })
        .unwrap_or_else(|| "—".to_string())
}

#[component]
pub fn GraphProof() -> Element {
    let mut source = use_signal(String::new);
    let mut volume = use_signal(String::new);
    let mut memory_mib = use_signal(|| "32".to_string());
    let mut temp_gib = use_signal(|| "32".to_string());
    let mut phase = use_signal(|| Phase::Ready);
    let mut status = use_signal(String::new);
    let mut report = use_signal(|| None::<Report>);

    let start = move |_| {
        let source_path = source.read().trim().to_string();
        let q42_path = volume.read().trim().to_string();
        let memory = match limit(&memory_mib.read(), "RAM budget") {
            Ok(value) => value,
            Err(error) => {
                phase.set(Phase::Error);
                status.set(error);
                return;
            }
        };
        let temporary = match limit(&temp_gib.read(), "Temporary disk budget") {
            Ok(value) => value,
            Err(error) => {
                phase.set(Phase::Error);
                status.set(error);
                return;
            }
        };
        if source_path.is_empty() || q42_path.is_empty() {
            phase.set(Phase::Error);
            status.set("Choose the N-Triples source and its Q42 volume.".to_string());
            return;
        }

        phase.set(Phase::Running);
        report.set(None);
        status.set("Creating bounded sort runs and comparing encoded quad sets…".to_string());
        spawn(async move {
            let arguments = serde_json::json!({
                "source_path": source_path,
                "q42_path": q42_path,
                "memory_mib": memory,
                "temp_gib": temporary,
            });
            match invoke_json("verify_graph_equivalence", arguments).await {
                Ok(value) => match serde_json::from_value::<Report>(value) {
                    Ok(value) => {
                        let matched = value.missing_from_q42 == 0 && value.unexpected_in_q42 == 0;
                        let next = if !matched {
                            Phase::Different
                        } else if value.source_contains_blank_nodes {
                            Phase::BlankNodeCaution
                        } else {
                            Phase::Proven
                        };
                        status.set(match next {
                            Phase::Proven => "Exact ground-graph equivalence proven.".to_string(),
                            Phase::BlankNodeCaution => "Encoded sets match; canonical blank-node RDF isomorphism remains unproven.".to_string(),
                            Phase::Different => "Encoded graphs differ. Witnesses are shown below.".to_string(),
                            _ => String::new(),
                        });
                        report.set(Some(value));
                        phase.set(next);
                    }
                    Err(error) => {
                        phase.set(Phase::Error);
                        status.set(format!("Invalid proof report from desktop host: {error}"));
                    }
                },
                Err(error) => {
                    phase.set(Phase::Error);
                    status.set(error);
                }
            }
        });
    };

    let current = *phase.read();
    let running = current == Phase::Running;
    let message = status.read().clone();
    let current_report = report.read().clone();
    let (color, headline) = match current {
        Phase::Ready => ("#93c5fd", "Ready"),
        Phase::Running => ("#fbbf24", "Proof in progress"),
        Phase::Proven => ("#4ade80", "Ground graph proven"),
        Phase::Different => ("#fb7185", "Graph difference found"),
        Phase::BlankNodeCaution => ("#fbbf24", "Blank-node caution"),
        Phase::Error => ("#fb7185", "Could not complete proof"),
    };
    let badge_style = format!("border:1px solid {color};color:{color};border-radius:999px;padding:.45rem .8rem;font-size:.82rem;font-weight:700;");
    let button_style = format!(
        "margin-top:1rem;padding:.75rem 1rem;border:0;border-radius:8px;background:{};color:white;font-weight:800;cursor:{};",
        if running { "#334155" } else { "linear-gradient(90deg,#06b6d4,#2563eb)" },
        if running { "wait" } else { "pointer" },
    );

    rsx! {
        section { style: "height:100%;overflow:auto;padding:1.5rem;box-sizing:border-box;background:radial-gradient(circle at top right,rgba(34,211,238,.12),transparent 28rem),linear-gradient(145deg,#101827,#07111d);color:#e5edf7;font-family:Inter,system-ui,sans-serif;",
            div { style: "max-width:1050px;margin:0 auto;display:flex;flex-direction:column;gap:1rem;",
                header { style: "display:flex;justify-content:space-between;gap:1rem;align-items:flex-start;flex-wrap:wrap;",
                    div {
                        div { style: "font-size:.75rem;letter-spacing:.12em;text-transform:uppercase;color:#67e8f9;font-weight:700;", "Qualia integrity workbench" }
                        h1 { style: "font-size:1.65rem;margin:.25rem 0;", "Graph Proof" }
                        p { style: "margin:0;color:#a5b4c7;max-width:48rem;line-height:1.5;", "Exact N-Triples → Q42 graph comparison. Uses a capped disk-backed sort: neither graph is loaded into memory." }
                    }
                    div { style: "{badge_style}", "{headline}" }
                }
                div { style: "display:grid;grid-template-columns:minmax(0,1.5fr) minmax(270px,.75fr);gap:1rem;align-items:start;",
                    div { style: "background:rgba(15,23,42,.78);border:1px solid rgba(148,163,184,.2);border-radius:14px;padding:1.1rem;",
                        h2 { style: "margin:0 0:.85rem;font-size:1rem;", "Proof inputs" }
                        label { style: "display:block;font-size:.8rem;color:#cbd5e1;margin-bottom:.35rem;", "Original N-Triples (.nt)" }
                        input { value: "{source}", disabled: running, placeholder: "C:\\data\\ontology.nt", oninput: move |event| source.set(event.value()), style: "width:100%;box-sizing:border-box;padding:.68rem .75rem;border-radius:8px;border:1px solid rgba(148,163,184,.35);background:#07111d;color:#f8fafc;font-family:ui-monospace,monospace;" }
                        label { style: "display:block;font-size:.8rem;color:#cbd5e1;margin:.85rem 0 .35rem;", "Qualia volume (.q42)" }
                        input { value: "{volume}", disabled: running, placeholder: "C:\\data\\ontology.q42", oninput: move |event| volume.set(event.value()), style: "width:100%;box-sizing:border-box;padding:.68rem .75rem;border-radius:8px;border:1px solid rgba(148,163,184,.35);background:#07111d;color:#f8fafc;font-family:ui-monospace,monospace;" }
                        div { style: "display:grid;grid-template-columns:1fr 1fr;gap:.7rem;margin-top:.85rem;",
                            div { label { style: "display:block;font-size:.8rem;color:#cbd5e1;margin-bottom:.35rem;", "RAM (MiB)" } input { value: "{memory_mib}", disabled: running, inputmode: "numeric", oninput: move |event| memory_mib.set(event.value()), style: "width:100%;box-sizing:border-box;padding:.68rem .75rem;border-radius:8px;border:1px solid rgba(148,163,184,.35);background:#07111d;color:#f8fafc;" } }
                            div { label { style: "display:block;font-size:.8rem;color:#cbd5e1;margin-bottom:.35rem;", "Temporary disk (GiB)" } input { value: "{temp_gib}", disabled: running, inputmode: "numeric", oninput: move |event| temp_gib.set(event.value()), style: "width:100%;box-sizing:border-box;padding:.68rem .75rem;border-radius:8px;border:1px solid rgba(148,163,184,.35);background:#07111d;color:#f8fafc;" } }
                        }
                        button { disabled: running, onclick: start, style: "{button_style}", if running { "Proving…" } else { "Run bounded proof" } }
                    }
                    aside { style: "background:rgba(8,47,73,.34);border:1px solid rgba(34,211,238,.28);border-radius:14px;padding:1.1rem;",
                        h2 { style: "margin:0 0:.6rem;font-size:1rem;", "Proof boundary" }
                        p { style: "margin:.25rem 0;color:#bae6fd;font-size:.87rem;line-height:1.5;", "Ground graphs: exact equivalence in Qualia's canonical Q42 encoding." }
                        p { style: "margin:.8rem 0 0;color:#cbd5e1;font-size:.82rem;line-height:1.5;", "Blank nodes: the result is deliberately cautious because a hash-only volume cannot independently prove a canonical bijection." }
                        p { style: "margin:.8rem 0 0;color:#94a3b8;font-size:.8rem;line-height:1.5;", "Temporary files are automatic, budgeted, and cleaned on success or failure." }
                    }
                }
                if !message.is_empty() {
                    div { style: "padding:.85rem 1rem;border-radius:10px;border-left:4px solid {color};background:rgba(15,23,42,.8);color:#dbeafe;line-height:1.45;", "{message}" }
                }
                if let Some(value) = current_report {
                    div { style: "background:rgba(15,23,42,.78);border:1px solid rgba(148,163,184,.2);border-radius:14px;padding:1.1rem;",
                        h2 { style: "margin:0 0:.85rem;font-size:1rem;", "Evidence report" }
                        div { style: "display:grid;grid-template-columns:repeat(auto-fit,minmax(160px,1fr));gap:.65rem;",
                            StatCard { label: "Source records".to_string(), value: count(value.source_records), danger: false }
                            StatCard { label: "Q42 records".to_string(), value: count(value.q42_records), danger: false }
                            StatCard { label: "Unique source quads".to_string(), value: count(value.source_unique_records), danger: false }
                            StatCard { label: "Unique Q42 quads".to_string(), value: count(value.q42_unique_records), danger: false }
                            StatCard { label: "Missing".to_string(), value: count(value.missing_from_q42), danger: true }
                            StatCard { label: "Unexpected".to_string(), value: count(value.unexpected_in_q42), danger: true }
                        }
                        div { style: "margin-top:.9rem;display:grid;gap:.55rem;font-size:.83rem;",
                            div { span { style: "color:#94a3b8;", "RDF claim: " } strong { style: "color:{color};", "{value.rdf_isomorphism}" } }
                            div { span { style: "color:#94a3b8;", "Skipped source lines: " } "{count(value.source_skipped_lines)}" }
                            if value.first_missing.is_some() { div { style: "font-family:ui-monospace,monospace;color:#fda4af;word-break:break-word;", "First missing: {witness(&value.first_missing)}" } }
                            if value.first_unexpected.is_some() { div { style: "font-family:ui-monospace,monospace;color:#fda4af;word-break:break-word;", "First unexpected: {witness(&value.first_unexpected)}" } }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn StatCard(label: String, value: String, danger: bool) -> Element {
    let color = if danger { "#fda4af" } else { "#e2e8f0" };
    rsx! {
        div { style: "border-radius:9px;background:#0b1728;padding:.7rem;border:1px solid rgba(148,163,184,.12);",
            div { style: "font-size:.72rem;color:#94a3b8;text-transform:uppercase;letter-spacing:.05em;", "{label}" }
            strong { style: "display:block;margin-top:.25rem;font-size:1.05rem;color:{color};", "{value}" }
        }
    }
}
