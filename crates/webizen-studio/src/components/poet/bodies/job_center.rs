//! Job center body — Background task queue & telemetry HUD.
//!
//! No Job.* family in ALL_BOUND. Do not invent Host methods. Empty is honest held.

use dioxus::prelude::*;

#[component]
pub fn JobCenterBody() -> Element {
    rsx! {
        div { style: "display:grid;gap:8px;padding:8px;",
            p { class: "held-bind-note", "Job · held / not yet — no live job-queue bind." }
            p { style: muted(), "Webizen Job Centre: Async task state machine, progress telemetry HUD, and ambient background worker supervision. No Job.* Host bind — chrome stays held." }
        }
    }
}

fn muted() -> &'static str {
    "margin:0;color:#94a3b8;font-size:.74rem;line-height:1.45;"
}
