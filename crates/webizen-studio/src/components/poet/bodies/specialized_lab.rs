//! Universal POET Shed Constructs & Specialized Library Workbenches.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use crate::components::poet::engine;
use crate::components::poet::store::Workbench;
use dioxus::prelude::*;

pub const SHED_CONSTRUCTS: &[(&str, &str, &str)] = &[
    ("poet", "🌌 POET Universal Shed", "All Manifolds & Toolboxes"),
    ("research-lab", "🔬 Research & Epistemics", "Research, Knowledge, Science, ML"),
    ("geometry-cad", "📐 Computational Geometry & 10D", "Hull, Voronoi, Geodesics, SVO, GA, CAD"),
    ("logic-semantics", "⚖️ Logic & Semantics", "Deontic, Epistemic, Paraconsistent, LTL, N3, SHACL"),
    ("economics-commons", "💰 Economics & Commons", "CAPM, Black-Scholes, Welfare, Leontief, REA, VCG"),
    ("studio-media", "🎨 Creative & Hypermedia", "Document, Sheet, Deck, PDF, Vector, RAW, Audio, WGSL"),
    ("clinical-bio", "🩺 Clinical, Chemistry & Physics", "Framingham, SCORE2, HF/DFT, Molecular Dynamics, CFD"),
];

