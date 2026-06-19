#![allow(non_snake_case)]
use dioxus::prelude::*;

// ── Domain types ──────────────────────────────────────────────────────────────

#[derive(Clone, PartialEq, Debug, Copy)]
enum NodeZone {
    Selfhood,
    Personhood,
    Bridge,
}

#[derive(Clone, PartialEq, Debug, Copy)]
enum NodeCat {
    Identity,
    Keys,
    Data,
    Agreement,
    Media,
    Commons,
    Agent,
    Knowledge,
}

#[derive(Clone, PartialEq, Debug)]
struct GraphNode {
    id: &'static str,
    label: &'static str,
    x: f64,
    y: f64,
    zone: NodeZone,
    cat: NodeCat,
    detail: &'static str,
}

#[derive(Clone, PartialEq, Debug)]
struct GraphEdge {
    from: &'static str,
    to: &'static str,
}

#[derive(Clone, PartialEq, Debug, Copy)]
enum Lens {
    All,
    Identity,
    Financial,
    Social,
    Knowledge,
}

#[derive(Clone, PartialEq)]
struct ChatMsg {
    is_user: bool,
    text: String,
}

// ── Static data ───────────────────────────────────────────────────────────────

fn graph_nodes() -> Vec<GraphNode> {
    vec![
        // Selfhood
        GraphNode {
            id: "identity",
            label: "Identity",
            x: 170.,
            y: 110.,
            zone: NodeZone::Selfhood,
            cat: NodeCat::Identity,
            detail: "did:q42:local-admin",
        },
        GraphNode {
            id: "keys",
            label: "Private Keys",
            x: 65.,
            y: 230.,
            zone: NodeZone::Selfhood,
            cat: NodeCat::Keys,
            detail: "ed25519 · x25519 ECDH",
        },
        GraphNode {
            id: "did-doc",
            label: "DID Document",
            x: 295.,
            y: 195.,
            zone: NodeZone::Selfhood,
            cat: NodeCat::Identity,
            detail: "W3C DID v1.1",
        },
        GraphNode {
            id: "enc-data",
            label: "Encrypted Data",
            x: 145.,
            y: 340.,
            zone: NodeZone::Selfhood,
            cat: NodeCat::Data,
            detail: "AES-256-GCM vault",
        },
        // Personhood
        GraphNode {
            id: "agreements",
            label: "Agreements",
            x: 560.,
            y: 105.,
            zone: NodeZone::Personhood,
            cat: NodeCat::Agreement,
            detail: "3 active bilateral",
        },
        GraphNode {
            id: "tsl",
            label: "TSL Artifacts",
            x: 685.,
            y: 210.,
            zone: NodeZone::Personhood,
            cat: NodeCat::Commons,
            detail: "Threshold Shift License",
        },
        GraphNode {
            id: "media",
            label: "Media Assets",
            x: 510.,
            y: 265.,
            zone: NodeZone::Personhood,
            cat: NodeCat::Media,
            detail: "34 NQuin-anchored",
        },
        GraphNode {
            id: "publications",
            label: "Commons Pub.",
            x: 670.,
            y: 345.,
            zone: NodeZone::Personhood,
            cat: NodeCat::Commons,
            detail: "WebTorrent seeded",
        },
        GraphNode {
            id: "webtorrent",
            label: "WebTorrent DHT",
            x: 565.,
            y: 430.,
            zone: NodeZone::Personhood,
            cat: NodeCat::Commons,
            detail: "Permissive Commons seed",
        },
        // Bridge
        GraphNode {
            id: "inforg",
            label: "Inforg Agent",
            x: 400.,
            y: 175.,
            zone: NodeZone::Bridge,
            cat: NodeCat::Agent,
            detail: "Local GGUF · Phase 8",
        },
        GraphNode {
            id: "knowledge",
            label: "Knowledge Base",
            x: 390.,
            y: 375.,
            zone: NodeZone::Bridge,
            cat: NodeCat::Knowledge,
            detail: "Standards + Ontologies",
        },
    ]
}

