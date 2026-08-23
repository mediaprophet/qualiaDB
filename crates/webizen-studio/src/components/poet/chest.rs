//! 4-way tool chest + floating palette — `toolbox-registry.js` structure.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use super::kinds::{ContainerKind, DockPos, ToolSpec, ToolboxId};
use super::store::Workbench;
use dioxus::prelude::*;

#[component]
pub fn ToolChest(wb: Signal<Workbench>) -> Element {
    let w = wb();
    let pos = w.dock.id();
    let open = w.open_box;
    rsx! {
        aside { class: "toolbox-dock dock-{pos}", id: "toolbox-dock",
            div { class: "dock-header-controls",
                for d in [DockPos::Left, DockPos::Top, DockPos::Bottom, DockPos::Right] {
                    button {
                        class: if w.dock == d { "dock-pos-btn active" } else { "dock-pos-btn" },
                        title: "Dock position",
                        onclick: move |_| { let mut s = wb(); s.dock = d; wb.set(s); },
                        match d {
                            DockPos::Left => "◀",
                            DockPos::Top => "▲",
                            DockPos::Bottom => "▼",
                            DockPos::Right => "▶",
                        }
                    }
                }
            }
            for tb in ToolboxId::ALL {
                button {
                    class: if open == Some(tb) { "toolbox-dock-btn active" } else { "toolbox-dock-btn" },
                    title: "{tb.title()}",
                    onclick: move |_| {
                        let mut s = wb();
                        s.open_box = if s.open_box == Some(tb) { None } else { Some(tb) };
                        wb.set(s);
                    },
                    "{tb.icon()}"
                }
            }
        }
        if let Some(tb) = open {
            Palette { wb, toolbox: tb }
        }
    }
}

