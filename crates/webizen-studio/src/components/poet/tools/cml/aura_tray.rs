//! Aura Tray Dioxus Component (<q-aura-tray>)
//!
//! Real-time footer for <q-doc> displaying SHACL conformance, certainty percentage,
//! Super-Quin slot expansion count, and .hcf container export action.
//!
//! Aligned with `06_HYPERMEDIA_LIBRARY_FOUNDATION_SPEC.md` and `POET-SPEC-006`.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use dioxus::prelude::*;

#[component]
pub fn AuraTray(
    #[props(default = true)] conformant: bool,
    #[props(default = 98)] certainty: u8,
    #[props(default = 48)] super_quins_count: usize,
) -> Element {
    let status_color = if conformant { "var(--accent-emerald, #00f2a9)" } else { "var(--accent-rose, #ef4444)" };
    let status_text = if conformant { "Full Conformance" } else { "SHACL Violations" };

    rsx! {
        div {
            class: "poet-aura-tray",
            style: "display:flex;align-items:center;justify-content:space-between;padding:4px 8px;background:var(--surface-panel);border:1px solid var(--border-subtle);border-radius:var(--radius-xs);font-family:var(--font-mono);font-size:10px;margin-top:6px;",
            
            // Left: SHACL Conformance & Status
            div { style: "display:flex;align-items:center;gap:6px;",
                span {
                    style: "display:inline-block;width:7px;height:7px;border-radius:50%;background:{status_color};box-shadow:0 0 6px {status_color};",
                }
                span { style: "font-weight:600;color:{status_color};", "SHACL: {status_text}" }
                span { style: "color:var(--text-muted);", "·" }
                span { style: "color:var(--text-secondary);", "Certainty: {certainty}%" }
            }

            // Right: Super-Quin Count & Export Action
            div { style: "display:flex;align-items:center;gap:8px;",
                span { style: "color:var(--text-muted);", "🧬 {super_quins_count} Quins" }
                button {
                    r#type: "button",
                    style: "padding:2px 6px;background:var(--surface-panel-elevated);border:1px solid var(--border-subtle);border-radius:3px;color:var(--accent-cyan);font-size:9px;cursor:pointer;",
                    "📦 Export .hcf"
                }
            }
        }
    }
}
