//! Manifold canvas: a real `webizen-render` frame, authored by `Render.scene`.

use super::engine::{self, PoetRenderResult};
use dioxus::prelude::*;

#[component]
pub fn PoetGpuFrame(kind: &'static str, width: u32, height: u32) -> Element {
    let mut frame = use_signal(PoetRenderResult::default);
    let mut status = use_signal(|| "requesting Render.scene → webizen-render…".to_string());
    let mut loaded = use_signal(|| "");

    use_effect(move || {
        if loaded() == kind {
            return;
        }
        loaded.set(kind);
        spawn(async move {
            match engine::render_preview(kind.to_string(), width, height).await {
                Ok(v) => {
                    status.set(if v.ok {
                        format!(
                            "{} · {} nodes / {} edges / {} faces · {}",
                            v.kind, v.node_count, v.edge_count, v.face_count, v.honesty
                        )
                    } else {
                        v.diagnostic
                            .clone()
                            .unwrap_or_else(|| "renderer returned no frame".into())
                    });
                    frame.set(v);
                }
                Err(e) => {
                    status.set(format!("{e} — open Webizen Desktop for the live GPU host"));
                    frame.set(PoetRenderResult::default());
                }
            }
        });
    });

    let result = frame();
    rsx! {
        div {
            style: "position:relative;min-height:220px;border-radius:12px;overflow:hidden;background:#05070c;border:1px solid #1a2230;",
            if let Some(uri) = result.data_uri.clone() {
                img {
                    src: "{uri}",
                    alt: "webizen-render frame for {kind}",
                    style: "display:block;width:100%;height:100%;object-fit:cover;min-height:220px;",
                }
            } else {
                div {
                    style: "min-height:220px;display:grid;place-items:center;padding:16px;background:
                        radial-gradient(circle at 30% 20%, rgba(0,210,255,.12), transparent 42%),
                        radial-gradient(circle at 78% 70%, rgba(167,139,250,.10), transparent 40%),
                        #07090e;",
                    p { style: "margin:0;color:#94a3b8;font-size:.74rem;text-align:center;max-width:42ch;line-height:1.5;",
                        "{status}"
                    }
                }
            }
            div {
                style: "position:absolute;left:10px;bottom:8px;padding:3px 8px;border-radius:999px;background:rgba(7,9,14,.72);color:#94a3b8;font-size:.64rem;letter-spacing:.04em;",
                "canvas · {kind} · {status}"
            }
        }
    }
}
