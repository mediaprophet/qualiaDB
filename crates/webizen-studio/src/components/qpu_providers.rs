use dioxus::prelude::*;

#[component]
pub fn QpuProviders() -> Element {
    rsx! {
        div {
            style: "flex: 1; padding: 2rem; background: var(--qualia-surface); border-radius: 12px; color: var(--qualia-text);",
            h2 { "QpuProviders Pending Implementation" }
            p { "This module is being developed by a specialized agent." }
        }
    }
}