#[component]
fn Palette(wb: Signal<Workbench>, toolbox: ToolboxId) -> Element {
    let pos = wb().dock.id();
    let mut selected_font = use_signal(|| "Inter".to_string());
    let mut selected_size = use_signal(|| "14px".to_string());
    let mut selected_color = use_signal(|| "#38bdf8".to_string());
    let mut brush_size = use_signal(|| 8.0);
    let mut brush_type = use_signal(|| "Round".to_string());

    rsx! {
        div { class: "floating-toolbox-palette dock-{pos} open", id: "floating-toolbox-palette",
            div { class: "palette-header",
                span { "{toolbox.icon()} {toolbox.title()}" }
                button { class: "container-action-btn",
                    onclick: move |_| { let mut s = wb(); s.open_box = None; wb.set(s); },
                    "×"
                }
            }
            div { class: "palette-body",
                // Specialized domain widgets
                if toolbox == ToolboxId::Office {
                    div { class: "tool-section", style: "background:rgba(255,255,255,0.02);padding:8px;border-radius:6px;margin-bottom:8px;display:grid;gap:6px;",
                        span { class: "tool-section-title", "Typography & Styling" }
                        div { style: "display:flex;gap:6px;align-items:center;",
                            select {
                                style: "flex:1;background:#141a23;color:var(--text-primary);border:1px solid rgba(255,255,255,0.1);border-radius:4px;padding:3px 6px;font-size:11px;",
                                value: "{selected_font}",
                                onchange: move |e| selected_font.set(e.value()),
                                option { value: "Inter", "Inter (Sans)" }
                                option { value: "Fira Code", "Fira Code (Mono)" }
                                option { value: "Merriweather", "Merriweather (Serif)" }
                                option { value: "JetBrains Mono", "JetBrains Mono" }
                            }
                            select {
                                style: "width:70px;background:#141a23;color:var(--text-primary);border:1px solid rgba(255,255,255,0.1);border-radius:4px;padding:3px 6px;font-size:11px;",
                                value: "{selected_size}",
                                onchange: move |e| selected_size.set(e.value()),
                                option { value: "11px", "11px" }
                                option { value: "13px", "13px" }
                                option { value: "14px", "14px" }
                                option { value: "16px", "16px" }
                                option { value: "20px", "20px" }
                            }
                        }
                    }
                } else if toolbox == ToolboxId::Image {
                    div { class: "tool-section", style: "background:rgba(255,255,255,0.02);padding:8px;border-radius:6px;margin-bottom:8px;display:grid;gap:6px;",
                        span { class: "tool-section-title", "Brush & Palette" }
                        div { style: "display:flex;gap:6px;align-items:center;",
                            select {
                                style: "flex:1;background:#141a23;color:var(--text-primary);border:1px solid rgba(255,255,255,0.1);border-radius:4px;padding:3px 6px;font-size:11px;",
                                value: "{brush_type}",
                                onchange: move |e| brush_type.set(e.value()),
                                option { value: "Round", "🖌️ Round Brush" }
                                option { value: "Flat", "🖊️ Flat Chisel" }
                                option { value: "Marker", "🖍️ Marker" }
                                option { value: "Airbrush", "💨 Airbrush" }
                            }
                            span { style: "font-size:11px;color:var(--text-muted);", "{brush_size()}px" }
                        }
                        input {
                            r#type: "range", min: "1", max: "64", value: "{brush_size}",
                            style: "width:100%;accent-color:var(--accent-cyan);",
                            oninput: move |e| {
                                if let Ok(v) = e.value().parse::<f64>() {
                                    brush_size.set(v);
                                }
                            }
                        }
                        div { style: "display:flex;gap:6px;margin-top:4px;",
                            for color in ["#38bdf8", "#00E676", "#fbbf24", "#f43f5e", "#a855f7", "#ffffff", "#000000"] {
                                button {
                                    style: format!(
                                        "width:20px;height:20px;border-radius:50%;background:{};border:{};cursor:pointer;",
                                        color,
                                        if selected_color() == color { "2px solid #ffffff" } else { "1px solid rgba(255,255,255,0.2)" }
                                    ),
                                    onclick: move |_| selected_color.set(color.to_string()),
                                }
                            }
                        }
                    }
                }

                for (section, tools) in toolbox.sections() {
                    div { class: "tool-section",
                        span { class: "tool-section-title", "{section}" }
                        div { class: "tool-grid",
                            for tool in tools {
                                button { class: "tool-btn",
                                    onclick: move |_| {
                                        let mut s = wb();
                                        if let Some(kind) = tool.places {
                                            s.place(kind);
                                        }
                                        wb.set(s);
                                    },
                                    "{tool.label}"
                                    if tool.honesty != "live" {
                                        span { style: "color:var(--accent-amber);font-size:9px;margin-left:6px;", "{tool.honesty}" }
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

impl ToolboxId {
    pub fn sections(self) -> &'static [(&'static str, &'static [ToolSpec])] {
        match self {
            Self::Epistemic => &[(
                "Objective vs Subjective Grounding",
                &[
                    ToolSpec {
                        label: "🔬 Tag Objective Telemetry",
                        places: None,
                        honesty: "live",
                    },
                    ToolSpec {
                        label: "🧠 Tag Subjective Qualia",
                        places: None,
                        honesty: "live",
                    },
                    ToolSpec {
                        label: "🌊 Tag Intersubjective Agreement",
                        places: None,
                        honesty: "live",
                    },
                    ToolSpec {
                        label: "⚖️ Tag Normative Mandate",
                        places: None,
                        honesty: "live",
                    },
                ],
            )],
            Self::Office => &[(
                "Document & CML Tools",
                &[
                    ToolSpec {
                        label: "+ HyperDoc Container",
                        places: Some(ContainerKind::Doc),
                        honesty: "live",
                    },
                    ToolSpec {
                        label: "🏷️ Insert CML Entity (<q-entity>)",
                        places: None,
                        honesty: "live",
                    },
                    ToolSpec {
                        label: "🔗 Insert Relation (<q-relation>)",
                        places: None,
                        honesty: "live",
                    },
                    ToolSpec {
                        label: "📜 Insert Citation (<q-citation>)",
                        places: None,
                        honesty: "live",
                    },
                ],
            )],
            Self::Image => &[(
                "Visual & Graphics Tools",
                &[
                    ToolSpec {
                        label: "+ Media Studio Container",
                        places: Some(ContainerKind::Media),
                        honesty: "live",
                    },
                    ToolSpec {
                        label: "✒️ Vector Pen / Bezier",
                        places: None,
                        honesty: "present",
                    },
                    ToolSpec {
                        label: "🖌️ Brush & Color Palette",
                        places: None,
                        honesty: "present",
                    },
                ],
            )],
            Self::Sheet => &[(
                "Spreadsheets & Vibe Formulas",
                &[
                    ToolSpec {
                        label: "+ Columnar Tensor Sheet",
                        places: Some(ContainerKind::Sheet),
                        honesty: "live",
                    },
                    ToolSpec {
                        label: "⚡ Insert Vibe Formula (=VIBE)",
                        places: None,
                        honesty: "live",
                    },
                    ToolSpec {
                        label: "📈 Generate Quick Chart",
                        places: None,
                        honesty: "present",
                    },
                ],
            )],
            Self::Spatial => &[(
                "3D Kinematics & Anatomy",
                &[
                    ToolSpec {
                        label: "+ 3D Anatomy Mesh (.10d)",
                        places: Some(ContainerKind::Mesh3d),
                        honesty: "live",
                    },
                    ToolSpec {
                        label: "+ Geospatial Case Map",
                        places: Some(ContainerKind::Map),
                        honesty: "live",
                    },
                    ToolSpec {
                        label: "+ Wormhole Portal",
                        places: Some(ContainerKind::Portal),
                        honesty: "present",
                    },
                    ToolSpec {
                        label: "+ Sub-Manifold Canvas",
                        places: Some(ContainerKind::Subcanvas),
                        honesty: "partial",
                    },
                ],
            )],
            Self::Audio => &[(
                "Triad Synthesis & Audio",
                &[
                    ToolSpec {
                        label: "+ Triad Formant Synthesizer",
                        places: Some(ContainerKind::Media),
                        honesty: "live",
                    },
                    ToolSpec {
                        label: "🎙️ Mic Capture (PCM Stream)",
                        places: None,
                        honesty: "live",
                    },
                    ToolSpec {
                        label: "🫀 Neural Audio Latents (P64)",
                        places: None,
                        honesty: "live",
                    },
                ],
            )],
            Self::Code => &[(
                "VibeScript IDE & Shaders",
                &[
                    ToolSpec {
                        label: "+ VibeScript IDE Container",
                        places: Some(ContainerKind::Code),
                        honesty: "live",
                    },
                    ToolSpec {
                        label: "▶ Run Reactive Cell (<q-cell>)",
                        places: None,
                        honesty: "live",
                    },
                    ToolSpec {
                        label: "🛡️ Check Capability Manifest",
                        places: None,
                        honesty: "live",
                    },
                    ToolSpec {
                        label: "+ Distributed Git Forge",
                        places: Some(ContainerKind::GitForge),
                        honesty: "live",
                    },
                ],
            )],
            Self::Erp => &[(
                "Cooperative ERP & Workstream A",
                &[
                    ToolSpec {
                        label: "+ Cooperative Kanban Board",
                        places: Some(ContainerKind::ErpKanban),
                        honesty: "live",
                    },
                    ToolSpec {
                        label: "📊 Gantt Timeline Cascade",
                        places: None,
                        honesty: "present",
                    },
                    ToolSpec {
                        label: "🗳️ M-of-N Voting Ballot",
                        places: None,
                        honesty: "live",
                    },
                ],
            )],
            Self::Mail => &[(
                "Inalienable Domain Communications",
                &[
                    ToolSpec {
                        label: "+ Inalienable Mail Inbox",
                        places: Some(ContainerKind::Mail),
                        honesty: "live",
                    },
                    ToolSpec {
                        label: "✉️ CML Mail Composer",
                        places: None,
                        honesty: "live",
                    },
                    ToolSpec {
                        label: "🌐 Web Site Publisher",
                        places: None,
                        honesty: "live",
                    },
                ],
            )],
            Self::Scientific => &[(
                "Clinical & Physics Labs",
                &[
                    ToolSpec {
                        label: "+ Health & Clinical Node",
                        places: Some(ContainerKind::Health),
                        honesty: "live",
                    },
                    ToolSpec {
                        label: "🧪 Molecular 3D Viewer",
                        places: None,
                        honesty: "present",
                    },
                    ToolSpec {
                        label: "📈 Thermodynamics MCMC",
                        places: None,
                        honesty: "live",
                    },
                ],
            )],
            Self::Ai => &[(
                "Local AI Mindware & Co-Pilots",
                &[
                    ToolSpec {
                        label: "🤖 Resident GGUF Model Co-Pilot",
                        places: None,
                        honesty: "live",
                    },
                    ToolSpec {
                        label: "📜 Deontic Rule Assistant",
                        places: None,
                        honesty: "live",
                    },
                    ToolSpec {
                        label: "👥 AI Agent Swarm Dispatch",
                        places: None,
                        honesty: "present",
                    },
                ],
            )],
            Self::Rights => &[(
                "Inalienable Rights & Sanctuary",
                &[
                    ToolSpec {
                        label: "✍️ Sign with Root DID Key",
                        places: None,
                        honesty: "live",
                    },
                    ToolSpec {
                        label: "🔒 Sanctuary Decoy Vault",
                        places: None,
                        honesty: "live",
                    },
                    ToolSpec {
                        label: "🤝 AgreementDID M-of-N Signer",
                        places: None,
                        honesty: "live",
                    },
                ],
            )],
            Self::Communication => &[(
                "Social & P2P WebRTC",
                &[
                    ToolSpec {
                        label: "+ Social AI Chat Graph",
                        places: Some(ContainerKind::Social),
                        honesty: "live",
                    },
                    ToolSpec {
                        label: "+ WebRTC P2P Data Channel",
                        places: Some(ContainerKind::WebRtc),
                        honesty: "live",
                    },
                    ToolSpec {
                        label: "+ Webview Browser Frame",
                        places: Some(ContainerKind::Webview),
                        honesty: "live",
                    },
                ],
            )],
            Self::Health => &[(
                "Health & Sensory Telemetry",
                &[
                    ToolSpec {
                        label: "+ Health Telemetry Node",
                        places: Some(ContainerKind::Health),
                        honesty: "live",
                    },
                ],
            )],
            Self::Sdn => &[(
                "SDN & Cooperative Economics",
                &[
                    ToolSpec {
                        label: "🌐 WebTorrent Swarm Seeder",
                        places: None,
                        honesty: "live",
                    },
                    ToolSpec {
                        label: "💰 Unit Economics Modeler",
                        places: None,
                        honesty: "live",
                    },
                    ToolSpec {
                        label: "⚡ Battery & Solar Governor",
                        places: None,
                        honesty: "live",
                    },
                ],
            )],
        }
    }
}
