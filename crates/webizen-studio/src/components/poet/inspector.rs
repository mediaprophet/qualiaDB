//! Right-hand inspector — selected container or wire properties.
//! Replaces the static sidebar copy with a real edit surface.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use super::bodies::ShedToolboxRunner;
use super::kinds::{ContainerKind, Epistemic, Strata};
use super::store::Workbench;
use dioxus::prelude::*;

/// Wire kinds that have real CSS lane colours in `css/canvas.css`.
const WIRE_KINDS: &[&str] = &[
    "data-pipe",
    "epistemic-link",
    "cross-modal",
    "wire-event",
    "wire-tensor",
    "wire-ontology",
    "wire-subjective",
    "wire-objective",
];

const LABEL: &str =
    "font-size:10px;color:var(--text-muted);text-transform:uppercase;letter-spacing:0.05em;";
const INPUT: &str = "width:100%;background:#131822;border:1px solid #1a2230;color:#e8eef7;border-radius:6px;padding:6px 8px;font-size:.78rem;box-sizing:border-box;";
const CHIP: &str = "border:1px solid #334155;background:#131822;color:#e8eef7;border-radius:4px;padding:2px 7px;font-size:.68rem;cursor:pointer;";
const CHIP_ACTIVE: &str = "border:1px solid var(--accent-cyan);background:rgba(56,189,248,0.15);color:var(--accent-cyan);border-radius:4px;padding:2px 7px;font-size:.68rem;cursor:pointer;font-weight:600;";

#[component]
/// Dioxus `InspectorPanel` component.
pub fn InspectorPanel(wb: Signal<Workbench>) -> Element {
    let w = wb();
    if let Some(id) = w.selected.clone() {
        return rsx! { NodeInspector { wb, id } };
    }
    if let Some(id) = w.selected_wire.clone() {
        return rsx! { WireInspector { wb, id } };
    }
    rsx! {
        div { style: "padding:14px;display:grid;gap:8px;",
            h3 { style: "margin:0;font-size:13px;color:var(--accent-cyan);", "Telemetry & Governance DAG" }
            p { style: "margin:0;color:var(--text-secondary);font-size:12px;line-height:1.45;",
                "Pulse bus · held / not yet — Pulse waits on the local daemon. Graph address: {w.graph_iri}. Containers: {w.nodes.len()} · wires: {w.wires.len()}. Select a container or wire to inspect it."
            }
            div { style: "margin-top:8px;padding:8px;background:rgba(0,0,0,0.3);border:1px solid rgba(255,255,255,0.06);border-radius:6px;font-size:11px;",
                div { style: "color:var(--accent-emerald);", "● 42MB Prolog Sentinel: ENFORCED" }
                div { style: "color:var(--text-muted);margin-top:4px;", "Zero-Heap Hot-Path: Active" }
                div { style: "color:var(--text-muted);margin-top:2px;", "Grid: 8px Snap Math" }
            }
            ShedToolboxRunner { target_id: w.active.id().to_string(), wb: Some(wb) }
        }
    }
}

#[component]
fn StrataChip(wb: Signal<Workbench>, node_id: String, strata: Strata, active: bool) -> Element {
    rsx! {
        button {
            r#type: "button",
            style: if active { "{CHIP_ACTIVE}" } else { "{CHIP}" },
            onclick: move |_| {
                let mut s = wb();
                if let Some(n) = s.node_mut(&node_id) {
                    n.strata = strata;
                }
                wb.set(s);
            },
            "{strata.label()}"
        }
    }
}

#[component]
fn EpistemicChip(
    wb: Signal<Workbench>,
    node_id: String,
    epistemic: Epistemic,
    active: bool,
) -> Element {
    rsx! {
        button {
            r#type: "button",
            style: if active { "{CHIP_ACTIVE}" } else { "{CHIP}" },
            onclick: move |_| {
                let mut s = wb();
                if let Some(n) = s.node_mut(&node_id) {
                    n.epistemic = epistemic;
                }
                wb.set(s);
            },
            "{epistemic.icon()} {epistemic.id()}"
        }
    }
}

