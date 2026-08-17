//! Health body — Framingham on a labeled reference profile, not a named person.

use crate::components::poet::engine::{self, PoetEvalResult};
use crate::components::poet::gpu_frame::PoetGpuFrame;
use dioxus::prelude::*;

const RISK: &str = r#"requires [ capability("capability.invoke") ];
effect fn risk() {
    return capability.invoke("ClinicalRisk.framingham", {
        age: 55, sex_male: true, total_cholesterol_mmol: 5.2,
        hdl_cholesterol_mmol: 1.3, systolic_bp: 130
    });
}
"#;

#[component]
pub fn HealthBody() -> Element {
    let mut out = use_signal(|| String::new());
    let mut busy = use_signal(|| false);
    rsx! {
        div { style: "display:grid;gap:8px;",
            p { style: muted(), "ClinicalRisk.framingham on a reference adult profile. Not a named person. Scene node radius follows risk." }
            PoetGpuFrame { kind: "health", width: 720, height: 300 }
            button {
                disabled: busy(),
                style: crate::components::poet::vibe_console::secondary(),
                onclick: move |_| {
                    busy.set(true);
                    spawn(async move {
                        match engine::eval(RISK.into(), false, Some("risk".into())).await {
                            Ok(PoetEvalResult { ok: true, value, .. }) => out.set(value),
                            Ok(v) => out.set(v.diagnostic.unwrap_or_else(|| "rejected".into())),
                            Err(e) => out.set(e),
                        }
                        busy.set(false);
                    });
                },
                "Score reference profile"
            }
            if !out().is_empty() {
                pre { style: "font-size:.72rem;white-space:pre-wrap;margin:0;", "{out}" }
            }
        }
    }
}

fn muted() -> &'static str {
    "margin:0;color:#94a3b8;font-size:.74rem;line-height:1.45;"
}
