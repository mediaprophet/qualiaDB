//! Sheet body — `Sheet.sum_range`, `Sheet.eval_formula`, and `Sheet.pivot_analyze` via capability.invoke.

use crate::components::poet::engine::{self, PoetEvalResult};
use crate::components::poet::tools::sheet::SheetToolbar;
use dioxus::prelude::*;

const SUM: &str = r#"requires [ capability("capability.invoke") ];
effect fn sum() {
    return capability.invoke("Sheet.sum_range", { grid: [[1,2],[3,4]], range: "A1:B2" });
}
"#;

const FORMULA: &str = r#"requires [ capability("capability.invoke") ];
effect fn eval_formula() {
    return capability.invoke("Sheet.eval_formula", { formula: "=SUM(A1:B2)*1.15" });
}
"#;

const PIVOT: &str = r#"requires [ capability("capability.invoke") ];
effect fn pivot() {
    return capability.invoke("Sheet.pivot_analyze", { dimension: "region", metric: "revenue" });
}
"#;

#[component]
/// Dioxus `SheetBody` component.
pub fn SheetBody() -> Element {
    let mut out = use_signal(|| String::new());
    let mut busy = use_signal(|| false);
    rsx! {
        div { style: "display:grid;gap:8px;",
            SheetToolbar {}
            p { style: muted(), "Sheet Studio: Columnar tensor formulas, dependency DAG evaluation, and pivot aggregation via capability.invoke." }
            div { style: "display:flex;gap:6px;flex-wrap:wrap;",
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
                    "Sheet.sum_range"
                }
                button {
                    disabled: busy(),
                    style: crate::components::poet::vibe_console::secondary(),
                    onclick: move |_| {
                        busy.set(true);
                        spawn(async move {
                            match engine::eval(FORMULA.into(), false, Some("eval_formula".into())).await {
                                Ok(PoetEvalResult { ok: true, value, .. }) => out.set(value),
                                Ok(v) => out.set(v.diagnostic.unwrap_or_else(|| "rejected".into())),
                                Err(e) => out.set(e),
                            }
                            busy.set(false);
                        });
                    },
                    "Sheet.eval_formula"
                }
                button {
                    disabled: busy(),
                    style: crate::components::poet::vibe_console::secondary(),
                    onclick: move |_| {
                        busy.set(true);
                        spawn(async move {
                            match engine::eval(PIVOT.into(), false, Some("pivot".into())).await {
                                Ok(PoetEvalResult { ok: true, value, .. }) => out.set(value),
                                Ok(v) => out.set(v.diagnostic.unwrap_or_else(|| "rejected".into())),
                                Err(e) => out.set(e),
                            }
                            busy.set(false);
                        });
                    },
                    "Sheet.pivot_analyze"
                }
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
