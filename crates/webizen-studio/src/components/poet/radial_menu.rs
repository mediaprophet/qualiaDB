//! 8-Sector Radial Action Ring Dioxus Component
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use super::kinds::ContainerKind;
use super::store::Workbench;
use dioxus::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RadialState {
    pub visible: bool,
    pub x: f64,
    pub y: f64,
}

impl Default for RadialState {
    fn default() -> Self {
        Self {
            visible: false,
            x: 0.0,
            y: 0.0,
        }
    }
}

pub struct RadialSector {
    pub label: &'static str,
    pub icon: &'static str,
    pub color: &'static str,
}

pub const SECTORS: [RadialSector; 8] = [
    RadialSector {
        label: "Inspect",
        icon: "🔍",
        color: "var(--accent-cyan, #00C8FF)",
    },
    RadialSector {
        label: "Connect Wire",
        icon: "⚡",
        color: "var(--accent-gold, #F5A623)",
    },
    RadialSector {
        label: "Clip Tray",
        icon: "🧺",
        color: "var(--accent-teal, #50E3C2)",
    },
    RadialSector {
        label: "Export .hcf",
        icon: "📦",
        color: "var(--accent-purple, #9B51E0)",
    },
    RadialSector {
        label: "Duplicate",
        icon: "📑",
        color: "var(--accent-blue, #4A90E2)",
    },
    RadialSector {
        label: "Snap 8px",
        icon: "📐",
        color: "var(--accent-orange, #FF7640)",
    },
    RadialSector {
        label: "Vibe REPL",
        icon: "💻",
        color: "var(--accent-green, #7ED321)",
    },
    RadialSector {
        label: "Delete",
        icon: "🗑️",
        color: "var(--accent-red, #D0021B)",
    },
];

#[component]
pub fn RadialActionRing(wb: Signal<Workbench>, state: Signal<RadialState>) -> Element {
    let s = state();
    if !s.visible {
        return rsx! {};
    }

    let cx = s.x;
    let cy = s.y;
    let r_inner = 40.0;
    let r_outer = 110.0;
    let center = 120.0;

    rsx! {
        div {
            class: "radial-action-ring-overlay",
            style: "position:fixed;left:{cx}px;top:{cy}px;width:240px;height:240px;transform:translate(-50%,-50%);z-index:9999;pointer-events:auto;",
            onclick: move |e| {
                e.stop_propagation();
                let mut st = state();
                st.visible = false;
                state.set(st);
            },
            svg {
                view_box: "0 0 240 240",
                width: "100%",
                height: "100%",
                style: "filter:drop-shadow(0 8px 24px rgba(0,0,0,0.6));",
                for (i, sector) in SECTORS.iter().enumerate() {
                    {
                        let start_deg = (i as f64) * 45.0 - 22.5;
                        let end_deg = start_deg + 44.0;
                        let start_rad = start_deg.to_radians();
                        let end_rad = end_deg.to_radians();

                        let x1_in = center + r_inner * start_rad.cos();
                        let y1_in = center + r_inner * start_rad.sin();
                        let x2_in = center + r_inner * end_rad.cos();
                        let y2_in = center + r_inner * end_rad.sin();

                        let x1_out = center + r_outer * start_rad.cos();
                        let y1_out = center + r_outer * start_rad.sin();
                        let x2_out = center + r_outer * end_rad.cos();
                        let y2_out = center + r_outer * end_rad.sin();

                        let path_d = format!(
                            "M {:.2} {:.2} L {:.2} {:.2} A {:.2} {:.2} 0 0 1 {:.2} {:.2} L {:.2} {:.2} A {:.2} {:.2} 0 0 0 {:.2} {:.2} Z",
                            x1_in, y1_in, x1_out, y1_out, r_outer, r_outer, x2_out, y2_out, x2_in, y2_in, r_inner, r_inner, x1_in, y1_in
                        );

                        let mid_rad = ((start_deg + end_deg) / 2.0).to_radians();
                        let r_mid = (r_inner + r_outer) / 2.0;
                        let mid_x = center + r_mid * mid_rad.cos();
                        let mid_y = center + r_mid * mid_rad.sin();

                        rsx! {
                            g {
                                key: "{i}",
                                class: "radial-sector",
                                style: "cursor:pointer;",
                                onclick: move |e| {
                                    e.stop_propagation();
                                    let mut st = state();
                                    st.visible = false;
                                    state.set(st);

                                    let mut w = wb();
                                    match i {
                                        0 => { w.sidebar = !w.sidebar; }, // Inspect
                                        5 => { // Snap 8px
                                            if let Some(id) = w.selected.clone() {
                                                if let Some(n) = w.node_mut(&id) {
                                                    n.x = (n.x / 8.0).round() * 8.0;
                                                    n.y = (n.y / 8.0).round() * 8.0;
                                                }
                                            }
                                        },
                                        6 => { w.place(ContainerKind::Code); }, // Vibe REPL
                                        7 => { // Delete
                                            if let Some(id) = w.selected.clone() {
                                                w.close(&id);
                                            }
                                        },
                                        _ => {}
                                    }
                                    wb.set(w);
                                },
                                path {
                                    d: "{path_d}",
                                    fill: "rgba(15, 20, 28, 0.92)",
                                    stroke: "{sector.color}",
                                    stroke_width: "1.2",
                                }
                                text {
                                    x: "{mid_x}",
                                    y: "{mid_y + 4.0}",
                                    text_anchor: "middle",
                                    font_size: "14",
                                    fill: "#FFFFFF",
                                    "{sector.icon}"
                                }
                            }
                        }
                    }
                }
                // Center POET circle
                circle {
                    cx: "{center}",
                    cy: "{center}",
                    r: "{r_inner - 2.0}",
                    fill: "rgba(8, 12, 18, 0.96)",
                    stroke: "var(--accent-cyan, #00C8FF)",
                    stroke_width: "1.5",
                }
                text {
                    x: "{center}",
                    y: "{center + 4.0}",
                    text_anchor: "middle",
                    font_size: "11",
                    font_weight: "700",
                    fill: "var(--accent-cyan, #00C8FF)",
                    "POET"
                }
            }
        }
    }
}