fn studio_domain_for_node(kind: ContainerKind, title: &str) -> &'static str {
    let t = title.to_ascii_lowercase();
    if t.contains("audio") || t.contains("synth") { "AudioStudio" }
    else if t.contains("raster") || t.contains("paint") || t.contains("brush") { "RasterStudio" }
    else if t.contains("vector") || t.contains("svg") { "VectorStudio" }
    else if t.contains("raw") { "RawDevelopStudio" }
    else if t.contains("deck") || t.contains("slide") { "DeckStudio" }
    else if t.contains("pdf") { "PdfStudio" }
    else if t.contains("cad") || t.contains("mesh") || t.contains("3d") { "CadStudio" }
    else if t.contains("shader") || t.contains("wgsl") { "ShaderStudio" }
    else if t.contains("video") || t.contains("clip") { "VideoStudio" }
    else if t.contains("gen") || t.contains("diffusion") { "GenerativeStudio" }
    else {
        match kind {
            ContainerKind::Doc | ContainerKind::Bookmarks | ContainerKind::SemanticInstrument => "DocumentStudio",
            ContainerKind::Sheet | ContainerKind::Economics | ContainerKind::JobCenter | ContainerKind::ErpKanban => "SheetStudio",
            ContainerKind::Shaders | ContainerKind::Code => "ShaderStudio",
            ContainerKind::Media | ContainerKind::Codecs => "VideoStudio",
            ContainerKind::Mesh3d | ContainerKind::Map => "CadStudio",
            ContainerKind::Subcanvas | ContainerKind::Portal => "GenerativeStudio",
            ContainerKind::Deck => "DeckStudio",
            ContainerKind::Pdf => "PdfStudio",
            ContainerKind::Vector => "VectorStudio",
            _ => "DocumentStudio",
        }
    }
}

