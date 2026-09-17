//! Spreadsheet / tensor toolbar. Formula bar lives here; grid body stays in `bodies/`.

use dioxus::prelude::*;

#[component]
pub fn SheetToolbar() -> Element {
    rsx! {
        div { style: bar(),
            span { style: label(), "Sheet" }
            button { style: btn(), title: "Sum the active range via Sheet.sum_range", "Σ Sum" }
            button { style: btn(), title: "Mean / Pearson via Statistics invoke", "Stats" }
            button { style: btn(), title: "Import CSV / CBOR-LD — Present", "Import" }
        }
    }
}

fn bar() -> &'static str {
    "display:flex;flex-wrap:wrap;gap:6px;align-items:center;padding:6px 8px;background:var(--surface-panel);border:1px solid var(--border-subtle);border-radius:var(--radius-sm);"
}
fn label() -> &'static str {
    "font-size:9px;letter-spacing:.06em;text-transform:uppercase;color:var(--text-muted);"
}
fn btn() -> &'static str {
    "border:1px solid var(--border-medium);background:var(--surface-panel-elevated);color:var(--text-primary);border-radius:6px;padding:4px 8px;font-size:11px;cursor:pointer;"
}
