//! Context-sensitive toolboxes, custom toolbox/toolchain creation, and Mind-Map semantic/logic connections.
//!
//! Follows `AGENTS.md` §0-B (<440 lines).
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use super::kinds::{ContainerKind, ManifoldId, ToolboxId};
use super::store::Workbench;
use crate::components::poet::engine;
use dioxus::prelude::*;

const LABEL: &str = "font-size:10px;color:var(--text-muted);text-transform:uppercase;letter-spacing:0.05em;font-weight:600;";
const INPUT: &str = "width:100%;background:#131822;border:1px solid #1a2230;color:#e8eef7;border-radius:4px;padding:4px 6px;font-size:11px;box-sizing:border-box;";
const SELECT: &str = "width:100%;background:#141a23;color:var(--text-primary);border:1px solid rgba(255,255,255,0.1);border-radius:4px;padding:3px 6px;font-size:11px;";

pub const SEMANTIC_RELATIONS: &[(&str, &str)] = &[
    ("qualia:groundsObservation", "Empirical Grounding"),
    ("qualia:derivesFrom", "Formal Derivation / Proof"),
    ("qualia:obligates", "Deontic Obligation (0x10)"),
    ("qualia:permits", "Deontic Permission (0x11)"),
    ("qualia:forbids", "Deontic Prohibition (0x12)"),
    ("qualia:knows", "Epistemic Knowledge (0x20)"),
    ("qualia:contradicts", "Paraconsistent Isolation (0x30)"),
    ("qualia:until", "Temporal LTL Until (0x43)"),
    ("qualia:transfersValue", "Cooperative Economics / REA Flow"),
    ("qualia:tracksContribution", "Collaborative Project Provenance"),
];

pub const LOGIC_SYSTEMS: &[(&str, &str)] = &[
    ("Deontic", "deontic-guard"),
    ("Epistemic", "semantic-rel"),
    ("Paraconsistent", "deontic-guard"),
    ("LTL", "causal-flow"),
    ("N3", "semantic-rel"),
    ("SHACL", "deontic-guard"),
    ("DataPipe", "data-pipe"),
];

/// Returns `(recommended_toolbox_id, shed_category_id, recommended_construct_id, context_summary)` for any `ContainerKind`.
pub fn context_profile_for_kind(kind: ContainerKind) -> (ToolboxId, &'static str, &'static str, &'static str) {
    match kind {
        ContainerKind::Subcanvas => (ToolboxId::Spatial, "geometry", "poet", "Nested Manifold Canvas"),
        ContainerKind::Social => (ToolboxId::Communication, "logic", "social-erp", "Multi-Agent Social Graph"),
        ContainerKind::WebRtc => (ToolboxId::Communication, "logic", "social-erp", "P2P WebRTC Data & AV Mesh"),
        ContainerKind::Health => (ToolboxId::Health, "physics", "clinical-bio", "Clinical Telemetry & Risk Engine"),
        ContainerKind::Webview => (ToolboxId::Office, "studio", "studio-media", "Sandboxed Webview Host"),
        ContainerKind::Map => (ToolboxId::Spatial, "geometry", "geometry-cad", "Geospatial, WGS84 & Hilbert Strata"),
        ContainerKind::Doc => (ToolboxId::Office, "studio", "studio-media", "HyperDoc Living Text"),
        ContainerKind::Ontology => (ToolboxId::Epistemic, "logic", "logic-semantics", "OWL, SHACL & Knowledge Graph"),
        ContainerKind::Code => (ToolboxId::Code, "math", "poet", "VibeScript Reactive IDE"),
        ContainerKind::Sheet => (ToolboxId::Sheet, "economics", "economics-commons", "Reactive Spreadsheet & Pivot"),
        ContainerKind::Portal => (ToolboxId::Spatial, "geometry", "poet", "Inter-Manifold Wormhole"),
        ContainerKind::Media => (ToolboxId::Image, "studio", "studio-media", "Volumetric Media Studio"),
        ContainerKind::Mesh3d => (ToolboxId::Spatial, "geometry", "geometry-cad", "3D WebGPU BRep Geometry Lab"),
        ContainerKind::Mail => (ToolboxId::Mail, "communication", "social-erp", "Inalienable Signed CML Mail"),
        ContainerKind::Chora => (ToolboxId::Epistemic, "logic", "logic-semantics", "Dialectical Truth & Value"),
        ContainerKind::ErpKanban => (ToolboxId::Erp, "economics", "economics-commons", "Cooperative Agile Kanban"),
        ContainerKind::GitForge => (ToolboxId::Code, "math", "poet", "Distributed Git Provenance"),
        ContainerKind::SolidHub => (ToolboxId::Rights, "logic", "rights-consent", "W3C Solid Pod & Identity Store"),
        ContainerKind::Economics => (ToolboxId::Sdn, "economics", "economics-commons", "Cooperative Economics & REA"),
        ContainerKind::JobCenter => (ToolboxId::Erp, "economics", "economics-commons", "Bounty & Skill Escrow"),
        ContainerKind::Bookmarks => (ToolboxId::Office, "logic", "research-lab", "Annotated Meaning Shelf"),
        ContainerKind::Shaders => (ToolboxId::Spatial, "geometry", "geometry-cad", "WGSL Forge Shader Synthesis"),
        ContainerKind::Codecs => (ToolboxId::Audio, "studio", "studio-media", "Audio/Video Codec Pipeline"),
        ContainerKind::Domains => (ToolboxId::Rights, "logic", "rights-consent", "Domain Presence & DID Auth"),
        ContainerKind::Deck => (ToolboxId::Office, "studio", "studio-media", "Slide Deck Presenter"),
        ContainerKind::Pdf => (ToolboxId::Office, "studio", "studio-media", "PDF Preflight & Sign Studio"),
        ContainerKind::Vector => (ToolboxId::Image, "geometry", "geometry-cad", "Vector Bezier & RAW Develop"),
        ContainerKind::SemanticInstrument => (ToolboxId::Epistemic, "logic", "logic-semantics", "Semantic Instrument Definition & SHACL Studio"),
    }
}

