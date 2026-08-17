//! Sheet body — `Sheet.sum_range` via capability.invoke.

use crate::components::poet::engine::{self, PoetEvalResult};
use crate::components::poet::tools::sheet::SheetToolbar;
use dioxus::prelude::*;

const SUM: &str = r#"requires [ capability("capability.invoke") ];
effect fn sum() {
    return capability.invoke("Sheet.sum_range", { grid: [[1,2],[3,4]], range: "A1:B2" });
}
"#;

#[component]
pub fn SheetBody() -> Element {
    let mut out = use_signal(|| String::new());
    let mut busy = use_signal(|| false);
    rsx! {
        div { style: "display:grid;gap:8px;",
            SheetToolbar {}
            p { style: muted(), "Sheet.sum_range via capability.invoke. CML cell→N3 is later." }
            button {
                disabled: busy(),
                style: crate::components::poet::vibe_console::secondary(),
                onclick: move |_| {
                    busy.set(true);
                    spawn(async move {
                        match engine::eval(SUM.into(), false, Some("sum".into())).await {
                            Ok(PoetEvalResult { ok: true, value, .. }) => out.set(value),
                            Ok(v) => out.set(v.diagnostic.unwrap_or_else(|| "rejected".into())),
                            Err(e) => out.set(e),
                        }
                        busy.set(false);
                    });
                },
                "Sum A1:B2 demo grid"
            }
            if !out().is_empty() {
                pre { style: "font-size:.76rem;white-space:pre-wrap;margin:0;", "{out}" }
            }
        }
    }
}

fn muted() -> &'static str {
    "margin:0;color:#94a3b8;font-size:.74rem;line-height:1.45;"
}