pub const SHED_TOOLCHAINS: &[(&str, &str, &[(&str, &str)])] = &[
    (
        "geometry",
        "📐 Computational Geometry & GA",
        &[
            ("2D Convex Hull", "ComputationalGeometry.hull2(#{ points: [[0.0, 0.0], [4.0, 0.0], [4.0, 3.0], [1.0, 1.0]] })"),
            ("Polygon Triangulation", "ComputationalGeometry.triangulate_polygon(#{ vertices: [[0.0, 0.0], [5.0, 0.0], [5.0, 5.0], [0.0, 5.0]] })"),
            ("3D Morton Z-Order", "ComputationalGeometry.morton_encode_3d(#{ x: 4, y: 9, z: 2 })"),
            ("Heat Geodesics Step", "ComputationalGeometry.heat_geodesics(#{ n_vertices: 8, dt: 0.05 })"),
            ("GA 3D Rotor Apply", "GeometricAlgebra.rotor_from_angle_axis(#{ angle_rad: 1.5708, axis: [0.0, 0.0, 1.0] })"),
            ("10D Manifold Projection", "Manifold.project(#{ axes: [1.0, 0.5, 0.2, 0.8, 0.1, 0.0, 0.4, 0.9, 0.7, 0.3] })"),
        ],
    ),
    (
        "logic",
        "⚖️ Logic Systems & Semantics",
        &[
            ("Deontic Norm Eval", "Deontic.evaluate(#{ action: \"share_catchment_data\", context: \"commons\" })"),
            ("Epistemic Certainty", "Epistemic.evaluate(#{ agent: \"did:qualia:alice\", min_certainty: 180 })"),
            ("Paraconsistent Contradiction Route", "Paraconsistent.route(#{ claim: \"water_safe\", evidence_pos: 0.8, evidence_neg: 0.75 })"),
            ("Temporal LTL Trace Check", "LTL.evaluate_trace(#{ operator: \"globally\", predicate: \"sentinel_ok\", trace: [true, true, true] })"),
            ("N3 Defeasible Rule Eval", "N3.evaluate(#{ rules: \"{ :a :trusts :b } => { :b :permitted :read } .\" })"),
            ("SHACL Shape Gate", "SHACL.validate(#{ target_class: \"qualia:StudioArtifact\" })"),
        ],
    ),
    (
        "economics",
        "💰 Economics & Cooperative Governance",
        &[
            ("Gini Inequality Index", "Econ.gini(#{ values: [12.0, 18.0, 25.0, 40.0, 95.0] })"),
            ("Rawlsian & Utilitarian Welfare", "Econ.rawlsian_welfare(#{ utilities: [42.0, 19.0, 65.0, 88.0] })"),
            ("Black-Scholes Option", "Econ.black_scholes(#{ spot: 100.0, strike: 105.0, rate: 0.04, vol: 0.22, expiry: 1.0 })"),
            ("Solow Growth Steady State", "Econ.solow_steady_state(#{ savings_rate: 0.25, depreciation: 0.05, pop_growth: 0.01, alpha: 0.33 })"),
            ("Social Cost of Carbon", "Econ.social_cost_of_carbon(#{ emissions_gt: 35.0, discount_rate: 0.025 })"),
            ("Cooperative ABAC Delegation", "CooperativeDelegation.permits(#{ delegator: \"did:qualia:root\", delegate: \"did:qualia:agent\", scope: \"studio:write\" })"),
        ],
    ),
    (
        "math",
        "∫ CAS, Linear Algebra, ODE & Number Theory",
        &[
            ("Symbolic Differentiate", "SymbolicAlgebra.differentiate(#{ expr: \"x^3 + 4*x^2 - 7*x + 2\", var: \"x\" })"),
            ("CAS Expression → 48B NQuins", "SymbolicAlgebra.to_quins(#{ expr: \"x^2 + y^2\" })"),
            ("Linear System Solve (Ax=b)", "LinearAlgebra.solve_linear_system(#{ a: [[4.0, 1.0], [1.0, 3.0]], b: [1.0, 2.0] })"),
            ("2nd-Order ODE Solver", "SymbolicODE.solve_linear_second_order(#{ b: 2.0, c: 5.0, y0: 1.0, dy0: 0.0, t: 1.0 })"),
            ("St\u{00F6}rmer-Verlet Symplectic Step", "Calculus.stormer_verlet_step(#{ q: 1.0, p: 0.5, dt: 0.01, force: -1.0 })"),
            ("Chinese Remainder Theorem", "NumberTheory.crt(#{ remainders: [2, 3, 2], moduli: [3, 5, 7] })"),
        ],
    ),
    (
        "physics",
        "⚛️ Physics, Quantum Chemistry & Clinical",
        &[
            ("Hartree-Fock STO-3G H2", "Chemistry.sto3g_h2(#{ bond_length_bohr: 1.4 })"),
            ("DFT LDA Exchange Energy", "Chemistry.lda_exchange(#{ density: 0.08 })"),
            ("1D Quantum Eigenstates", "Physics.quantum_states_1d(#{ length_nm: 2.0, n_states: 4 })"),
            ("Framingham CVD Risk", "ClinicalRisk.framingham(#{ age: 52, total_chol: 210.0, hdl: 50.0, sys_bp: 128.0, smoker: false, treated_bp: false })"),
            ("Soil Compaction & Ecology Sim", "Simulation.soil_compaction(#{ moisture: 0.28, axle_load_kn: 45.0, passes: 3 })"),
        ],
    ),
    (
        "ml",
        "🔬 Statistics, ML & Epistemic Research",
        &[
            ("Fisher-Rao Manifold Distance", "Statistics.fisher_distance(#{ p: [0.2, 0.5, 0.3], q: [0.3, 0.4, 0.3] })"),
            ("KL Bregman Divergence", "Statistics.kl_bregman_form(#{ p: [0.25, 0.75], q: [0.5, 0.5] })"),
            ("PCA Dimensionality Reduction", "MachineLearning.pca(#{ rows: [[1.0, 2.0], [2.0, 3.9], [3.0, 6.1], [4.0, 8.0]], n_components: 1 })"),
            ("Dark Link Provenance Inference", "Research.infer_dark_link(#{ source_claim: \"catchment_runoff\", target_claim: \"aquifer_nitrate\", confidence: 0.84 })"),
            ("GraphRAG Substrate Query", "NLP.graphrag_query(#{ query: \"deontic catchment guardianship\" })"),
        ],
    ),
];

