//! Rich-text surface: customisable toolbar + document body + gazetteer.

use super::spec::{RichCommand, ToolbarSpec};
use crate::components::poet::engine::{self, PoetGazetteerResult};
use dioxus::prelude::*;

#[component]
pub fn RichTextTool(#[props(default = ToolbarSpec::OFFICE)] spec: ToolbarSpec) -> Element {
    let mut document = use_signal(|| {
        "North Spring is the reference catchment. Timothy Charles Holborn recorded 12.5 mm of rain on 2026-08-15 at the reference site.".to_string()
    });
    let mut gaz = use_signal(PoetGazetteerResult::default);
    let mut busy = use_signal(|| false);
    let mut status =
        use_signal(|| "Marks wrap the selection. Gazetteer is NLP, not Vibe.".to_string());

    rsx! {
        div { class: "poet-rich-text", style: "display:grid;gap:8px;min-height:0;flex:1;",
            div { class: "poet-toolbar", style: toolbar_bar(),
                for group in spec.groups {
                    div { style: "display:flex;gap:4px;align-items:center;",
                        span { style: "font-size:9px;letter-spacing:.06em;text-transform:uppercase;color:var(--text-muted);", "{group.title}" }
                        for cmd in group.commands {
                            button {
                                r#type: "button",
                                title: "{cmd.title()}",
                                disabled: busy() && *cmd == RichCommand::Gazetteer,
                                style: tool_btn(*cmd),
                                onclick: move |_| {
                                    match *cmd {
                                        RichCommand::Gazetteer => {
                                            busy.set(true);
                                            let doc = document();
                                            spawn(async move {
                                                match engine::gazetteer(doc).await {
                                                    Ok(value) => {
                                                        status.set(format!(
                                                            "tokens={} sentences={} sealed={}",
                                                            value.token_count, value.sentence_count, value.sealed
                                                        ));
                                                        gaz.set(value);
                                                    }
                                                    Err(e) => status.set(e),
                                                }
                                                busy.set(false);
                                            });
                                        }
                                        other => {
                                            document.set(wrap_mark(&document(), other));
                                            status.set(format!("applied {}", other.title()));
                                        }
                                    }
                                },
                                "{cmd.label()}"
                            }
                        }
                    }
                }
            }
            textarea {
                style: "width:100%;min-height:120px;flex:1;font-family:var(--font-sans);font-size:13px;line-height:1.5;background:var(--surface-base);color:var(--text-primary);border:1px solid var(--border-subtle);border-radius:var(--radius-sm);padding:8px;",
                value: "{document}",
                oninput: move |e| document.set(e.value()),
            }
            p { style: "margin:0;color:var(--text-muted);font-size:11px;", "{status}" }
            for (i, hit) in gaz().hits.iter().enumerate() {
                div { key: "{i}", style: "font-size:12px;",
                    span { style: "color:var(--accent-amber);", "{hit.kind} " }
                    "{hit.surface} → {hit.iri}"
                }
            }
        }
    }
}

fn wrap_mark(src: &str, cmd: RichCommand) -> String {
    let token = src.split_whitespace().last().unwrap_or("selection");
    let marked = match cmd {
        RichCommand::Bold => format!("**{token}**"),
        RichCommand::Italic => format!("*{token}*"),
        RichCommand::Heading => format!("## {token}"),
        RichCommand::Entity => format!("<q-entity data-id=\"concept:{token}\">{token}</q-entity>"),
        RichCommand::Gazetteer => token.to_string(),
    };
    if let Some(idx) = src.rfind(token) {
        let mut out = src.to_string();
        out.replace_range(idx..idx + token.len(), &marked);
        out
    } else {
        format!("{src} {marked}")
    }
}

fn toolbar_bar() -> &'static str {
    "display:flex;flex-wrap:wrap;gap:10px;align-items:center;padding:6px 8px;background:var(--surface-panel);border:1px solid var(--border-subtle);border-radius:var(--radius-sm);"
}

fn tool_btn(cmd: RichCommand) -> String {
    let weight = if matches!(cmd, RichCommand::Bold) {
        "800"
    } else {
        "600"
    };
    let italic = if matches!(cmd, RichCommand::Italic) {
        "italic"
    } else {
        "normal"
    };
    format!(
        "border:1px solid var(--border-medium);background:var(--surface-panel-elevated);color:var(--text-primary);border-radius:6px;padding:4px 8px;font-size:11px;font-weight:{weight};font-style:{italic};cursor:pointer;"
    )
}
