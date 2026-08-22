//! Job center body — Background task queue & telemetry HUD.

use crate::components::poet::engine::{self, PoetEvalResult};
use dioxus::prelude::*;

const QUEUE_STATS: &str = r#"requires [ capability("capability.invoke") ];
effect fn queue_stats() {
    return capability.invoke("Jobs.stats", {});
}
"#;

#[component]
pub fn JobCenterBody() -> Element {
    let mut out = use_signal(|| String::new());
    let mut busy = use_signal(|| false);

    rsx! {
        div { style: "display:grid;gap:8px;padding:8px;",
            p { style: muted(), "Webizen Job Centre: Async task state machine, progress telemetry HUD, and ambient background worker supervision." }
            div { style: "display:flex;gap:8px;flex-wrap:wrap;",
                button {
                    disabled: busy(),
                    style: crate::components::poet::vibe_console::secondary(),
                    onclick: move |_| {
                        busy.set(true);
                        spawn(async move {
                            match engine::eval(QUEUE_STATS.into(), false, Some("queue_stats".into())).await {
                                Ok(PoetEvalResult { ok: true, value, .. }) => out.set(value),
                                Ok(v) => out.set(v.diagnostic.unwrap_or_else(|| "rejected".into())),
                                Err(e) => out.set(e),
                            }
                            busy.set(false);
                        });
                    },
                    "Jobs.stats"
                }
            }
            if !out().is_empty() {
                pre { style: "font-size:.72rem;white-space:pre-wrap;margin:0;font-family:var(--font-mono);color:#a78bfa;", "{out}" }
            }
        }
    }
}

fn muted() -> &'static str {
    "margin:0;color:#94a3b8;font-size:.74rem;line-height:1.45;"
}