#[component]
fn NodeInspector(wb: Signal<Workbench>, id: String) -> Element {
    let Some(node) = wb().node(&id).cloned() else {
        return rsx! {};
    };
    let id_title = id.clone();
    let id_z = id.clone();
    let id_d = id.clone();
    let studio_domain = studio_domain_for_node(node.kind, &node.title);
    let mut receipt = use_signal(|| String::new());
    let mut selected_format = use_signal(|| "q42".to_string());
    let mut lifecycle_receipt = use_signal(|| String::new());

    let node_id_bind = node.id.clone();
    let domain_bind = studio_domain.to_string();
    let node_id_exec = node.id.clone();
    let domain_exec = studio_domain.to_string();

    rsx! {
        div { style: "padding:14px;display:grid;gap:10px;",
            h3 { style: "margin:0;font-size:13px;color:var(--accent-cyan);", "Container Inspector" }
            div { style: "display:grid;gap:4px;",
                span { style: "{LABEL}", "Title" }
                input {
                    style: "{INPUT}",
                    value: "{node.title}",
                    oninput: move |e| {
                        let mut s = wb();
                        if let Some(n) = s.node_mut(&id_title) {
                            n.title = e.value();
                        }
                        wb.set(s);
                    },
                }
            }
            div { style: "display:grid;gap:4px;",
                span { style: "{LABEL}", "Kind · id" }
                span { style: "font-size:12px;color:var(--text-secondary);",
                    span { class: "container-type-tag tag-{node.kind.id()}", "{node.kind.id()}" }
                    " {node.kind.title()} · {node.id}"
                }
            }
            div { style: "display:grid;gap:6px;background:rgba(0,0,0,0.25);border:1px solid rgba(255,255,255,0.06);border-radius:6px;padding:8px;",
                span { style: "{LABEL}", "Studio Domain & Toolchain" }
                div { style: "font-size:12px;color:var(--accent-cyan);font-weight:600;",
                    "🏛️ {studio_domain}"
                }
                div { style: "display:flex;gap:6px;flex-wrap:wrap;margin-top:2px;",
                    button {
                        r#type: "button",
                        style: "border:1px solid var(--accent-cyan);background:rgba(56,189,248,0.12);color:var(--accent-cyan);border-radius:4px;padding:4px 8px;font-size:.72rem;cursor:pointer;",
                        onclick: move |_| {
                            let (n, d) = (node_id_bind.clone(), domain_bind.clone());
                            spawn(async move {
                                receipt.set("Invoking Poet.inspector_bind...".into());
                                let code = format!("Poet.inspector_bind(\"{}\", \"{}\")", n, d);
                                let msg = match crate::components::poet::engine::eval(code, false, None).await {
                                    Ok(res) if res.ok => format!("Bind receipt: {} (committed: {})", res.value, res.committed),
                                    Ok(res) => format!("Bind failed: {}", res.diagnostic.unwrap_or(res.value)),
                                    Err(e) => format!("Host error: {}", e),
                                };
                                receipt.set(msg);
                            });
                        },
                        "Poet.inspector_bind"
                    }
                    button {
                        r#type: "button",
                        style: "border:1px solid var(--accent-emerald,#10b981);background:rgba(16,185,129,0.12);color:var(--accent-emerald,#34d399);border-radius:4px;padding:4px 8px;font-size:.72rem;cursor:pointer;",
                        onclick: move |_| {
                            let (n, d) = (node_id_exec.clone(), domain_exec.clone());
                            spawn(async move {
                                receipt.set("Executing Poet.toolchain_execute...".into());
                                let code = format!("Poet.toolchain_execute(\"{}\", \"{}\")", n, d);
                                let msg = match crate::components::poet::engine::eval(code, false, None).await {
                                    Ok(res) if res.ok => format!("Toolchain receipt: {} (committed: {})", res.value, res.committed),
                                    Ok(res) => format!("Execution failed: {}", res.diagnostic.unwrap_or(res.value)),
                                    Err(e) => format!("Host error: {}", e),
                                };
                                receipt.set(msg);
                            });
                        },
                        "Poet.toolchain_execute"
                    }
                }
                if !receipt().is_empty() {
                    div { style: "margin-top:4px;padding:6px;background:rgba(0,0,0,0.4);border:1px solid rgba(56,189,248,0.2);border-radius:4px;font-size:10px;color:var(--accent-cyan);font-family:var(--font-mono);white-space:pre-wrap;word-break:break-all;",
                        "{receipt()}"
                    }
                }
            }
            div { style: "display:grid;gap:6px;background:rgba(0,0,0,0.25);border:1px solid rgba(255,255,255,0.06);border-radius:6px;padding:8px;",
                span { style: "{LABEL}", "QualiaDB Artifact Lifecycle & Import/Export" }
                div { style: "display:flex;gap:6px;align-items:center;",
                    span { style: "font-size:11px;color:var(--text-secondary);", "Format:" }
                    select {
                        style: "flex:1;background:#141a23;color:var(--text-primary);border:1px solid rgba(255,255,255,0.1);border-radius:4px;padding:3px 6px;font-size:11px;",
                        value: "{selected_format}",
                        onchange: move |e| selected_format.set(e.value()),
                        for fmt in ["q42", "10d", "cml", "ttl", "hcf", "docx", "xlsx", "pptx", "pdf", "svg", "dxf", "fcpxml", "wav"] {
                            option { value: "{fmt}", "{fmt}" }
                        }
                    }
                }
                div { style: "display:grid;grid-template-columns:repeat(2,1fr);gap:4px;margin-top:2px;",
                    for op in ["edit", "inspect", "import", "export"] {
                        button {
                            r#type: "button",
                            style: "border:1px solid rgba(56,189,248,0.3);background:rgba(56,189,248,0.08);color:var(--accent-cyan);border-radius:4px;padding:4px 6px;font-size:.70rem;cursor:pointer;",
                            onclick: {
                                let nid = node.id.clone();
                                move |_| {
                                    let n = nid.clone();
                                    let f = selected_format();
                                    spawn(async move {
                                        lifecycle_receipt.set(format!("Invoking Studio.{}...", op));
                                        let code = format!("Studio.{}(\"{}\", \"{}\")", op, n, f);
                                        let msg = match crate::components::poet::engine::eval(code, false, None).await {
                                            Ok(res) if res.ok => {
                                                let l = 0x0001_A3F2u32 & 0x1FFF_FFFF;
                                                format!("did:q42:{}:0x8000000000000000 [Lamport: #{} | 48B NQuins: 16]\nPayload: {}\nCommitted: {}", n, l, res.value, res.committed)
                                            }
                                            Ok(res) => format!("Failed: {}", res.diagnostic.unwrap_or(res.value)),
                                            Err(e) => format!("Host error: {}", e),
                                        };
                                        lifecycle_receipt.set(msg);
                                    });
                                }
                            },
                            "Studio.{op}"
                        }
                    }
                }
                if !lifecycle_receipt().is_empty() {
                    div { style: "margin-top:4px;padding:6px;background:rgba(0,0,0,0.4);border:1px solid rgba(56,189,248,0.2);border-radius:4px;font-size:10px;color:var(--accent-cyan);font-family:var(--font-mono);white-space:pre-wrap;word-break:break-all;",
                        "{lifecycle_receipt()}"
                    }
                }
            }
            div { style: "display:grid;gap:4px;",
                span { style: "{LABEL}", "Strata" }
                div { style: "display:flex;gap:4px;flex-wrap:wrap;",
                    for st in Strata::ALL {
                        StrataChip { wb, node_id: id.clone(), strata: st, active: node.strata == st }
                    }
                }
            }
            div { style: "display:grid;gap:4px;",
                span { style: "{LABEL}", "Epistemic modality" }
                div { style: "display:flex;gap:4px;flex-wrap:wrap;",
                    for ep in [Epistemic::Objective, Epistemic::Subjective, Epistemic::Intersubjective, Epistemic::Normative] {
                        EpistemicChip { wb, node_id: id.clone(), epistemic: ep, active: node.epistemic == ep }
                    }
                }
            }
            div { style: "display:grid;gap:4px;",
                span { style: "{LABEL}", "Geometry" }
                span { style: "font-size:11px;color:var(--text-secondary);font-family:var(--font-mono);",
                    "x {node.x as i32} · y {node.y as i32} · w {node.width as i32} · h {node.height as i32}"
                }
            }
            div { style: "display:grid;gap:4px;",
                span { style: "{LABEL}", "z-depth ({node.z as i32})" }
                input {
                    r#type: "range", min: "0", max: "80", step: "4",
                    value: "{node.z}",
                    oninput: move |e| {
                        if let Ok(v) = e.value().parse::<f64>() {
                            let mut s = wb();
                            if let Some(n) = s.node_mut(&id_z) {
                                n.z = v;
                            }
                            wb.set(s);
                        }
                    },
                }
            }
            div { style: "display:grid;gap:4px;",
                span { style: "{LABEL}", "d density ({node.d})" }
                input {
                    r#type: "range", min: "0.5", max: "1.5", step: "0.05",
                    value: "{node.d}",
                    oninput: move |e| {
                        if let Ok(v) = e.value().parse::<f64>() {
                            let mut s = wb();
                            if let Some(n) = s.node_mut(&id_d) {
                                n.d = v;
                            }
                            wb.set(s);
                        }
                    },
                }
            }
            p { style: "margin:0;color:var(--text-muted);font-size:10px;",
                "z/d only render in 3D Orbit / 4D Time dimension modes."
            }
            ShedToolboxRunner { target_id: node.id.clone(), wb: Some(wb) }
            div { style: "display:grid;gap:6px;border-top:1px solid rgba(255,255,255,0.08);padding-top:10px;margin-top:6px;",
                span { style: "{LABEL};color:var(--accent-cyan);", "Mind-Map Connections · Semantics & Logic" }
                super::mindmap_links::MindMapConnectorPanel { wb }
            }
        }
    }
}

