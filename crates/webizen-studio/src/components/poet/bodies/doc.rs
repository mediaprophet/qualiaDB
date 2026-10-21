use crate::components::poet::engine::{self, PoetEvalResult};
use crate::components::poet::tools::cml::AuraTray;
use crate::components::poet::tools::rich_text::RichTextTool;
use dioxus::prelude::*;

const DOC_LAYOUT: &str = r#"requires [ capability("capability.invoke") ];
effect fn layout_doc() {
    return capability.invoke("Document.layout_paragraph", { text: "HyperCanvas living document paragraph layout with Knuth-Plass line breaking.", measure_pt: 432.0 });
}
"#;

const DOC_PROOF: &str = r#"requires [ capability("capability.invoke") ];
effect fn proof_doc() {
    return capability.invoke("Document.proof_check", { text: "Verify epistemic clarity and citation grounding." });
}
"#;

#[component]
/// Dioxus `DocBody` component.
pub fn DocBody() -> Element {
    let mut out = use_signal(|| String::new());
    let mut busy = use_signal(|| false);

    rsx! {
        div { style: "display:flex;flex-direction:column;gap:6px;flex:1;",
            RichTextTool {}
            AuraTray {}
            div { style: "display:flex;gap:6px;flex-wrap:wrap;padding:2px 4px;",
                button {
                    disabled: busy(),
                    style: crate::components::poet::vibe_console::secondary(),
                    onclick: move |_| {
                        busy.set(true);
                        spawn(async move {
                            match engine::eval(DOC_LAYOUT.into(), false, Some("layout_doc".into())).await {
                                Ok(PoetEvalResult { ok: true, value, .. }) => out.set(value),
                                Ok(v) => out.set(v.diagnostic.unwrap_or_else(|| "rejected".into())),
                                Err(e) => out.set(e),
                            }
                            busy.set(false);
                        });
                    },
                    "Document.layout_paragraph"
                }
                button {
                    disabled: busy(),
                    style: crate::components::poet::vibe_console::secondary(),
                    onclick: move |_| {
                        busy.set(true);
                        spawn(async move {
                            match engine::eval(DOC_PROOF.into(), false, Some("proof_doc".into())).await {
                                Ok(PoetEvalResult { ok: true, value, .. }) => out.set(value),
                                Ok(v) => out.set(v.diagnostic.unwrap_or_else(|| "rejected".into())),
                                Err(e) => out.set(e),
                            }
                            busy.set(false);
                        });
                    },
                    "Document.proof_check"
                }
            }
            if !out().is_empty() {
                pre { style: "font-size:.72rem;white-space:pre-wrap;margin:0;padding:4px;font-family:var(--font-mono);color:#38bdf8;", "{out}" }
            }
        }
    }
}
