//! Hypermedia bookmarks body — Meaning Shelf & autonomous VibeMark probes.

use crate::components::poet::engine::{self, PoetEvalResult};
use dioxus::prelude::*;

const HARVEST_BOOKMARKS: &str = r#"requires [ capability("capability.invoke") ];
effect fn list_bookmarks() {
    return capability.invoke("Bookmarks.list", {
        shelf: "research"
    });
}
"#;

#[component]
pub fn BookmarksBody() -> Element {
    let mut out = use_signal(|| String::new());
    let mut busy = use_signal(|| false);

    rsx! {
        div { style: "display:grid;gap:8px;padding:8px;",
            p { style: muted(), "Meaning Shelf: Graph-native hypermedia bookmarks, media fragment anchors (3D/video/text), and reactive VibeMark probes." }
            div { style: "display:flex;gap:8px;flex-wrap:wrap;",
                button {
                    disabled: busy(),
                    style: crate::components::poet::vibe_console::secondary(),
                    onclick: move |_| {
                        busy.set(true);
                        spawn(async move {
                            match engine::eval(HARVEST_BOOKMARKS.into(), false, Some("list_bookmarks".into())).await {
                                Ok(PoetEvalResult { ok: true, value, .. }) => out.set(value),
                                Ok(v) => out.set(v.diagnostic.unwrap_or_else(|| "rejected".into())),
                                Err(e) => out.set(e),
                            }
                            busy.set(false);
                        });
                    },
                    "Bookmarks.list"
                }
            }
            if !out().is_empty() {
                pre { style: "font-size:.72rem;white-space:pre-wrap;margin:0;font-family:var(--font-mono);color:#fbbf24;", "{out}" }
            }
        }
    }
}

fn muted() -> &'static str {
    "margin:0;color:#94a3b8;font-size:.74rem;line-height:1.45;"
}