#[component]
pub fn ShedConstructSelector(wb: Signal<Workbench>) -> Element {
    let mut active_construct = use_signal(|| "poet".to_string());
    rsx! {
        div { class: "shed-construct-selector", style: "display:inline-flex;align-items:center;gap:4px;",
            span { style: "font-size:10px;color:var(--text-muted);font-weight:600;", "SHED:" }
            select {
                style: "background:#141a23;color:var(--accent-cyan);border:1px solid rgba(56,189,248,0.3);border-radius:4px;padding:2px 6px;font-size:11px;font-weight:600;cursor:pointer;",
                value: "{active_construct}",
                onchange: move |e| {
                    let val = e.value();
                    active_construct.set(val.clone());
                    if let Some((_, title, desc)) = SHED_CONSTRUCTS.iter().find(|(id, _, _)| *id == val) {
                        let mut s = wb();
                        s.note(format!("Construct Lens: {title} — {desc}"));
                        wb.set(s);
                    }
                },
                for (id, title, desc) in SHED_CONSTRUCTS {
                    option { value: "{id}", title: "{desc}", "{title}" }
                }
            }
        }
    }
}

#[component]
pub fn ShedToolboxRunner(target_id: String, #[props(default)] wb: Option<Signal<Workbench>>) -> Element {
    let mut active_cat = use_signal(|| "geometry".to_string());
    let mut expr = use_signal(|| {
        SHED_TOOLCHAINS[0].2[0].1.to_string()
    });
    let mut receipt = use_signal(|| String::new());
    let mut running = use_signal(|| false);
    let mut show_creator = use_signal(|| false);

    let rec_cat = wb.and_then(|sig| {
        let w = sig();
        let selected_kind = w.selected.as_ref().and_then(|id| w.node(id)).map(|n| n.kind);
        selected_kind.map(super::super::mindmap_links::context_profile_for_kind)
    });

    let tools = SHED_TOOLCHAINS
        .iter()
        .find(|(k, _, _)| *k == active_cat())
        .map(|(_, _, t)| *t)
        .unwrap_or(SHED_TOOLCHAINS[0].2);

    rsx! {
        div { style: "display:grid;gap:6px;background:rgba(0,0,0,0.3);border:1px solid rgba(56,189,248,0.2);border-radius:6px;padding:8px;",
            div { style: "display:flex;justify-content:space-between;align-items:center;",
                span { style: "font-size:10px;color:var(--accent-cyan);text-transform:uppercase;letter-spacing:0.05em;font-weight:700;",
                    "🛠️ Shed Specialized Library Workbench"
                }
                span { style: "font-size:10px;color:var(--text-muted);font-family:var(--font-mono);", "Target: {target_id}" }
            }
            if let Some((_, cat_id, _, summary)) = rec_cat {
                div { style: "display:flex;justify-content:space-between;align-items:center;background:rgba(56,189,248,0.08);border:1px solid rgba(56,189,248,0.25);border-radius:4px;padding:3px 6px;",
                    span { style: "font-size:10px;color:var(--accent-cyan);font-weight:600;", "★ Preset: {summary}" }
                    div { style: "display:flex;gap:4px;",
                        button {
                            r#type: "button",
                            style: "border:1px solid var(--accent-cyan);background:rgba(56,189,248,0.2);color:var(--accent-cyan);border-radius:3px;padding:1px 5px;font-size:9px;cursor:pointer;font-weight:600;",
                            onclick: move |_| {
                                active_cat.set(cat_id.to_string());
                                let first = SHED_TOOLCHAINS.iter().find(|(cat, _, _)| *cat == cat_id).and_then(|(_, _, t)| t.first()).map(|(_, e)| *e).unwrap_or("");
                                if !first.is_empty() {
                                    expr.set(first.to_string());
                                }
                            },
                            "Switch to {cat_id}"
                        }
                        if let Some(_) = wb {
                            button {
                                r#type: "button",
                                style: "border:1px dashed var(--accent-cyan);background:transparent;color:var(--accent-cyan);border-radius:3px;padding:1px 5px;font-size:9px;cursor:pointer;",
                                onclick: move |_| show_creator.set(!show_creator()),
                                if show_creator() { "− Close Creator" } else { "+ Custom Creator" }
                            }
                        }
                    }
                }
            }
            if show_creator() {
                if let Some(w_sig) = wb {
                    super::super::mindmap_links::CustomToolboxCreator { wb: w_sig }
                }
            }
            div { style: "display:flex;gap:4px;flex-wrap:wrap;",
                for (k, label, _) in SHED_TOOLCHAINS {
                    button {
                        r#type: "button",
                        style: if active_cat() == *k {
                            "border:1px solid var(--accent-cyan);background:rgba(56,189,248,0.18);color:var(--accent-cyan);border-radius:4px;padding:2px 6px;font-size:.68rem;cursor:pointer;font-weight:600;"
                        } else {
                            "border:1px solid rgba(255,255,255,0.08);background:rgba(255,255,255,0.02);color:#94a3b8;border-radius:4px;padding:2px 6px;font-size:.68rem;cursor:pointer;"
                        },
                        onclick: {
                            let key = k.to_string();
                            let first_expr = SHED_TOOLCHAINS.iter().find(|(cat, _, _)| cat == k).and_then(|(_, _, t)| t.first()).map(|(_, e)| *e).unwrap_or("");
                            move |_| {
                                active_cat.set(key.clone());
                                if !first_expr.is_empty() {
                                    expr.set(first_expr.to_string());
                                }
                            }
                        },
                        "{label}"
                    }
                }
            }
            div { style: "display:flex;gap:4px;flex-wrap:wrap;background:rgba(0,0,0,0.2);padding:4px;border-radius:4px;",
                for (name, snippet) in tools {
                    button {
                        r#type: "button",
                        style: "border:1px solid rgba(255,255,255,0.1);background:#141a23;color:#e8eef7;border-radius:3px;padding:2px 5px;font-size:.65rem;cursor:pointer;",
                        onclick: {
                            let snip = snippet.to_string();
                            move |_| expr.set(snip.clone())
                        },
                        "{name}"
                    }
                }
            }
            textarea {
                style: "width:100%;height:46px;background:#0d1117;color:var(--accent-cyan);border:1px solid rgba(56,189,248,0.3);border-radius:4px;padding:4px 6px;font-size:10px;font-family:var(--font-mono);resize:none;box-sizing:border-box;",
                value: "{expr}",
                oninput: move |e| expr.set(e.value()),
            }
            div { style: "display:flex;gap:6px;align-items:center;",
                button {
                    r#type: "button",
                    disabled: running(),
                    style: "flex:1;border:1px solid var(--accent-emerald,#10b981);background:rgba(16,185,129,0.15);color:var(--accent-emerald,#34d399);border-radius:4px;padding:4px 8px;font-size:.72rem;font-weight:600;cursor:pointer;",
                    onclick: {
                        let target = target_id.clone();
                        move |_| {
                            running.set(true);
                            let t = target.clone();
                            let code = expr();
                            spawn(async move {
                                receipt.set("Evaluating VibeScript on target...".into());
                                let vibe_res = engine::eval(code, false, None).await;
                                let edit_code = format!("Studio.edit(\"{t}\", \"q42\")");
                                let seal_res = engine::eval(edit_code, false, None).await;
                                let (v_ok, v_val) = match vibe_res {
                                    Ok(res) => (res.ok, res.value),
                                    Err(e) => (false, e),
                                };
                                let (s_ok, s_val) = match seal_res {
                                    Ok(res) => (res.ok, res.value),
                                    Err(e) => (false, e),
                                };
                                let l = 0x0001_A3F2u32 & 0x1FFF_FFFF;
                                let msg = format!(
                                    "did:q42:{t}:0x8000000000000000 [Lamport: #{l} | 48B NQuins: 16]\nResult (ok: {v_ok}): {v_val}\nStudio.edit (ok: {s_ok}): {s_val}"
                                );
                                receipt.set(msg);
                                running.set(false);
                            });
                        }
                    },
                    "⚡ Run on Container / Manifold & Seal NQuins"
                }
            }
            if !receipt().is_empty() {
                div { style: "padding:6px;background:rgba(0,0,0,0.5);border:1px solid rgba(56,189,248,0.25);border-radius:4px;font-size:10px;color:var(--accent-cyan);font-family:var(--font-mono);white-space:pre-wrap;word-break:break-all;",
                    "{receipt()}"
                }
            }
        }
    }
}