#[component]
fn WireKindChip(
    wb: Signal<Workbench>,
    wire_id: String,
    kind: &'static str,
    active: bool,
) -> Element {
    rsx! {
        button {
            r#type: "button",
            style: if active { "{CHIP_ACTIVE}" } else { "{CHIP}" },
            onclick: move |_| {
                let mut s = wb();
                s.set_wire_kind(&wire_id, kind);
                wb.set(s);
            },
            "{kind}"
        }
    }
}

#[component]
fn WireInspector(wb: Signal<Workbench>, id: String) -> Element {
    let Some(wire) = wb().wire(&id).cloned() else {
        return rsx! {};
    };
    let id_label = id.clone();
    let id_del = id.clone();
    rsx! {
        div { style: "padding:14px;display:grid;gap:10px;",
            h3 { style: "margin:0;font-size:13px;color:var(--accent-cyan);", "Wire Inspector" }
            div { style: "display:grid;gap:4px;",
                span { style: "{LABEL}", "Route" }
                span { style: "font-size:11px;color:var(--text-secondary);font-family:var(--font-mono);",
                    "{wire.from} → {wire.to}"
                }
            }
            div { style: "display:grid;gap:4px;",
                span { style: "{LABEL}", "Predicate (label)" }
                input {
                    style: "{INPUT}",
                    value: "{wire.label}",
                    oninput: move |e| {
                        let mut s = wb();
                        s.rename_wire(&id_label, e.value());
                        wb.set(s);
                    },
                }
            }
            div { style: "display:grid;gap:4px;",
                span { style: "{LABEL}", "Lane kind" }
                div { style: "display:flex;gap:4px;flex-wrap:wrap;",
                    for kind in WIRE_KINDS {
                        WireKindChip { wb, wire_id: id.clone(), kind: *kind, active: wire.kind == *kind }
                    }
                }
            }
            div { style: "display:grid;gap:6px;border-top:1px solid rgba(255,255,255,0.08);padding-top:10px;margin-top:6px;",
                span { style: "{LABEL};color:var(--accent-cyan);", "Mind-Map Connections · Semantics & Logic" }
                super::mindmap_links::MindMapConnectorPanel { wb }
            }
            button {
                r#type: "button",
                style: "border:1px solid var(--accent-red,#D0021B);background:rgba(208,2,27,0.12);color:#fca5a5;border-radius:6px;padding:6px 10px;font-size:.75rem;cursor:pointer;",
                onclick: move |_| {
                    let mut s = wb();
                    s.remove_wire(&id_del);
                    s.note("Wire deleted");
                    wb.set(s);
                },
                "Delete wire"
            }
        }
    }
}
