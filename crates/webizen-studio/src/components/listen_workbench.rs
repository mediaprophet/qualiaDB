//! Listen workbench — Ears MVP surface (synthetic demo via desktop).

use dioxus::prelude::*;

use super::qapp_engine::invoke_json;

#[component]
pub fn ListenWorkbench() -> Element {
    let mut status = use_signal(|| {
        String::from("Run Ears demo: synthetic tone → features → reference events → epistemic quins.")
    });
    let mut busy = use_signal(|| false);
    let mut detail = use_signal(|| String::new());

    let run = {
        let mut status = status.clone();
        let mut busy = busy.clone();
        let mut detail = detail.clone();
        move |_| {
            busy.set(true);
            status.set("Running Ears demo…".into());
            spawn(async move {
                match invoke_json(
                    "audio_ears_demo",
                    serde_json::json!({ "persist": true }),
                )
                .await
                {
                    Ok(v) => {
                        detail.set(format!("{v}"));
                        status.set(format!(
                            "OK events={} quins={} ref={} — {}",
                            v.get("n_events").and_then(|x| x.as_u64()).unwrap_or(0),
                            v.get("n_quins").and_then(|x| x.as_u64()).unwrap_or(0),
                            v.get("is_reference")
                                .and_then(|x| x.as_bool())
                                .unwrap_or(true),
                            v.get("note").and_then(|x| x.as_str()).unwrap_or("")
                        ));
                    }
                    Err(e) => status.set(format!(
                        "Native audio unavailable ({e}). Open in desktop shell."
                    )),
                }
                busy.set(false);
            });
        }
    };

    let run_x = {
        let mut status = status.clone();
        let mut busy = busy.clone();
        move |_| {
            busy.set(true);
            spawn(async move {
                match invoke_json("audio_cross_modal_demo", serde_json::json!({})).await {
                    Ok(v) => status.set(format!(
                        "Cross-modal: n={} causal={} — {}",
                        v.get("n_correlations").and_then(|x| x.as_u64()).unwrap_or(0),
                        v.get("asserts_causality_any")
                            .and_then(|x| x.as_bool())
                            .unwrap_or(true),
                        v.get("note").and_then(|x| x.as_str()).unwrap_or("")
                    )),
                    Err(e) => status.set(format!("X demo failed: {e}")),
                }
                busy.set(false);
            });
        }
    };

    rsx! {
        div {
            style: "flex:1; overflow-y:auto; padding:1.5rem 2rem; max-width:720px; margin:0 auto; color:var(--qualia-text);",
            h1 { style: "margin:0 0 0.35rem; font-size:1.55rem; font-weight:700;", "Listen" }
            p {
                style: "margin:0 0 1.25rem; color:var(--qualia-text-muted); line-height:1.5; font-size:0.92rem;",
                "Local ears path — synthetic tone, log-mel, reference VAD/events, epistemic quins. Not production speech recognition. No cloud."
            }
            div { style: "display:flex; flex-wrap:wrap; gap:0.65rem; margin-bottom:1rem;",
                button {
                    disabled: busy(),
                    onclick: run,
                    style: "padding:0.45rem 0.9rem; border-radius:8px; border:1px solid var(--qualia-border); background:rgba(0,180,220,0.15); font-weight:600; cursor:pointer;",
                    if busy() { "Running…" } else { "Run Ears demo" }
                }
                button {
                    disabled: busy(),
                    onclick: run_x,
                    style: "padding:0.45rem 0.9rem; border-radius:8px; border:1px solid var(--qualia-border); cursor:pointer;",
                    "Cross-modal (X) demo"
                }
            }
            p { style: "font-size:0.88rem; color:var(--qualia-text-muted); line-height:1.45;", "{status}" }
            if !detail().is_empty() {
                pre {
                    style: "font-size:0.75rem; overflow:auto; padding:0.75rem; border-radius:8px; border:1px solid var(--qualia-border); background:rgba(0,0,0,0.25);",
                    "{detail}"
                }
            }
        }
    }
}