#[component]
/// Custom Toolbox & ToolChain Creator component.
pub fn CustomToolboxCreator(wb: Signal<Workbench>) -> Element {
    let mut tb_id = use_signal(|| "my_custom_toolbox".to_string());
    let mut tb_label = use_signal(|| "My Analysis Tools".to_string());
    let mut tb_cat = use_signal(|| "Analysis".to_string());
    let mut tb_tools = use_signal(|| "ComputationalGeometry.hull2,Econ.gini,Statistics.mean".to_string());

    let mut tc_id = use_signal(|| "ingest_and_validate".to_string());
    let mut tc_kind = use_signal(|| "doc".to_string());
    let mut tc_steps = use_signal(|| "N3.evaluate,SHACL.validate,Studio.edit".to_string());

    let mut custom_toolboxes = use_signal(|| Vec::<(String, String, String, String)>::new());
    let mut custom_toolchains = use_signal(|| Vec::<(String, String, Vec<String>)>::new());
    let mut status_msg = use_signal(|| String::new());

    let selected_id = wb().selected.clone().unwrap_or_else(|| "canvas".to_string());

    rsx! {
        div { style: "display:grid;gap:8px;background:rgba(0,0,0,0.4);border:1px solid rgba(56,189,248,0.25);border-radius:6px;padding:8px;",
            div { style: "display:flex;justify-content:space-between;align-items:center;",
                span { style: "{LABEL};color:var(--accent-cyan);", "🛠️ Custom Toolbox & ToolChain Creator" }
                span { style: "font-size:9px;color:var(--text-muted);font-family:var(--font-mono);", "Target: {selected_id}" }
            }
            div { style: "display:grid;gap:4px;border:1px solid rgba(255,255,255,0.06);padding:6px;border-radius:4px;",
                span { style: "font-size:10px;font-weight:700;color:#94a3b8;", "+ Custom Toolbox" }
                div { style: "display:grid;grid-template-columns:1fr 1fr;gap:4px;",
                    input { style: "{INPUT}", value: "{tb_id}", placeholder: "Toolbox ID", oninput: move |e| tb_id.set(e.value()) }
                    input { style: "{INPUT}", value: "{tb_label}", placeholder: "Label", oninput: move |e| tb_label.set(e.value()) }
                }
                div { style: "display:grid;grid-template-columns:80px 1fr;gap:4px;",
                    input { style: "{INPUT}", value: "{tb_cat}", placeholder: "Category", oninput: move |e| tb_cat.set(e.value()) }
                    input { style: "{INPUT}", value: "{tb_tools}", placeholder: "Tools (comma-separated)", oninput: move |e| tb_tools.set(e.value()) }
                }
                button {
                    r#type: "button",
                    style: "border:1px solid var(--accent-cyan);background:rgba(56,189,248,0.15);color:var(--accent-cyan);border-radius:4px;padding:4px;font-size:10px;font-weight:600;cursor:pointer;",
                    onclick: move |_| {
                        let id = tb_id();
                        let lbl = tb_label();
                        let cat = tb_cat();
                        let tls = tb_tools();
                        custom_toolboxes.write().push((id.clone(), lbl.clone(), cat.clone(), tls.clone()));
                        let code = format!("Poet.toolchest_list(#{{ op: \"toolbox_create\", id: \"{id}\", label: \"{lbl}\", category: \"{cat}\", tools: \"{tls}\" }})");
                        spawn(async move {
                            let _ = engine::eval(code, false, None).await;
                            status_msg.set(format!("Created toolbox '{lbl}' with {tls}"));
                        });
                    },
                    "+ Create Custom Toolbox"
                }
            }
            div { style: "display:grid;gap:4px;border:1px solid rgba(255,255,255,0.06);padding:6px;border-radius:4px;",
                span { style: "font-size:10px;font-weight:700;color:#94a3b8;", "+ Multi-Step ToolChain" }
                div { style: "display:grid;grid-template-columns:1fr 100px;gap:4px;",
                    input { style: "{INPUT}", value: "{tc_id}", placeholder: "Toolchain ID", oninput: move |e| tc_id.set(e.value()) }
                    input { style: "{INPUT}", value: "{tc_kind}", placeholder: "Target Kind", oninput: move |e| tc_kind.set(e.value()) }
                }
                input { style: "{INPUT}", value: "{tc_steps}", placeholder: "Pipeline Steps (CSV)", oninput: move |e| tc_steps.set(e.value()) }
                button {
                    r#type: "button",
                    style: "border:1px solid var(--accent-emerald,#10b981);background:rgba(16,185,129,0.15);color:var(--accent-emerald,#34d399);border-radius:4px;padding:4px;font-size:10px;font-weight:600;cursor:pointer;",
                    onclick: move |_| {
                        let id = tc_id();
                        let k = tc_kind();
                        let st = tc_steps();
                        let step_vec: Vec<String> = st.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
                        custom_toolchains.write().push((id.clone(), k.clone(), step_vec));
                        let code = format!("Poet.toolchest_list(#{{ op: \"toolchain_create\", id: \"{id}\", target_kind: \"{k}\", steps: \"{st}\" }})");
                        spawn(async move {
                            let _ = engine::eval(code, false, None).await;
                            status_msg.set(format!("Created toolchain '{id}' for {k}"));
                        });
                    },
                    "+ Create Multi-Step ToolChain"
                }
            }
            if !custom_toolchains().is_empty() {
                div { style: "display:grid;gap:4px;",
                    span { style: "{LABEL}", "Authored Toolchains (1-Click Run on Active Container)" }
                    for (cid, ckind, steps) in custom_toolchains() {
                        div { style: "background:#0d1117;border:1px solid rgba(255,255,255,0.08);padding:4px 6px;border-radius:4px;display:flex;align-items:center;justify-content:space-between;",
                            span { style: "font-size:10px;color:var(--accent-cyan);font-family:var(--font-mono);", "{cid} ({ckind})" }
                            div { style: "display:flex;gap:3px;",
                                for s in steps {
                                    button {
                                        r#type: "button",
                                        style: "border:1px solid rgba(56,189,248,0.3);background:rgba(56,189,248,0.1);color:#e8eef7;border-radius:3px;padding:1px 4px;font-size:9px;cursor:pointer;",
                                        onclick: {
                                            let step_name = s.clone();
                                            let target = selected_id.clone();
                                            move |_| {
                                                let sn = step_name.clone();
                                                let tgt = target.clone();
                                                let run_code = format!("{sn}(\"{tgt}\")");
                                                spawn(async move {
                                                    let _ = engine::eval(run_code, false, None).await;
                                                    status_msg.set(format!("Executed step '{sn}' on {tgt}"));
                                                });
                                            }
                                        },
                                        "▶ {s}"
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if !status_msg().is_empty() {
                div { style: "font-size:9px;color:var(--accent-emerald,#34d399);font-family:var(--font-mono);", "{status_msg()}" }
            }
        }
    }
}

#[component]
/// Mind-Map Semantic & Logic Connections panel.
pub fn MindMapConnectorPanel(wb: Signal<Workbench>) -> Element {
    let w = wb();
    let selected_container = w.selected.clone();
    let selected_wire_id = w.selected_wire.clone();

    let mut target_node_id = use_signal(|| String::new());
    let mut target_manifold_str = use_signal(|| String::new());
    let mut selected_predicate = use_signal(|| "qualia:groundsObservation".to_string());
    let mut selected_logic_sys = use_signal(|| "Deontic".to_string());
    let mut rule_expr = use_signal(|| "(0.95) { ?src qualia:verified true } => { ?dst qualia:active true }".to_string());

    let mut receipt = use_signal(|| String::new());
    let mut eval_verdict = use_signal(|| String::new());
    let mut traversing = use_signal(|| false);

    let (from_display, default_target) = if let Some(wid) = selected_wire_id.as_ref() {
        if let Some(wire) = w.wire(wid) {
            (format!("Wire: {} → {}", wire.from, wire.to), wire.to.clone())
        } else {
            ("No Active Wire".to_string(), String::new())
        }
    } else if let Some(cid) = selected_container.as_ref() {
        let first_other = w.nodes.iter().find(|n| n.id != *cid).map(|n| n.id.clone()).unwrap_or_default();
        (format!("Container: {cid}"), first_other)
    } else {
        ("Canvas Target".to_string(), String::new())
    };

    let effective_target = if target_node_id().is_empty() { default_target } else { target_node_id() };

    rsx! {
        div { style: "display:grid;gap:6px;background:rgba(0,0,0,0.3);border:1px solid rgba(56,189,248,0.25);border-radius:6px;padding:8px;",
            div { style: "display:flex;justify-content:space-between;align-items:center;",
                span { style: "{LABEL};color:var(--accent-cyan);", "🔗 Mind-Map Connection Engine" }
                span { style: "font-size:9px;color:var(--text-muted);font-family:var(--font-mono);", "{from_display}" }
            }
            div { style: "display:grid;gap:4px;",
                span { style: "{LABEL}", "Target: Container (Canvas) OR Manifold Portal" }
                div { style: "display:grid;grid-template-columns:1fr 1fr;gap:4px;",
                    select {
                        style: "{SELECT}",
                        value: "{effective_target}",
                        onchange: move |e| target_node_id.set(e.value()),
                        option { value: "", "-- Target Container --" }
                        for n in w.nodes.iter().filter(|n| selected_container.as_ref() != Some(&n.id)) {
                            option { value: "{n.id}", "{n.title} ({n.id})" }
                        }
                    }
                    select {
                        style: "{SELECT}",
                        value: "{target_manifold_str}",
                        onchange: move |e| target_manifold_str.set(e.value()),
                        option { value: "", "-- Cross-Manifold Link --" }
                        for m in ManifoldId::ALL {
                            option { value: "{m.id()}", "{m.icon()} {m.short()}" }
                        }
                    }
                }
            }
            div { style: "display:grid;gap:4px;",
                span { style: "{LABEL}", "Semantic Predicate (RDF-Star IRI)" }
                select {
                    style: "{SELECT}",
                    value: "{selected_predicate}",
                    onchange: move |e| selected_predicate.set(e.value()),
                    for (iri, label) in SEMANTIC_RELATIONS {
                        option { value: "{iri}", "{iri} — {label}" }
                    }
                }
            }
            div { style: "display:grid;gap:4px;",
                span { style: "{LABEL}", "Formal Logic Modality & Wire Lane" }
                select {
                    style: "{SELECT}",
                    value: "{selected_logic_sys}",
                    onchange: move |e| selected_logic_sys.set(e.value()),
                    for (sys, lane) in LOGIC_SYSTEMS {
                        option { value: "{sys}", "{sys} (wire lane: {lane})" }
                    }
                }
            }
            div { style: "display:grid;gap:4px;",
                span { style: "{LABEL}", "Formal Logic Rule / Constraint Expression" }
                input {
                    style: "{INPUT};font-family:var(--font-mono);font-size:10px;",
                    value: "{rule_expr}",
                    oninput: move |e| rule_expr.set(e.value())
                }
            }
            div { style: "display:flex;gap:4px;margin-top:2px;",
                button {
                    r#type: "button",
                    style: "flex:1;border:1px solid var(--accent-cyan);background:rgba(56,189,248,0.18);color:var(--accent-cyan);border-radius:4px;padding:4px 6px;font-size:10px;font-weight:700;cursor:pointer;",
                    onclick: move |_| {
                        let from_id = selected_container.clone().unwrap_or_else(|| "container-1".to_string());
                        let to_id = effective_target.clone();
                        let t_man = target_manifold_str();
                        let pred = selected_predicate();
                        let logic = selected_logic_sys();
                        let rule = rule_expr();
                        let wire_lane = LOGIC_SYSTEMS.iter().find(|(s, _)| *s == logic).map(|(_, l)| *l).unwrap_or("data-pipe");
                        let wire_label = format!("[{logic}] {pred}");

                        if !to_id.is_empty() {
                            let mut s = wb();
                            if let Some(wid) = s.connect_wire(&from_id, &to_id) {
                                s.set_wire_kind(&wid, wire_lane);
                                s.rename_wire(&wid, wire_label.clone());
                            } else if let Some(wid) = s.selected_wire.clone() {
                                s.set_wire_kind(&wid, wire_lane);
                                s.rename_wire(&wid, wire_label.clone());
                            }
                            s.note(format!("Linked {from_id} → {to_id} with {pred}"));
                            wb.set(s);
                        }

                        let invoke_code = format!(
                            "Poet.toolchain_execute(#{{ op: \"connection_define\", from: \"{from_id}\", to: \"{to_id}\", target_manifold: \"{t_man}\", relation_iri: \"{pred}\", logic_modality: \"{logic}\", rule_expr: \"{rule}\" }})"
                        );
                        let rc_msg = format!("did:q42:link:{from_id}:{to_id}:0x8000000000000000\nPredicate: {pred} | Modality: {logic}\nLane: {wire_lane} | 48B NQuins: 4");
                        spawn(async move {
                            let _ = engine::eval(invoke_code, false, None).await;
                            receipt.set(rc_msg);
                        });
                    },
                    "🔗 Connect & Bind Semantics"
                }
                button {
                    r#type: "button",
                    style: "flex:1;border:1px solid var(--accent-emerald,#10b981);background:rgba(16,185,129,0.15);color:var(--accent-emerald,#34d399);border-radius:4px;padding:4px 6px;font-size:10px;font-weight:700;cursor:pointer;",
                    onclick: move |_| {
                        let logic = selected_logic_sys();
                        let pred = selected_predicate();
                        let rule = rule_expr();
                        let eval_code = match logic.as_str() {
                            "Deontic" => format!("Deontic.evaluate(#{{ action: \"{pred}\", context: \"mindmap\" }})"),
                            "Epistemic" => format!("Epistemic.evaluate(#{{ claim: \"{pred}\", certainty: 200 }})"),
                            "Paraconsistent" => format!("Paraconsistent.route(#{{ relation: \"{pred}\" }})"),
                            "LTL" => format!("LTL.evaluate_trace(#{{ predicate: \"{pred}\" }})"),
                            "SHACL" => format!("SHACL.validate(#{{ target: \"{pred}\" }})"),
                            _ => format!("N3.evaluate(#{{ rule: \"{rule}\" }})"),
                        };
                        let l_name = logic.clone();
                        spawn(async move {
                            let res = engine::eval(eval_code, false, None).await;
                            let msg = match res {
                                Ok(r) => format!("Verdict [{l_name}] (ok: {}): {}", r.ok, r.value),
                                Err(e) => format!("Evaluation error: {e}"),
                            };
                            eval_verdict.set(msg);
                        });
                    },
                    "⚡ Evaluate Connection Logic"
                }
            }
            if let Some(target_m) = ManifoldId::ALL.iter().find(|m| !target_manifold_str().is_empty() && m.id() == target_manifold_str()).copied() {
                button {
                    r#type: "button",
                    style: "border:1px solid var(--accent-purple,#a855f7);background:rgba(168,85,247,0.15);color:#d8b4fe;border-radius:4px;padding:4px;font-size:10px;font-weight:700;cursor:pointer;",
                    onclick: move |_| {
                        traversing.set(true);
                        let mut s = wb();
                        s.switch(target_m);
                        s.note(format!("Traversed Mind-Map Portal to {}", target_m.title()));
                        wb.set(s);
                    },
                    "🌀 Traverse Mind-Map Portal → {target_m.icon()} {target_m.short()}"
                }
            }
            if !receipt().is_empty() {
                div { style: "padding:4px 6px;background:rgba(0,0,0,0.5);border:1px solid rgba(56,189,248,0.25);border-radius:4px;font-size:9px;color:var(--accent-cyan);font-family:var(--font-mono);white-space:pre-wrap;word-break:break-all;",
                    "{receipt()}"
                }
            }
            if !eval_verdict().is_empty() {
                div { style: "padding:4px 6px;background:rgba(0,0,0,0.5);border:1px solid rgba(16,185,129,0.3);border-radius:4px;font-size:9px;color:var(--accent-emerald,#34d399);font-family:var(--font-mono);white-space:pre-wrap;word-break:break-all;",
                    "{eval_verdict()}"
                }
            }
        }
    }
}
