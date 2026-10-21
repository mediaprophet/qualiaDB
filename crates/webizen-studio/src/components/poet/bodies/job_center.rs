//! Job center body — Background task queue & telemetry HUD via `Job.list`.

use crate::components::poet::engine::{self, PoetEvalResult};
use dioxus::prelude::*;

const JOB_LIST_SRC: &str = r#"requires [ capability("capability.invoke") ];
effect fn list_jobs() {
    return capability.invoke("Job.list", {});
}
"#;

#[component]
/// Dioxus `JobCenterBody` component.
pub fn JobCenterBody() -> Element {
    let mut out = use_signal(|| String::new());
    let mut busy = use_signal(|| false);

    rsx! {
        div { style: "display:grid;gap:8px;padding:8px;",
            p { style: muted(), "Webizen Job Centre: Async task state machine, progress telemetry HUD, and 42MB Sentinel budget supervision." }
            div { style: "display:flex;gap:8px;flex-wrap:wrap;",
                button {
                    disabled: busy(),
                    style: crate::components::poet::vibe_console::secondary(),
                    onclick: move |_| {
                        busy.set(true);
                        spawn(async move {
                            match engine::eval(JOB_LIST_SRC.into(), false, Some("list_jobs".into())).await {
                                Ok(PoetEvalResult { ok: true, value, .. }) => out.set(value),
                                Ok(v) => out.set(v.diagnostic.unwrap_or_else(|| "rejected".into())),
                                Err(e) => out.set(e),
                            }
                            busy.set(false);
                        });
                    },
                    "Job.list"
                }
            }
            if !out().is_empty() {
                pre { style: "font-size:.72rem;white-space:pre-wrap;margin:0;font-family:var(--font-mono);color:#38bdf8;", "{out}" }
            }
        }
    }
}

fn muted() -> &'static str {
    "margin:0;color:#94a3b8;font-size:.74rem;line-height:1.45;"
}
