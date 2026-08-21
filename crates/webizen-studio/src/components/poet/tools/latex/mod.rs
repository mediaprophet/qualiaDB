//! LaTeX / CAS toolbar. Wraps SymbolicAlgebra; does not embed a third-party editor.

use dioxus::prelude::*;

#[component]
pub fn LatexToolbar() -> Element {
    rsx! {
        div { style: bar(),
            span { style: label(), "TeX" }
            button { style: btn(), title: "Inline math $…$", "$ $" }
            button { style: btn(), title: "Display math", "$$" }
            button { style: btn(), title: "SymbolicAlgebra.eval via Vibe — Present until wired", "CAS" }
        }
    }
}

#[allow(dead_code)]
fn bar() -> &'static str {
    "display:flex;flex-wrap:wrap;gap:6px;align-items:center;padding:6px 8px;background:var(--surface-panel);border:1px solid var(--border-subtle);border-radius:var(--radius-sm);"
}
#[allow(dead_code)]
fn label() -> &'static str {
    "font-size:9px;letter-spacing:.06em;text-transform:uppercase;color:var(--text-muted);"
}
#[allow(dead_code)]
fn btn() -> &'static str {
    "border:1px solid var(--border-medium);background:var(--surface-panel-elevated);color:var(--text-primary);border-radius:6px;padding:4px 8px;font-size:11px;cursor:pointer;font-family:var(--font-mono);"
}