fn graph_edges() -> Vec<GraphEdge> {
    vec![
        GraphEdge {
            from: "identity",
            to: "keys",
        },
        GraphEdge {
            from: "identity",
            to: "did-doc",
        },
        GraphEdge {
            from: "identity",
            to: "inforg",
        },
        GraphEdge {
            from: "inforg",
            to: "agreements",
        },
        GraphEdge {
            from: "did-doc",
            to: "agreements",
        },
        GraphEdge {
            from: "did-doc",
            to: "tsl",
        },
        GraphEdge {
            from: "enc-data",
            to: "knowledge",
        },
        GraphEdge {
            from: "agreements",
            to: "media",
        },
        GraphEdge {
            from: "agreements",
            to: "tsl",
        },
        GraphEdge {
            from: "media",
            to: "publications",
        },
        GraphEdge {
            from: "publications",
            to: "webtorrent",
        },
        GraphEdge {
            from: "inforg",
            to: "knowledge",
        },
    ]
}

// ── Helper fns ────────────────────────────────────────────────────────────────

fn zone_color(zone: NodeZone) -> &'static str {
    match zone {
        NodeZone::Selfhood => "#d97706",
        NodeZone::Personhood => "#0ea5e9",
        NodeZone::Bridge => "#8b5cf6",
    }
}

fn node_opacity(cat: NodeCat, lens: Lens) -> f64 {
    match lens {
        Lens::All => 1.0,
        Lens::Identity => {
            if matches!(cat, NodeCat::Identity | NodeCat::Keys | NodeCat::Agent) {
                1.0
            } else {
                0.12
            }
        }
        Lens::Financial => {
            if matches!(cat, NodeCat::Agreement | NodeCat::Commons) {
                1.0
            } else {
                0.12
            }
        }
        Lens::Social => {
            if matches!(cat, NodeCat::Agreement | NodeCat::Media | NodeCat::Commons) {
                1.0
            } else {
                0.12
            }
        }
        Lens::Knowledge => {
            if matches!(cat, NodeCat::Knowledge | NodeCat::Agent) {
                1.0
            } else {
                0.12
            }
        }
    }
}

fn find_xy(id: &str, nodes: &[GraphNode]) -> Option<(f64, f64)> {
    nodes.iter().find(|n| n.id == id).map(|n| (n.x, n.y))
}

fn inforg_reply(q: &str) -> String {
    let q = q.to_lowercase();
    if q.contains("identity") || q.contains("did") {
        "Found 1 DID document: did:q42:local-admin. Bound to 3 ed25519 keys. SHACL validated ✓. Last modified: 2026-06-13.".into()
    } else if q.contains("agreement") || q.contains("contract") {
        "3 active bilateral agreements in Personhood graph. TSL-licensed artifacts: 7. Pending fiduciary review: 0. Sentinel: all ODRL policies satisfied.".into()
    } else if q.contains("media") || q.contains("asset") {
        "Media index: 34 NQuin-anchored assets. 12 seeded to WebTorrent DHT. Local storage: 2.3 GB. All in Zero-Trust sandboxed state pending SHACL verification.".into()
    } else if q.contains("knowledge") || q.contains("standard") || q.contains("ontology") {
        "Knowledge base: 1,842 NQuins across 9 ontology domains (PROV-O, ODRL, GeoSPARQL, W3C CogAI, SHACL, KML, SKOS, DC, RDF). Last sync: 2026-06-13.".into()
    } else if q.contains("sparql") || q.contains("select") || q.contains("query") {
        "Query dispatched to Webizen VM. Sentinel pre-flight: PASS. Result: 0 grounded NQuins returned (GGUF offline). Connect a local model in LLM Harness to enable full inference.".into()
    } else if q.contains("commons") || q.contains("webtorrent") || q.contains("seed") {
        "Commons Gateway: 3 artifacts seeded to DHT (TSL-v1.3, PROV-O, ODRL-2.2). PermissiveRoutingLane: open. No micropayment gates active. 14 peers discovered.".into()
    } else {
        format!(
            "Querying NQuin space for '{}' — 0 grounded results. The Inforg Sentinel requires provenance citations for all responses. Try: identity, agreements, media, knowledge, or a SPARQL query.",
            q
        )
    }
}

// ── Component ─────────────────────────────────────────────────────────────────

