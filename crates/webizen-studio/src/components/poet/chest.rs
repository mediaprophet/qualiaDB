//! 4-way tool chest + floating palette — `toolbox-registry.js` structure.

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
                        honesty: "present",
                    },
                    ToolSpec {
                        label: "🧠 Tag Subjective Qualia",
                        places: None,
                        honesty: "present",
                    },
                    ToolSpec {
                        label: "🌊 Tag Intersubjective Agreement",
                        places: None,
                        honesty: "present",
                    },
                    ToolSpec {
                        label: "⚖️ Tag Normative Mandate",
                        places: None,
                        honesty: "present",
                    },
                ],
            )],
            Self::Office => &[(
                "Containers & Templates",
                &[
                    ToolSpec {
                        label: "+ Document Container",
                        places: Some(ContainerKind::Doc),
                        honesty: "live",
                    },
                    ToolSpec {
                        label: "+ Ontology Mapping Node",
                        places: Some(ContainerKind::Ontology),
                        honesty: "live",
                    },
                ],
            )],
            Self::Image => &[(
                "Generative Assets",
                &[ToolSpec {
                    label: "+ Visual Asset Card",
                    places: Some(ContainerKind::Media),
                    honesty: "live",
                }],
            )],
            Self::Sheet => &[(
                "Data Containers",
                &[ToolSpec {
                    label: "+ Columnar Tensor Sheet",
                    places: Some(ContainerKind::Sheet),
                    honesty: "live",
                }],
            )],
            Self::Spatial => &[(
                "Spatial & GIS Containers",
                &[
                    ToolSpec {
                        label: "+ Geospatial Case Map",
                        places: Some(ContainerKind::Map),
                        honesty: "live",
                    },
                    ToolSpec {
                        label: "+ 3D Mesh Container",
                        places: Some(ContainerKind::Mesh3d),
                        honesty: "present",
                    },
                    ToolSpec {
                        label: "+ Wormhole Portal",
                        places: Some(ContainerKind::Portal),
                        honesty: "present",
                    },
                    ToolSpec {
                        label: "+ Sub-manifold",
                        places: Some(ContainerKind::Subcanvas),
                        honesty: "partial",
                    },
                ],
            )],
            Self::Communication => &[(
                "Collaborative Containers",
                &[
                    ToolSpec {
                        label: "+ Social AI Chat Graph",
                        places: Some(ContainerKind::Social),
                        honesty: "live",
                    },
                    ToolSpec {
                        label: "+ WebRTC Live Video",
                        places: Some(ContainerKind::WebRtc),
                        honesty: "present",
                    },
                    ToolSpec {
                        label: "+ Webview Browser Frame",
                        places: Some(ContainerKind::Webview),
                        honesty: "present",
                    },
                ],
            )],
            Self::Rights => &[(
                "Permission Groups & Scopes",
                &[ToolSpec {
                    label: "✍️ Sign with DID Key",
                    places: None,
                    honesty: "present",
                }],
            )],
            Self::Health => &[(
                "Clinical Containers",
                &[ToolSpec {
                    label: "+ Health Telemetry Node",
                    places: Some(ContainerKind::Health),
                    honesty: "live",
                }],
            )],
            Self::Code => &[(
                "DSL Containers & Cells",
                &[ToolSpec {
                    label: "+ VibeScript Container",
                    places: Some(ContainerKind::Code),
                    honesty: "live",
                }],
            )],
            Self::Ai => &[(
                "Mindware Co-Authors & Agents",
                &[ToolSpec {
                    label: "🤖 LLM Semantic Co-Author",
                    places: None,
                    honesty: "present",
                }],
            )],
        }
    }
}
