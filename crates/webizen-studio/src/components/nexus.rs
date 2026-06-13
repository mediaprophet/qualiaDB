use dioxus::prelude::*;

// ── Enums ─────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Lens { Causal, Mathematical, Visual, Epistemic }

#[derive(Clone, Copy, PartialEq)]
enum NType { Paper, Claim, Dataset, Hypothesis, Simulation }

#[derive(Clone, Copy, PartialEq)]
enum EType { Supports, Contradicts, DerivedFrom }

#[derive(Clone, Copy, PartialEq)]
enum ModalOp { Knows, Believes, Common, Doubts }

// ── Domain structs ────────────────────────────────────────────────────────────

struct RNode {
    id:    &'static str,
    label: &'static str,
    year:  u32,
    ntype: NType,
    x:     f64,
    y:     f64,
}

struct REdge {
    from:  &'static str,
    to:    &'static str,
    etype: EType,
}

#[derive(Clone)]
struct EClaim {
    author:  String,
    op:      ModalOp,
    content: String,
    year:    u32,
}

// ── Pre-render data ───────────────────────────────────────────────────────────

struct ND { id: &'static str, label: &'static str, x: f64, y: f64, color: &'static str, ring: f64 }
struct ED { x1: f64, y1: f64, x2: f64, y2: f64, color: &'static str, dash: &'static str }
struct CD { key: String, op_label: &'static str, op_color: &'static str, author: String, year: u32, content: String }

// ── Static data ───────────────────────────────────────────────────────────────

fn research_nodes() -> Vec<RNode> {
    vec![
        RNode { id: "p-engel",  label: "FMO Coherence",       year: 2018, ntype: NType::Paper,      x: 130.0, y: 140.0 },
        RNode { id: "c-walk",   label: "Quantum Walk Claim",   year: 2018, ntype: NType::Claim,      x: 295.0, y: 80.0  },
        RNode { id: "p-smith",  label: "Decoherence Critique", year: 2022, ntype: NType::Paper,      x: 340.0, y: 295.0 },
        RNode { id: "c-deco",   label: "300K Eliminates Coh.", year: 2022, ntype: NType::Claim,      x: 515.0, y: 240.0 },
        RNode { id: "ds-sm",    label: "SM Spectroscopy HDF5", year: 2024, ntype: NType::Dataset,    x: 565.0, y: 70.0  },
        RNode { id: "h-vet",    label: "Vibration-Assist. ET", year: 2025, ntype: NType::Hypothesis, x: 655.0, y: 178.0 },
        RNode { id: "s-dft",    label: "DFT FMO Trimer",       year: 2025, ntype: NType::Simulation, x: 705.0, y: 325.0 },
        RNode { id: "p-pinn",   label: "PINN Binding Model",   year: 2020, ntype: NType::Paper,      x: 195.0, y: 395.0 },
        RNode { id: "c-dlvdft", label: "DL Surpasses DFT",     year: 2020, ntype: NType::Claim,      x: 395.0, y: 420.0 },
    ]
}

fn research_edges() -> Vec<REdge> {
    vec![
        REdge { from: "p-engel",  to: "c-walk",   etype: EType::DerivedFrom  },
        REdge { from: "p-smith",  to: "c-deco",   etype: EType::DerivedFrom  },
        REdge { from: "c-deco",   to: "c-walk",   etype: EType::Contradicts  },
        REdge { from: "ds-sm",    to: "h-vet",    etype: EType::Supports     },
        REdge { from: "h-vet",    to: "s-dft",    etype: EType::DerivedFrom  },
        REdge { from: "s-dft",    to: "c-walk",   etype: EType::Supports     },
        REdge { from: "p-pinn",   to: "c-dlvdft", etype: EType::DerivedFrom  },
    ]
}

fn initial_claims() -> Vec<EClaim> {
    vec![
        EClaim {
            author:  "dr.jones@did:q42:alice".into(),
            op:      ModalOp::Knows,
            content: "Quantum coherence time ≈500 fs at 77 K in FMO complex (2D-ES measurement).".into(),
            year:    2018,
        },
        EClaim {
            author:  "dr.smith@did:q42:bob".into(),
            op:      ModalOp::Believes,
            content: "Thermal noise at 300 K eliminates any functional role of quantum coherence in EET.".into(),
            year:    2022,
        },
        EClaim {
            author:  "dr.chen@did:q42:carol".into(),
            op:      ModalOp::Common,
            content: "The FMO complex contains 8 bacteriochlorophyll-a chromophores arranged as a trimer.".into(),
            year:    2024,
        },
        EClaim {
            author:  "dr.jones@did:q42:alice".into(),
            op:      ModalOp::Doubts,
            content: "PINN models adequately capture nuclear quantum effects in non-adiabatic energy transfer.".into(),
            year:    2025,
        },
    ]
}

// ── Component ─────────────────────────────────────────────────────────────────

#[component]
pub fn Nexus() -> Element {
    // Signals
    let mut lens       = use_signal(|| Lens::Causal);
    let mut tl_year    = use_signal(|| 2025_u32);
    let mut pan_x      = use_signal(|| -20.0_f64);
    let mut pan_y      = use_signal(|| -5.0_f64);
    let mut zoom       = use_signal(|| 1.0_f64);
    let mut dragging   = use_signal(|| false);
    let mut drag_sx    = use_signal(|| 0.0_f64);
    let mut drag_sy    = use_signal(|| 0.0_f64);
    let mut pan_ax     = use_signal(|| -20.0_f64);
    let mut pan_ay     = use_signal(|| -5.0_f64);
    let mut sel        = use_signal(|| Option::<&'static str>::None);
    let mut dispatch   = use_signal(|| Option::<&'static str>::None);
    let mut url_input  = use_signal(|| String::new());
    let mut claim_text = use_signal(|| String::new());
    let mut claim_op   = use_signal(|| ModalOp::Believes);
    let mut claims     = use_signal(|| initial_claims());

    // Snapshot signals
    let lns = lens();
    let yr  = tl_year();
    let z   = zoom();
    let px  = pan_x();
    let py  = pan_y();
    let s   = sel();
    let ds  = dispatch();
    let cs  = claims();

    // Static graph data
    let static_nodes = research_nodes();
    let static_edges = research_edges();

    // Helpers
    let get_xy   = |id: &str| static_nodes.iter().find(|n| n.id == id).map(|n| (n.x, n.y));
    let get_year = |id: &str| static_nodes.iter().find(|n| n.id == id).map_or(9999, |n| n.year);

    // Pre-compute edge display data
    let edge_data: Vec<ED> = static_edges.iter().filter_map(|e| {
        if get_year(e.from) > yr || get_year(e.to) > yr { return None; }
        let (x1, y1) = get_xy(e.from)?;
        let (x2, y2) = get_xy(e.to)?;
        let (color, dash) = match e.etype {
            EType::Supports    => ("#10b981", "none"),
            EType::Contradicts => ("#ef4444", "none"),
            EType::DerivedFrom => ("#64748b", "6 3"),
        };
        Some(ED { x1, y1, x2, y2, color, dash })
    }).collect();

    // Pre-compute node display data
    let node_data: Vec<ND> = static_nodes.iter()
        .filter(|n| n.year <= yr)
        .map(|n| {
            let label = match lns {
                Lens::Mathematical => "∀x:P",
                Lens::Visual       => "⬡",
                Lens::Epistemic    => "K(φ)",
                Lens::Causal       => n.label,
            };
            let color = match n.ntype {
                NType::Paper       => "#3b82f6",
                NType::Claim       => "#f59e0b",
                NType::Dataset     => "#10b981",
                NType::Hypothesis  => "#8b5cf6",
                NType::Simulation  => "#06b6d4",
            };
            ND { id: n.id, label, x: n.x, y: n.y, color, ring: if s == Some(n.id) { 3.5 } else { 1.5 } }
        }).collect();

    // Pre-compute claim display data
    let claim_data: Vec<CD> = cs.iter().map(|c| {
        let (op_label, op_color) = match c.op {
            ModalOp::Knows    => ("KNOWS",    "#10b981"),
            ModalOp::Believes => ("BELIEVES", "#3b82f6"),
            ModalOp::Common   => ("COMMON K.","#8b5cf6"),
            ModalOp::Doubts   => ("DOUBTS",   "#f59e0b"),
        };
        CD {
            key: format!("{}-{}", c.author, c.year),
            op_label, op_color,
            author:  c.author.clone(),
            year:    c.year,
            content: c.content.clone(),
        }
    }).collect();

    // SVG viewport dimensions
    let vw = 820.0 / z;
    let vh = 510.0 / z;

    // Selected node info for left panel
    let sel_info: Option<String> = s.and_then(|sid| {
        static_nodes.iter().find(|n| n.id == sid).map(|n| {
            format!("{} · {} · {}", n.label, n.year, match n.ntype {
                NType::Paper       => "Paper",
                NType::Claim       => "Claim",
                NType::Dataset     => "Dataset",
                NType::Hypothesis  => "Hypothesis",
                NType::Simulation  => "Simulation",
            })
        })
    });

    rsx! {
        div {
            style: "display:flex; flex-direction:column; width:100%; height:100%; overflow:hidden; font-size:0.8rem; color:var(--qualia-text);",

            // ── Header + Timeline ──────────────────────────────────────────
            div {
                style: "padding:0.65rem 1.1rem 0.55rem; background:var(--qualia-surface); border-bottom:1px solid var(--qualia-border); flex-shrink:0;",

                // Title + lens switcher
                div {
                    style: "display:flex; align-items:center; justify-content:space-between; margin-bottom:0.55rem;",

                    div { style: "display:flex; align-items:center; gap:0.55rem;",
                        div { style: "width:28px; height:28px; border-radius:8px; background:linear-gradient(135deg,#8b5cf6,#06b6d4); display:flex; align-items:center; justify-content:center; font-size:0.9rem;", "⚛" }
                        div {
                            div { style: "font-weight:700; font-size:0.9rem;", "Nexus" }
                            div { style: "font-size:0.65rem; color:var(--qualia-text-muted);", "Quantum Biology & Physics Research Cooperative" }
                        }
                    }

                    div { style: "display:flex; gap:0.25rem;",
                        for (lv, lbl) in [(Lens::Causal,"Causal"),(Lens::Mathematical,"Math"),(Lens::Visual,"Visual"),(Lens::Epistemic,"Epistemic")] {
                            {
                                let active = lns == lv;
                                let bg  = if active { "var(--qualia-accent)" } else { "rgba(128,128,128,0.1)" };
                                let col = if active { "white" } else { "var(--qualia-text-muted)" };
                                rsx! {
                                    button {
                                        key: "{lbl}",
                                        onclick: move |_| lens.set(lv),
                                        style: "background:{bg}; color:{col}; border:1px solid var(--qualia-border); border-radius:6px; padding:0.18rem 0.5rem; font-size:0.68rem; font-weight:500; font-family:'Inter',sans-serif; cursor:pointer;",
                                        "{lbl}"
                                    }
                                }
                            }
                        }
                    }
                }

                // Timeline scrubber
                div { style: "display:flex; align-items:center; gap:0.65rem;",
                    span { style: "font-size:0.65rem; color:var(--qualia-text-muted); flex-shrink:0;", "2010" }
                    div { style: "flex:1; position:relative;",
                        input {
                            r#type: "range", min: "2010", max: "2025", value: "{yr}",
                            style: "width:100%; accent-color:#8b5cf6;",
                            oninput: move |e| { if let Ok(v) = e.value().parse::<u32>() { tl_year.set(v); } },
                        }
                    }
                    span { style: "font-size:0.7rem; color:#8b5cf6; font-weight:700; flex-shrink:0; min-width:32px; text-align:right;", "▶ {yr}" }

                    // Zoom controls
                    div { style: "display:flex; gap:0.2rem; flex-shrink:0; margin-left:0.35rem;",
                        button {
                            onclick: move |_| zoom.set((z * 1.2).min(3.5)),
                            style: "background:rgba(128,128,128,0.1); border:1px solid var(--qualia-border); border-radius:5px; padding:0.15rem 0.45rem; font-size:0.72rem; cursor:pointer; color:var(--qualia-text); font-family:'Inter',sans-serif;",
                            "+"
                        }
                        button {
                            onclick: move |_| zoom.set((z * 0.83).max(0.35)),
                            style: "background:rgba(128,128,128,0.1); border:1px solid var(--qualia-border); border-radius:5px; padding:0.15rem 0.45rem; font-size:0.72rem; cursor:pointer; color:var(--qualia-text); font-family:'Inter',sans-serif;",
                            "−"
                        }
                        button {
                            onclick: move |_| { zoom.set(1.0); pan_x.set(-20.0); pan_y.set(-5.0); },
                            style: "background:rgba(128,128,128,0.1); border:1px solid var(--qualia-border); border-radius:5px; padding:0.15rem 0.45rem; font-size:0.65rem; cursor:pointer; color:var(--qualia-text-muted); font-family:'Inter',sans-serif;",
                            "⌂"
                        }
                    }
                }
            }

            // ── 3-column body ──────────────────────────────────────────────
            div {
                style: "display:grid; grid-template-columns:215px 1fr 248px; flex:1; overflow:hidden; min-height:0;",

                // ── LEFT: Ingestion Engine ─────────────────────────────────
                div {
                    style: "background:var(--qualia-surface); border-right:1px solid var(--qualia-border); padding:0.7rem; overflow-y:auto; display:flex; flex-direction:column; gap:0.6rem;",

                    div { style: "font-size:0.63rem; font-weight:700; letter-spacing:0.08em; text-transform:uppercase; color:var(--qualia-text-muted);", "Ingestion Engine" }

                    // PDF drop zone
                    div {
                        style: "border:2px dashed var(--qualia-border); border-radius:10px; padding:0.85rem 0.6rem; text-align:center; cursor:pointer;",
                        div { style: "font-size:1.25rem; margin-bottom:0.3rem;", "📄" }
                        div { style: "font-size:0.68rem; color:var(--qualia-text-muted); line-height:1.4;",
                            "Drop PDFs to extract SMILES, formulas, DOIs, and claim nodes"
                        }
                    }

                    // URL archiver
                    div {
                        div { style: "font-size:0.68rem; font-weight:600; margin-bottom:0.28rem;", "URL Archiver" }
                        div { style: "display:flex; gap:0.28rem;",
                            input {
                                r#type: "text", placeholder: "https://arxiv.org/...",
                                value: "{url_input()}",
                                style: "flex:1; min-width:0; background:rgba(128,128,128,0.1); border:1px solid var(--qualia-border); border-radius:6px; padding:0.27rem 0.4rem; font-size:0.67rem; color:var(--qualia-text); font-family:'Inter',sans-serif;",
                                oninput: move |e| url_input.set(e.value()),
                            }
                            button {
                                style: "flex-shrink:0; background:var(--qualia-accent); color:white; border:none; border-radius:6px; padding:0.27rem 0.5rem; font-size:0.63rem; font-weight:700; font-family:'Inter',sans-serif; cursor:pointer;",
                                "ICN"
                            }
                        }
                    }

                    // Extracted artifacts
                    div {
                        div { style: "font-size:0.68rem; font-weight:600; margin-bottom:0.28rem;", "Extracted Artifacts" }
                        for (lbl, kind, col) in [
                            ("c1ccccc1",          "SMILES",   "#10b981"),
                            ("10.1038/nphys2017", "DOI",      "#3b82f6"),
                            ("ħ∂ψ/∂t = Ĥψ",      "Formula",  "#8b5cf6"),
                            ("Engel et al., 2007","Citation", "#f59e0b"),
                        ] {
                            div {
                                key: "{lbl}",
                                style: "display:flex; align-items:center; gap:0.32rem; padding:0.24rem 0.32rem; background:rgba(128,128,128,0.06); border-radius:5px; margin-bottom:3px;",
                                span { style: "font-size:0.58rem; font-weight:700; color:{col}; background:rgba(128,128,128,0.1); border-radius:3px; padding:0.07rem 0.28rem; flex-shrink:0;", "{kind}" }
                                span { style: "font-size:0.65rem; color:var(--qualia-text-muted); overflow:hidden; text-overflow:ellipsis; white-space:nowrap;", "{lbl}" }
                            }
                        }
                    }

                    // Node type legend
                    div {
                        div { style: "font-size:0.68rem; font-weight:600; margin-bottom:0.28rem;", "Node Types" }
                        for (lbl, col) in [
                            ("Paper","#3b82f6"),("Claim","#f59e0b"),("Dataset","#10b981"),
                            ("Hypothesis","#8b5cf6"),("Simulation","#06b6d4"),
                        ] {
                            div { key: "{lbl}", style: "display:flex; align-items:center; gap:0.32rem; margin-bottom:3px;",
                                div { style: "width:9px; height:9px; border-radius:50%; background:{col}; flex-shrink:0;" }
                                span { style: "font-size:0.65rem; color:var(--qualia-text-muted);", "{lbl}" }
                            }
                        }
                        div { style: "margin-top:0.35rem; font-size:0.63rem; line-height:1.8; color:var(--qualia-text-muted);",
                            span { style: "color:#10b981;", "─ " } "Supports  "
                            span { style: "color:#ef4444;", "─ " } "Contradicts  "
                            span { style: "color:#64748b;", "╌ " } "Derived"
                        }
                    }

                    // Selected node info
                    if let Some(info) = &sel_info {
                        div {
                            style: "background:rgba(139,92,246,0.08); border:1px solid rgba(139,92,246,0.25); border-radius:8px; padding:0.42rem 0.5rem; margin-top:auto;",
                            div { style: "font-size:0.63rem; font-weight:700; color:#8b5cf6; margin-bottom:0.18rem;", "Selected" }
                            div { style: "font-size:0.67rem; color:var(--qualia-text); line-height:1.4;", "{info}" }
                        }
                    }
                }

                // ── CENTER: Research Canvas ────────────────────────────────
                div {
                    style: "position:relative; overflow:hidden; background:var(--qualia-bg); cursor:grab;",
                    onmousedown: move |evt| {
                        dragging.set(true);
                        drag_sx.set(evt.client_coordinates().x);
                        drag_sy.set(evt.client_coordinates().y);
                        pan_ax.set(pan_x());
                        pan_ay.set(pan_y());
                    },
                    onmousemove: move |evt| {
                        if dragging() {
                            let cz = zoom();
                            pan_x.set(pan_ax() - (evt.client_coordinates().x - drag_sx()) / cz);
                            pan_y.set(pan_ay() - (evt.client_coordinates().y - drag_sy()) / cz);
                        }
                    },
                    onmouseup:    move |_| dragging.set(false),
                    onmouseleave: move |_| dragging.set(false),

                    // Lens overlay badge
                    div {
                        style: "position:absolute; top:0.5rem; right:0.5rem; z-index:10; font-size:0.62rem; font-weight:600; color:#8b5cf6; background:rgba(139,92,246,0.1); border:1px solid rgba(139,92,246,0.25); border-radius:5px; padding:0.12rem 0.4rem; pointer-events:none;",
                        match lns {
                            Lens::Causal       => "Causal Provenance View",
                            Lens::Mathematical => "N3 Logic Rule View",
                            Lens::Visual       => "Molecular Structure View",
                            Lens::Epistemic    => "Epistemic State View",
                        }
                    }
                    div {
                        style: "position:absolute; top:0.5rem; left:0.5rem; z-index:10; font-size:0.62rem; color:var(--qualia-text-muted); pointer-events:none;",
                        "Showing ≤ {yr}"
                    }

                    svg {
                        style: "width:100%; height:100%;",
                        view_box: "{px} {py} {vw} {vh}",

                        defs {
                            pattern {
                                id: "nx-grid", width: "40", height: "40",
                                pattern_units: "userSpaceOnUse",
                                path { d: "M 40 0 L 0 0 0 40", fill: "none", stroke: "rgba(128,128,128,0.07)", "stroke-width": "0.5" }
                            }
                        }
                        rect { x: "{px-40.0}", y: "{py-40.0}", width: "{vw+80.0}", height: "{vh+80.0}", fill: "url(#nx-grid)" }

                        // Edges
                        for ed in edge_data.iter() {
                            line {
                                x1: "{ed.x1}", y1: "{ed.y1}",
                                x2: "{ed.x2}", y2: "{ed.y2}",
                                stroke: "{ed.color}", "stroke-width": "1.6",
                                "stroke-dasharray": "{ed.dash}", opacity: "0.6",
                            }
                        }

                        // Nodes
                        for nd in node_data.iter() {
                            {
                                let nid = nd.id;
                                rsx! {
                                    g {
                                        key: "{nd.id}",
                                        onclick: move |_| sel.set(if s == Some(nid) { None } else { Some(nid) }),
                                        style: "cursor:pointer;",
                                        circle {
                                            cx: "{nd.x}", cy: "{nd.y}", r: "18",
                                            fill: "{nd.color}", opacity: "0.88",
                                            stroke: "rgba(255,255,255,0.45)", "stroke-width": "{nd.ring}",
                                        }
                                        text {
                                            x: "{nd.x}", y: "{nd.y + 31.5}",
                                            "text-anchor": "middle", "font-size": "8.5",
                                            fill: "rgba(255,255,255,0.82)",
                                            "{nd.label}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // ── RIGHT: Epistemic Threads + Dispatch ────────────────────
                div {
                    style: "background:var(--qualia-surface); border-left:1px solid var(--qualia-border); padding:0.7rem; overflow-y:auto; display:flex; flex-direction:column; gap:0.6rem;",

                    // Epistemic thread list
                    div {
                        div { style: "font-size:0.63rem; font-weight:700; letter-spacing:0.08em; text-transform:uppercase; color:var(--qualia-text-muted); margin-bottom:0.4rem;", "Epistemic Threads" }

                        for cd in claim_data.iter() {
                            div {
                                key: "{cd.key}",
                                style: "background:rgba(128,128,128,0.05); border:1px solid var(--qualia-border); border-radius:8px; padding:0.42rem 0.48rem; margin-bottom:0.32rem;",
                                div { style: "display:flex; align-items:center; gap:0.32rem; margin-bottom:0.22rem; flex-wrap:wrap;",
                                    span { style: "font-size:0.57rem; font-weight:700; color:{cd.op_color}; background:rgba(128,128,128,0.1); border-radius:3px; padding:0.07rem 0.28rem; flex-shrink:0;", "{cd.op_label}" }
                                    span { style: "font-size:0.62rem; color:var(--qualia-text-muted); overflow:hidden; text-overflow:ellipsis; white-space:nowrap; max-width:110px;", "{cd.author}" }
                                    span { style: "font-size:0.59rem; color:var(--qualia-text-muted); flex-shrink:0;", "· {cd.year}" }
                                }
                                div { style: "font-size:0.67rem; line-height:1.45; color:var(--qualia-text);", "{cd.content}" }
                            }
                        }
                    }

                    // Assert claim form
                    div {
                        style: "border-top:1px solid var(--qualia-border); padding-top:0.5rem;",
                        div { style: "font-size:0.67rem; font-weight:600; margin-bottom:0.3rem;", "Assert Claim" }

                        // Modal op selector
                        div { style: "display:flex; gap:0.18rem; margin-bottom:0.3rem; flex-wrap:wrap;",
                            for (op, lbl) in [(ModalOp::Believes,"BELIEVES"),(ModalOp::Knows,"KNOWS"),(ModalOp::Common,"COMMON"),(ModalOp::Doubts,"DOUBTS")] {
                                {
                                    let active = claim_op() == op;
                                    let bg  = if active { "var(--qualia-accent)" } else { "rgba(128,128,128,0.1)" };
                                    let col = if active { "white" } else { "var(--qualia-text-muted)" };
                                    rsx! {
                                        button {
                                            key: "{lbl}",
                                            onclick: move |_| claim_op.set(op),
                                            style: "background:{bg}; color:{col}; border:1px solid var(--qualia-border); border-radius:4px; padding:0.1rem 0.28rem; font-size:0.57rem; font-weight:700; font-family:'Inter',sans-serif; cursor:pointer;",
                                            "{lbl}"
                                        }
                                    }
                                }
                            }
                        }

                        textarea {
                            placeholder: "State your epistemic claim...",
                            rows: "3",
                            value: "{claim_text()}",
                            style: "width:100%; background:rgba(128,128,128,0.08); border:1px solid var(--qualia-border); border-radius:6px; padding:0.28rem 0.42rem; font-size:0.67rem; color:var(--qualia-text); font-family:'Inter',sans-serif; resize:none; box-sizing:border-box;",
                            oninput: move |e| claim_text.set(e.value()),
                        }
                        button {
                            style: "margin-top:0.28rem; width:100%; background:var(--qualia-accent); color:white; border:none; border-radius:6px; padding:0.3rem; font-size:0.68rem; font-weight:600; font-family:'Inter',sans-serif; cursor:pointer;",
                            onclick: move |_| {
                                let txt = claim_text();
                                if txt.trim().is_empty() { return; }
                                let op  = claim_op();
                                let mut v = claims();
                                v.push(EClaim { author: "you@did:local".into(), op, content: txt, year: 2025 });
                                claims.set(v);
                                claim_text.set(String::new());
                            },
                            "Assert →"
                        }
                    }

                    // Native dispatch
                    div {
                        style: "border-top:1px solid var(--qualia-border); padding-top:0.5rem;",
                        div { style: "font-size:0.63rem; font-weight:700; letter-spacing:0.08em; text-transform:uppercase; color:var(--qualia-text-muted); margin-bottom:0.38rem;", "Native Dispatch" }

                        for (lbl, icon, op_id) in [
                            ("SW Alignment",       "bezier2",   "bioinformatics_align"),
                            ("DFT Ground State",   "cpu",       "qpu_dft"),
                            ("MCMC Thermodynamics","activity",  "chemical_analysis"),
                            ("RK4 ODE Solve",      "graph-up",  "ode_solve"),
                        ] {
                            {
                                let running = ds == Some(op_id);
                                let bg  = if running { "rgba(139,92,246,0.12)" } else { "rgba(128,128,128,0.07)" };
                                let brd = if running { "1px solid rgba(139,92,246,0.4)" } else { "1px solid var(--qualia-border)" };
                                let dot = if running { "#8b5cf6" } else { "rgba(128,128,128,0.3)" };
                                rsx! {
                                    button {
                                        key: "{lbl}",
                                        onclick: move |_| dispatch.set(Some(op_id)),
                                        style: "display:flex; align-items:center; gap:0.42rem; width:100%; background:{bg}; border:{brd}; border-radius:7px; padding:0.38rem 0.52rem; font-size:0.68rem; font-weight:500; font-family:'Inter',sans-serif; color:var(--qualia-text); cursor:pointer; margin-bottom:0.26rem; text-align:left;",
                                        div { style: "width:6px; height:6px; border-radius:50%; background:{dot}; flex-shrink:0;" }
                                        sl-icon { "name": "{icon}", style: "font-size:0.8rem; color:var(--qualia-accent); flex-shrink:0;" }
                                        "{lbl}"
                                    }
                                }
                            }
                        }

                        if let Some(op) = ds {
                            div {
                                style: "background:rgba(139,92,246,0.08); border:1px solid rgba(139,92,246,0.25); border-radius:7px; padding:0.42rem 0.48rem;",
                                div { style: "font-size:0.63rem; font-weight:700; color:#8b5cf6; margin-bottom:0.18rem;", "● Dispatching: {op}" }
                                div { style: "font-size:0.62rem; color:var(--qualia-text-muted); line-height:1.4;",
                                    "Routing to :4242 via SlgOpcode. Results annotated as NQuin provenance citations."
                                }
                                button {
                                    onclick: move |_| dispatch.set(None),
                                    style: "margin-top:0.28rem; font-size:0.6rem; color:var(--qualia-text-muted); background:none; border:none; cursor:pointer; font-family:'Inter',sans-serif; padding:0;",
                                    "cancel"
                                }
                            }
                        }
                    }

                    // Permissive Commons
                    div {
                        style: "border-top:1px solid var(--qualia-border); padding-top:0.5rem;",
                        div { style: "font-size:0.63rem; font-weight:700; letter-spacing:0.08em; text-transform:uppercase; color:var(--qualia-text-muted); margin-bottom:0.38rem;", "Permissive Commons" }
                        div {
                            style: "background:rgba(128,128,128,0.05); border:1px solid var(--qualia-border); border-radius:7px; padding:0.42rem 0.48rem;",
                            div { style: "font-size:0.65rem; color:var(--qualia-text); margin-bottom:0.18rem;", "Threshold Shift License v2.3" }
                            div { style: "font-size:0.61rem; color:var(--qualia-text-muted); line-height:1.4; margin-bottom:0.28rem;",
                                "WebTorrent DHT · did:q42:nexus-cooperative · 4 peers"
                            }
                            div { style: "display:flex; gap:0.28rem;",
                                button {
                                    style: "flex:1; background:var(--qualia-accent); color:white; border:none; border-radius:5px; padding:0.27rem; font-size:0.62rem; font-weight:600; font-family:'Inter',sans-serif; cursor:pointer;",
                                    "Seed"
                                }
                                button {
                                    style: "flex:1; background:rgba(128,128,128,0.1); color:var(--qualia-text-muted); border:1px solid var(--qualia-border); border-radius:5px; padding:0.27rem; font-size:0.62rem; font-family:'Inter',sans-serif; cursor:pointer;",
                                    "Audit TSL"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
