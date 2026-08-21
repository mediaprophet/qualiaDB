//! Social graph — LWW + peer hash, presented on the renderer contract.

use crate::components::poet::engine::{self, PoetEvalResult};
use crate::components::poet::gpu_frame::PoetGpuFrame;
use dioxus::prelude::*;

const LWW: &str = r#"requires [ capability("capability.invoke") ];
effect fn merge() {
    return capability.invoke("Social.lww", {
        local: { s: 1, p: 2, o: 10, clock: 1 },
        remote: { s: 1, p: 2, o: 20, clock: 5 }
    });
}
"#;

const PEER: &str = r#"requires [ capability("capability.invoke") ];
effect fn peer() {
    return capability.invoke("Net.peer_hash", "did:q42:preview");
}
"#;

#[component]
pub fn SocialBody() -> Element {
    let mut out = use_signal(|| String::new());
    let mut busy = use_signal(|| false);
    rsx! {
        div { style: "display:grid;gap:8px;",
            p { style: muted(), "Peers are hashes, not a fake chat stream. LWW is the sync kernel; the ring is Render.scene(social)." }
            PoetGpuFrame { kind: "social", width: 720, height: 300 }
            div { style: "display:flex;gap:8px;flex-wrap:wrap;",
                button {
                    disabled: busy(),
                    style: crate::components::poet::vibe_console::secondary(),
                    onclick: move |_| run(LWW, "merge", &mut busy, &mut out),
                    "Social.lww"
                }
                button {
                    disabled: busy(),
                    style: crate::components::poet::vibe_console::secondary(),
                    onclick: move |_| run(PEER, "peer", &mut busy, &mut out),
                    "Net.peer_hash"
                }
            }
            if !out().is_empty() {
                pre { style: "font-size:.72rem;white-space:pre-wrap;margin:0;", "{out}" }
            }
        }
    }
}

fn run(src: &'static str, name: &'static str, busy: &mut Signal<bool>, out: &mut Signal<String>) {
    busy.set(true);
    let mut busy = *busy;
    let mut out = *out;
    spawn(async move {
        match engine::eval(src.into(), false, Some(name.into())).await {
            Ok(PoetEvalResult {
                ok: true, value, ..
            }) => out.set(value),
            Ok(v) => out.set(v.diagnostic.unwrap_or_else(|| "rejected".into())),
            Err(e) => out.set(e),
        }
        busy.set(false);
    });
}

fn muted() -> &'static str {
    "margin:0;color:#94a3b8;font-size:.74rem;line-height:1.45;"
}