#[component]
pub fn ContextualWorkspace() -> Element {
    // Canvas state
    let mut pan_x = use_signal(|| 0.0_f64);
    let mut pan_y = use_signal(|| 0.0_f64);
    let mut zoom = use_signal(|| 1.0_f64);
    let mut dragging = use_signal(|| false);
    let mut drag_sx = use_signal(|| 0.0_f64);
    let mut drag_sy = use_signal(|| 0.0_f64);
    let mut pan_at_dx = use_signal(|| 0.0_f64);
    let mut pan_at_dy = use_signal(|| 0.0_f64);

    // UI state
    let mut active_lens = use_signal(|| Lens::All);
    let mut selected = use_signal(|| None::<String>);
    let mut time_pos = use_signal(|| 100u32);

    // Chat
    let mut msgs = use_signal(|| {
        vec![
        ChatMsg { is_user: false, text: "Context Studio ready. I can query your semantic graph, navigate the Permissive Commons, or inspect the temporal ledger. What would you like to explore?".into() }
    ]
    });
    let mut chat_in = use_signal(|| String::new());

    // Commons
    let mut commons_q = use_signal(|| String::new());

    // Derived
    let lens = active_lens();
    let t = time_pos();
    let z = zoom();
    let vw = 800.0 / z;
    let vh = 500.0 / z;
    let vbox = format!("{:.1} {:.1} {:.1} {:.1}", pan_x(), pan_y(), vw, vh);

    let nodes = graph_nodes();
    let edges = graph_edges();

    // Time label
    let time_label = if t == 100 {
        "Now".to_string()
    } else {
        let days_ago = (100 - t) as i32 * 163 / 100;
        format!("−{days_ago}d")
    };

    let scrubber_accent = if t < 100 {
        "#f59e0b"
    } else {
        "var(--qualia-accent)"
    };
    let cursor = if dragging() {
        "cursor:grabbing;"
    } else {
        "cursor:grab;"
    };

    // ── Event handlers ──────────────────────────────────────────────────

    let on_md = move |evt: Event<MouseData>| {
        let c = evt.data().client_coordinates();
        drag_sx.set(c.x);
        drag_sy.set(c.y);
        pan_at_dx.set(pan_x());
        pan_at_dy.set(pan_y());
        dragging.set(true);
    };

    let on_mm = move |evt: Event<MouseData>| {
        if !dragging() {
            return;
        }
        let c = evt.data().client_coordinates();
        pan_x.set(pan_at_dx() - (c.x - drag_sx()) / zoom());
        pan_y.set(pan_at_dy() - (c.y - drag_sy()) / zoom());
    };

    let on_mu = move |_: Event<MouseData>| dragging.set(false);

    let on_zoom = move |e: Event<FormData>| {
        if let Ok(v) = e.value().parse::<f64>() {
            zoom.set(v);
        }
    };

    let on_time = move |e: Event<FormData>| {
        if let Ok(v) = e.value().parse::<u32>() {
            time_pos.set(v);
        }
    };

    let submit_chat = move |_: Event<MouseData>| {
        let q = chat_in().trim().to_string();
        if q.is_empty() {
            return;
        }
        msgs.write().push(ChatMsg {
            is_user: true,
            text: q.clone(),
        });
        msgs.write().push(ChatMsg {
            is_user: false,
            text: inforg_reply(&q),
        });
        chat_in.set(String::new());
    };

    // ── Build edge SVG data (pre-compute to avoid complex closures in RSX) ──
    let chat_snapshot = msgs();

    let edge_svgs: Vec<(f64, f64, f64, f64, f64)> = edges
        .iter()
        .filter_map(|e| {
            let (fx, fy) = find_xy(e.from, &nodes)?;
            let (tx, ty) = find_xy(e.to, &nodes)?;
            let fn_ = nodes.iter().find(|n| n.id == e.from)?;
            let tn_ = nodes.iter().find(|n| n.id == e.to)?;
            let op = ((node_opacity(fn_.cat, lens) + node_opacity(tn_.cat, lens)) / 2.0 * 0.55)
                .max(0.06);
            Some((fx, fy, tx, ty, op))
        })
        .collect();

    // ── Build node render data ──
    let node_renders: Vec<(
        &'static str,
        f64,
        f64,
        &'static str,
        f64,
        bool,
        &'static str,
        &'static str,
    )> = nodes
        .iter()
        .map(|n| {
            let color = zone_color(n.zone);
            let opacity = node_opacity(n.cat, lens);
            let is_sel = selected().as_deref() == Some(n.id);
            let stroke = if is_sel {
                "white"
            } else {
                "rgba(255,255,255,0.2)"
            };
            let sw = if is_sel { "3" } else { "1.5" };
            (n.id, n.x, n.y, color, opacity, is_sel, stroke, sw)
        })
        .collect();

    // ── Node labels (separate pass so they render above circles) ──
    let label_renders: Vec<(&'static str, f64, f64, f64)> = nodes
        .iter()
        .map(|n| (n.label, n.x, n.y + 40.0, node_opacity(n.cat, lens)))
        .collect();

    rsx! {
        div {
            style: "width:100%; height:100%; display:flex; flex-direction:column; overflow:hidden; background: var(--qualia-bg);",

            // ── Toolbar ───────────────────────────────────────────────────
            div {
                style: "display:flex; align-items:center; gap:0.75rem; padding:0.55rem 1rem; background:var(--qualia-surface); border-bottom:1px solid var(--qualia-border); backdrop-filter:blur(12px); flex-shrink:0; flex-wrap:wrap;",

                // Title
                div { style:"display:flex; align-items:center; gap:0.5rem; margin-right:0.25rem;",
                    sl-icon { "name":"diagram-3", style:"font-size:1rem; color:var(--qualia-accent);" }
                    span { style:"font-size:0.875rem; font-weight:700; color:var(--qualia-text);", "Context Studio" }
                    span { style:"font-size:0.65rem; color:var(--qualia-text-muted); background:rgba(128,128,128,0.1); border:1px solid var(--qualia-border); border-radius:4px; padding:1px 5px;", "1.0-draft" }
                }

                // Lens buttons
                div { style:"display:flex; align-items:center; gap:0.25rem;",
                    span { style:"font-size:0.7rem; color:var(--qualia-text-muted); margin-right:0.15rem;", "Lens:" }
                    {[
                        (Lens::All,       "All"),
                        (Lens::Identity,  "Identity"),
                        (Lens::Financial, "Financial"),
                        (Lens::Social,    "Social"),
                        (Lens::Knowledge, "Knowledge"),
                    ].iter().map(|(l, label)| {
                        let is_active = *l == lens;
                        let bg    = if is_active { "var(--qualia-accent)" } else { "rgba(128,128,128,0.1)" };
                        let color = if is_active { "white" } else { "var(--qualia-text-muted)" };
                        let lv = *l;
                        rsx! {
                            button {
                                key: "{label}",
                                onclick: move |_| active_lens.set(lv),
                                style: "background:{bg}; color:{color}; border:1px solid var(--qualia-border); border-radius:5px; padding:0.18rem 0.55rem; font-size:0.7rem; font-weight:500; cursor:pointer; font-family:'Inter',sans-serif; transition:all 0.15s;",
                                "{label}"
                            }
                        }
                    })}
                }

                div { style:"flex:1;" }

                // Zone legend
                div { style:"display:flex; align-items:center; gap:0.75rem; font-size:0.7rem;",
                    div { style:"display:flex; align-items:center; gap:0.3rem;",
                        div { style:"width:9px; height:9px; border-radius:2px; background:rgba(217,119,6,0.3); border:1px solid #d97706;" }
                        span { style:"color:var(--qualia-text-muted);", "Selfhood" }
                    }
                    div { style:"display:flex; align-items:center; gap:0.3rem;",
                        div { style:"width:9px; height:9px; border-radius:2px; background:rgba(14,165,233,0.25); border:1px solid #0ea5e9;" }
                        span { style:"color:var(--qualia-text-muted);", "Personhood" }
                    }
                    div { style:"display:flex; align-items:center; gap:0.3rem;",
                        div { style:"width:9px; height:9px; border-radius:2px; background:rgba(139,92,246,0.25); border:1px solid #8b5cf6;" }
                        span { style:"color:var(--qualia-text-muted);", "Bridge" }
                    }
                }

                // Zoom slider
                div { style:"display:flex; align-items:center; gap:0.35rem;",
                    sl-icon { "name":"zoom-out", style:"font-size:0.72rem; color:var(--qualia-text-muted);" }
                    input {
                        r#type:"range", min:"0.35", max:"2.5", step:"0.05", value:"{z}",
                        oninput: on_zoom,
                        style:"width:72px; accent-color:var(--qualia-accent);",
                    }
                    sl-icon { "name":"zoom-in", style:"font-size:0.72rem; color:var(--qualia-text-muted);" }
                }
            }

            // ── Main row ──────────────────────────────────────────────────
            div { style:"flex:1; display:flex; overflow:hidden;",

                // Left nav tree
                div {
                    style:"width:176px; flex-shrink:0; background:var(--qualia-surface); border-right:1px solid var(--qualia-border); overflow-y:auto; padding:0.75rem 0.625rem; display:flex; flex-direction:column; gap:0.2rem;",

                    // Selfhood
                    div { style:"margin-bottom:0.4rem;",
                        div { style:"display:flex; align-items:center; gap:0.35rem; padding:0.25rem 0; margin-bottom:0.15rem;",
                            div { style:"width:7px; height:7px; border-radius:2px; background:#d97706;" }
                            span { style:"font-size:0.68rem; font-weight:700; color:#d97706; letter-spacing:0.05em; text-transform:uppercase;", "Selfhood" }
                        }
                        {nodes.iter().filter(|n| n.zone == NodeZone::Selfhood).map(|n| {
                            let is_sel = selected().as_deref() == Some(n.id);
                            let bg = if is_sel { "rgba(217,119,6,0.14)" } else { "transparent" };
                            let nid = n.id.to_string();
                            rsx! {
                                div {
                                    key: "{n.id}",
                                    onclick: move |_| selected.set(Some(nid.clone())),
                                    style: "display:flex; align-items:center; gap:0.35rem; padding:0.27rem 0.45rem; border-radius:5px; cursor:pointer; background:{bg}; transition:background 0.12s;",
                                    div { style:"width:5px; height:5px; border-radius:50%; background:#d97706; flex-shrink:0;" }
                                    span { style:"font-size:0.76rem; color:var(--qualia-text);", "{n.label}" }
                                }
                            }
                        })}
                    }

                    div { style:"height:1px; background:var(--qualia-border); margin:0.2rem 0;" }

                    // Personhood
                    div { style:"margin-bottom:0.4rem;",
                        div { style:"display:flex; align-items:center; gap:0.35rem; padding:0.25rem 0; margin-bottom:0.15rem;",
                            div { style:"width:7px; height:7px; border-radius:2px; background:#0ea5e9;" }
                            span { style:"font-size:0.68rem; font-weight:700; color:#0ea5e9; letter-spacing:0.05em; text-transform:uppercase;", "Personhood" }
                        }
                        {nodes.iter().filter(|n| n.zone == NodeZone::Personhood).map(|n| {
                            let is_sel = selected().as_deref() == Some(n.id);
                            let bg = if is_sel { "rgba(14,165,233,0.14)" } else { "transparent" };
                            let nid = n.id.to_string();
                            rsx! {
                                div {
                                    key: "{n.id}",
                                    onclick: move |_| selected.set(Some(nid.clone())),
                                    style: "display:flex; align-items:center; gap:0.35rem; padding:0.27rem 0.45rem; border-radius:5px; cursor:pointer; background:{bg}; transition:background 0.12s;",
                                    div { style:"width:5px; height:5px; border-radius:50%; background:#0ea5e9; flex-shrink:0;" }
                                    span { style:"font-size:0.76rem; color:var(--qualia-text);", "{n.label}" }
                                }
                            }
                        })}
                    }

                    div { style:"height:1px; background:var(--qualia-border); margin:0.2rem 0;" }

                    // Bridge
                    div {
                        div { style:"display:flex; align-items:center; gap:0.35rem; padding:0.25rem 0; margin-bottom:0.15rem;",
                            div { style:"width:7px; height:7px; border-radius:2px; background:#8b5cf6;" }
                            span { style:"font-size:0.68rem; font-weight:700; color:#8b5cf6; letter-spacing:0.05em; text-transform:uppercase;", "Bridge" }
                        }
                        {nodes.iter().filter(|n| n.zone == NodeZone::Bridge).map(|n| {
                            let is_sel = selected().as_deref() == Some(n.id);
                            let bg = if is_sel { "rgba(139,92,246,0.14)" } else { "transparent" };
                            let nid = n.id.to_string();
                            rsx! {
                                div {
                                    key: "{n.id}",
                                    onclick: move |_| selected.set(Some(nid.clone())),
                                    style: "display:flex; align-items:center; gap:0.35rem; padding:0.27rem 0.45rem; border-radius:5px; cursor:pointer; background:{bg}; transition:background 0.12s;",
                                    div { style:"width:5px; height:5px; border-radius:50%; background:#8b5cf6; flex-shrink:0;" }
                                    span { style:"font-size:0.76rem; color:var(--qualia-text);", "{n.label}" }
                                }
                            }
                        })}
                    }

                    div { style:"flex:1;" }

                    // Selected node detail card
                    if let Some(sel_id) = selected().as_deref() {
                        if let Some(n) = nodes.iter().find(|n| n.id == sel_id) {
                            div {
                                style: "background:rgba(128,128,128,0.07); border:1px solid var(--qualia-border); border-radius:8px; padding:0.55rem; margin-top:0.4rem;",
                                div { style:"font-size:0.76rem; font-weight:600; color:var(--qualia-text); margin-bottom:0.2rem;", "{n.label}" }
                                div { style:"font-size:0.68rem; color:var(--qualia-text-muted); font-family:monospace; word-break:break-all;", "{n.detail}" }
                            }
                        }
                    }
                }

                // ── Semantic canvas ───────────────────────────────────────
                div {
                    style: "flex:1; overflow:hidden; position:relative; {cursor}",
                    onmousedown: on_md,
                    onmousemove: on_mm,
                    onmouseup:   on_mu,
                    onmouseleave: move |_| dragging.set(false),

                    // Time-travel banner
                    if t < 100 {
                        div {
                            style: "position:absolute; top:10px; left:50%; transform:translateX(-50%); z-index:10; background:rgba(245,158,11,0.12); border:1px solid rgba(245,158,11,0.45); border-radius:8px; padding:0.25rem 0.75rem; font-size:0.7rem; color:#f59e0b; font-weight:600; backdrop-filter:blur(8px); pointer-events:none; white-space:nowrap;",
                            "⏪  Temporal view: {time_label} from present  ·  AS OF query active"
                        }
                    }

                    svg {
                        width: "100%",
                        height: "100%",
                        view_box: "{vbox}",
                        style: "pointer-events:none; user-select:none; display:block;",

                        // Zone backgrounds
                        rect {
                            x:"8", y:"8", width:"368", height:"455", rx:"14",
                            fill:"rgba(217,119,6,0.06)",
                            stroke:"rgba(217,119,6,0.22)",
                            stroke_width:"1.5",
                            stroke_dasharray:"6 3",
                        }
                        text {
                            x:"22", y:"28",
                            style:"font-size:9px; font-family:Inter,sans-serif; font-weight:700; letter-spacing:0.07em; fill:rgba(217,119,6,0.55);",
                            "SELFHOOD — PRIVATE"
                        }

                        rect {
                            x:"440", y:"8", width:"360", height:"455", rx:"14",
                            fill:"rgba(14,165,233,0.055)",
                            stroke:"rgba(14,165,233,0.2)",
                            stroke_width:"1.5",
                            stroke_dasharray:"6 3",
                        }
                        text {
                            x:"455", y:"28",
                            style:"font-size:9px; font-family:Inter,sans-serif; font-weight:700; letter-spacing:0.07em; fill:rgba(14,165,233,0.55);",
                            "PERSONHOOD — SOCIAL"
                        }

                        // Edges
                        {edge_svgs.iter().enumerate().map(|(i, (fx, fy, tx, ty, op))| rsx! {
                            line {
                                key: "{i}",
                                x1:"{fx}", y1:"{fy}", x2:"{tx}", y2:"{ty}",
                                stroke:"rgba(160,160,190,0.7)",
                                stroke_width:"1.4",
                                stroke_linecap:"round",
                                opacity:"{op}",
                            }
                        })}

                        // Node circles
                        {node_renders.iter().map(|(id, x, y, color, op, _is_sel, stroke, sw)| rsx! {
                            circle {
                                key: "{id}",
                                cx:"{x}", cy:"{y}", r:"22",
                                fill:"{color}",
                                fill_opacity:"0.82",
                                stroke:"{stroke}",
                                stroke_width:"{sw}",
                                opacity:"{op}",
                            }
                        })}

                        // Node labels (rendered on top)
                        {label_renders.iter().map(|(label, x, y, op)| rsx! {
                            text {
                                key: "{label}",
                                x:"{x}", y:"{y}",
                                text_anchor:"middle",
                                style:"font-size:10.5px; font-family:Inter,sans-serif; font-weight:500; fill:rgba(210,210,230,0.92);",
                                opacity:"{op}",
                                "{label}"
                            }
                        })}
                    }
                }

                // ── Right panels ──────────────────────────────────────────
                div {
                    style:"width:268px; flex-shrink:0; display:flex; flex-direction:column; border-left:1px solid var(--qualia-border); overflow:hidden;",

                    // Inforg Assistant
                    div {
                        style:"flex:1; display:flex; flex-direction:column; background:var(--qualia-surface); border-bottom:1px solid var(--qualia-border); min-height:0;",

                        div { style:"padding:0.55rem 0.8rem; border-bottom:1px solid var(--qualia-border); display:flex; align-items:center; gap:0.45rem; flex-shrink:0;",
                            div { style:"width:7px; height:7px; border-radius:50%; background:#8b5cf6; box-shadow:0 0 5px #8b5cf6;" }
                            span { style:"font-size:0.775rem; font-weight:600; color:var(--qualia-text);", "Inforg Assistant" }
                            span { style:"font-size:0.63rem; color:var(--qualia-text-muted); margin-left:auto;", "Local GGUF · Phase 8" }
                        }

                        // Message list
                        div {
                            style:"flex:1; overflow-y:auto; padding:0.6rem; display:flex; flex-direction:column; gap:0.45rem; min-height:0;",
                            {chat_snapshot.iter().enumerate().map(|(i, m)| {
                                let s = if m.is_user {
                                    "align-self:flex-end; background:var(--qualia-accent); color:white; border-radius:10px 10px 2px 10px; padding:0.4rem 0.65rem; font-size:0.76rem; max-width:88%; line-height:1.45;"
                                } else {
                                    "align-self:flex-start; background:rgba(128,128,128,0.1); color:var(--qualia-text); border-radius:10px 10px 10px 2px; padding:0.4rem 0.65rem; font-size:0.76rem; max-width:92%; line-height:1.45;"
                                };
                                let txt = m.text.clone();
                                rsx! { div { key:"{i}", style:"{s}", "{txt}" } }
                            })}
                        }

                        // Input row
                        div { style:"display:flex; gap:0.35rem; padding:0.45rem 0.6rem; border-top:1px solid var(--qualia-border); flex-shrink:0;",
                            input {
                                r#type:"text",
                                value:"{chat_in}",
                                placeholder:"Query the NQuin space...",
                                oninput: move |e| chat_in.set(e.value()),
                                style:"flex:1; background:rgba(128,128,128,0.07); border:1px solid var(--qualia-border); border-radius:6px; padding:0.37rem 0.55rem; color:var(--qualia-text); font-size:0.76rem; outline:none; font-family:'Inter',sans-serif;",
                            }
                            button {
                                onclick: submit_chat,
                                style:"background:var(--qualia-accent); color:white; border:none; border-radius:6px; padding:0.37rem 0.55rem; cursor:pointer; flex-shrink:0; display:flex; align-items:center;",
                                sl-icon { "name":"send", style:"font-size:0.8rem;" }
                            }
                        }
                    }

                    // Commons Gateway
                    div {
                        style:"flex-shrink:0; background:var(--qualia-surface); display:flex; flex-direction:column; max-height:210px;",

                        div { style:"padding:0.5rem 0.8rem; border-bottom:1px solid var(--qualia-border); display:flex; align-items:center; gap:0.4rem; flex-shrink:0;",
                            sl-icon { "name":"globe2", style:"font-size:0.82rem; color:var(--qualia-accent);" }
                            span { style:"font-size:0.775rem; font-weight:600; color:var(--qualia-text);", "Commons Gateway" }
                            span { style:"font-size:0.63rem; color:#10b981; margin-left:auto;", "14 peers" }
                        }

                        div { style:"padding:0.55rem 0.6rem; display:flex; flex-direction:column; gap:0.45rem; overflow-y:auto; flex:1;",
                            // Search
                            div { style:"display:flex; gap:0.3rem;",
                                input {
                                    r#type:"text",
                                    value:"{commons_q}",
                                    placeholder:"ICN hash or q42: URI...",
                                    oninput: move |e| commons_q.set(e.value()),
                                    style:"flex:1; background:rgba(128,128,128,0.07); border:1px solid var(--qualia-border); border-radius:5px; padding:0.32rem 0.5rem; color:var(--qualia-text); font-size:0.7rem; outline:none; font-family:'Inter',sans-serif;",
                                }
                                button {
                                    style:"background:rgba(128,128,128,0.1); border:1px solid var(--qualia-border); border-radius:5px; padding:0.32rem 0.5rem; cursor:pointer; color:var(--qualia-accent); font-size:0.7rem; font-family:'Inter',sans-serif; font-weight:600;",
                                    "Pull"
                                }
                            }

                            div { style:"font-size:0.65rem; color:var(--qualia-text-muted); font-weight:600; text-transform:uppercase; letter-spacing:0.05em;", "Seeded Artifacts" }

                            {[
                                ("q42:TSL-v1.3",   "Threshold Shift License", "#10b981"),
                                ("q42:PROV-O",      "Provenance Ontology",    "#0ea5e9"),
                                ("q42:ODRL-2.2",    "Rights Expression Lang.", "#8b5cf6"),
                            ].iter().map(|(hash, label, col)| rsx! {
                                div {
                                    key: "{hash}",
                                    style: "display:flex; align-items:center; gap:0.4rem; padding:0.28rem 0.38rem; background:rgba(128,128,128,0.05); border-radius:5px; border:1px solid var(--qualia-border);",
                                    div { style:"width:5px; height:5px; border-radius:50%; background:{col}; flex-shrink:0;" }
                                    div {
                                        div { style:"font-size:0.65rem; font-family:monospace; color:var(--qualia-text-muted);", "{hash}" }
                                        div { style:"font-size:0.7rem; color:var(--qualia-text);", "{label}" }
                                    }
                                }
                            })}
                        }
                    }
                }
            }

            // ── Temporal Scrubber ─────────────────────────────────────────
            div {
                style: "display:flex; align-items:center; gap:0.65rem; padding:0.45rem 1rem; background:var(--qualia-surface); border-top:1px solid var(--qualia-border); flex-shrink:0;",

                sl-icon { "name":"clock-history", style:"font-size:0.82rem; color:{scrubber_accent};" }
                span { style:"font-size:0.68rem; color:var(--qualia-text-muted); white-space:nowrap;", "2026-01-01" }

                input {
                    r#type:"range", min:"0", max:"100", value:"{t}",
                    oninput: on_time,
                    style:"flex:1; accent-color:{scrubber_accent};",
                }

                span { style:"font-size:0.68rem; color:var(--qualia-text-muted); white-space:nowrap;", "Now" }

                div {
                    style: "font-size:0.7rem; font-weight:600; padding:0.18rem 0.55rem; border-radius:5px; background:rgba(128,128,128,0.08); border:1px solid var(--qualia-border); min-width:64px; text-align:center; color:{scrubber_accent};",
                    "{time_label}"
                }

                span { style:"font-size:0.65rem; color:var(--qualia-text-muted); margin-left:0.25rem; white-space:nowrap;",
                    if t == 100 { "Live · DAG tip" } else { "AS OF · TemporalMode::AsOf" }
                }
            }
        }
    }
}
