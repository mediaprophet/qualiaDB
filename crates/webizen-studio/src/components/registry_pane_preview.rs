//! Honest, interactive previews for generic Studio panes.
//!
//! These panes are design-system primitives rather than standalone QApps. They
//! still need to render when placed on a canvas, but must not pretend that
//! sample values are live application data.

#![allow(non_snake_case)]

use dioxus::prelude::*;

const WRAP: &str = "height:100%;min-height:100px;padding:14px;border-radius:12px;background:rgba(15,23,42,.7);border:1px solid rgba(148,163,184,.18);color:#e2e8f0;overflow:auto;";
const INPUT: &str = "width:100%;box-sizing:border-box;background:#081222;color:#e2e8f0;border:1px solid #334155;border-radius:8px;padding:9px 10px;";

#[component]
pub fn RegistryPanePreview(element_tag: String) -> Element {
    let mut checked = use_signal(|| true);
    let mut range = use_signal(|| 62_i32);
    let mut selected = use_signal(|| "Local".to_string());

    rsx! {
        div { style: WRAP,
            match element_tag.as_str() {
                "sl-alert" => rsx! {
                    div { style: "border:1px solid #38bdf8;background:rgba(14,116,144,.18);color:#bae6fd;border-radius:10px;padding:12px;",
                        strong { "Webizen notice" }
                        div { style: "font-size:.74rem;margin-top:4px;", "Alerts placed in a QApp render here." }
                    }
                },
                "sl-avatar" => rsx! {
                    div { style: "display:flex;align-items:center;gap:12px;",
                        div { style: "width:52px;height:52px;border-radius:50%;display:grid;place-items:center;background:linear-gradient(135deg,#38bdf8,#8b5cf6);font-weight:800;", "W" }
                        div { strong { "Avatar" } div { style: "font-size:.7rem;color:#94a3b8;", "Bind an image, initials or identity." } }
                    }
                },
                "sl-badge" => rsx! { span { style: "display:inline-block;border-radius:999px;padding:6px 10px;background:#0f766e;color:#ccfbf1;font-size:.72rem;font-weight:750;", "Active" } },
                "sl-card" => rsx! {
                    div { style: "padding:14px;border-radius:11px;background:#111c30;border:1px solid #334155;",
                        strong { "Card title" }
                        p { style: "color:#94a3b8;font-size:.72rem;line-height:1.5;margin:7px 0 0;", "Drop other panes here or bind card content in the inspector." }
                    }
                },
                "sl-checkbox" => rsx! {
                    label { style: "display:flex;align-items:center;gap:9px;cursor:pointer;",
                        input { r#type:"checkbox", checked: checked(), onchange: move |e| checked.set(e.checked()) }
                        "Enabled"
                    }
                },
                "sl-color-picker" => rsx! { input { r#type:"color", value:"#38bdf8", style:"width:52px;height:42px;background:transparent;border:0;" } },
                "sl-details" => rsx! {
                    details { open:true,
                        summary { style:"cursor:pointer;font-weight:700;", "Details" }
                        p { style:"color:#94a3b8;font-size:.72rem;line-height:1.5;", "Expandable QApp content." }
                    }
                },
                "sl-divider" => rsx! { div { style:"height:1px;background:#475569;margin:26px 0;" } },
                "sl-input" => rsx! { input { style:INPUT, placeholder:"Text input" } },
                "sl-textarea" => rsx! { textarea { style:"{INPUT}min-height:90px;resize:vertical;", placeholder:"Long-form input" } },
                "sl-range" => rsx! {
                    div {
                        input { r#type:"range", min:"0", max:"100", value:"{range}", style:"width:100%;", oninput:move |e| if let Ok(v)=e.value().parse(){ range.set(v) } }
                        div { style:"text-align:right;color:#94a3b8;font-size:.7rem;", "{range}" }
                    }
                },
                "sl-rating" => rsx! { div { style:"color:#fbbf24;font-size:1.35rem;letter-spacing:4px;", "★★★★☆" } },
                "sl-select" => rsx! {
                    select { style:INPUT, value:"{selected}", onchange:move |e| selected.set(e.value()),
                        option { "Local" }
                        option { "Connected" }
                        option { "Disabled" }
                    }
                },
                "sl-switch" => rsx! {
                    label { style:"display:flex;align-items:center;gap:9px;cursor:pointer;",
                        input { r#type:"checkbox", role:"switch", checked:checked(), onchange:move |e| checked.set(e.checked()) }
                        if checked() { "On" } else { "Off" }
                    }
                },
                "sl-spinner" => rsx! {
                    div { style:"display:flex;align-items:center;gap:10px;",
                        div { style:"width:24px;height:24px;border-radius:50%;border:3px solid #334155;border-top-color:#38bdf8;" }
                        span { style:"font-size:.73rem;color:#94a3b8;", "Loading indicator" }
                    }
                },
                "sl-progress-bar" => rsx! {
                    div { style:"height:10px;border-radius:999px;background:#1e293b;overflow:hidden;",
                        div { style:"width:68%;height:100%;background:linear-gradient(90deg,#38bdf8,#8b5cf6);" }
                    }
                },
                "sl-qr-code" => rsx! {
                    div { style:"display:grid;place-items:center;min-height:120px;",
                        div { style:"background:white;color:#0f172a;padding:20px;font-size:1.8rem;line-height:1;letter-spacing:2px;font-family:monospace;", "▦▥▤" }
                    }
                },
                "sl-skeleton" => rsx! {
                    div { style:"display:grid;gap:9px;",
                        div { style:"height:14px;width:55%;border-radius:7px;background:#334155;" }
                        div { style:"height:11px;width:90%;border-radius:7px;background:#253247;" }
                        div { style:"height:11px;width:72%;border-radius:7px;background:#253247;" }
                    }
                },
                "sl-split-panel" => rsx! {
                    div { style:"display:grid;grid-template-columns:1fr 5px 1fr;min-height:110px;gap:8px;",
                        div { style:"border:1px dashed #475569;border-radius:8px;display:grid;place-items:center;color:#64748b;font-size:.7rem;", "Start" }
                        div { style:"background:#334155;border-radius:4px;" }
                        div { style:"border:1px dashed #475569;border-radius:8px;display:grid;place-items:center;color:#64748b;font-size:.7rem;", "End" }
                    }
                },
                "sl-tab-group" => rsx! {
                    div {
                        div { style:"display:flex;gap:6px;border-bottom:1px solid #334155;margin-bottom:12px;",
                            span { style:"padding:7px 10px;border-bottom:2px solid #38bdf8;color:#bae6fd;font-size:.72rem;", "Overview" }
                            span { style:"padding:7px 10px;color:#94a3b8;font-size:.72rem;", "Details" }
                        }
                        div { style:"font-size:.72rem;color:#94a3b8;", "Tab content area" }
                    }
                },
                "sl-dialog" => rsx! {
                    div { style:"max-width:360px;margin:auto;padding:16px;border-radius:12px;background:#111c30;border:1px solid #475569;box-shadow:0 18px 45px rgba(0,0,0,.35);",
                        strong { "Dialog preview" }
                        p { style:"font-size:.72rem;color:#94a3b8;", "Runtime actions control whether this dialog opens." }
                    }
                },
                "sl-carousel" | "sl-image-comparer" => rsx! {
                    div { style:"min-height:120px;border:1px dashed #475569;border-radius:10px;display:grid;place-items:center;color:#94a3b8;font-size:.72rem;",
                        "Bind media assets in the pane inspector"
                    }
                },
                "qualia-dynamic-form" => rsx! {
                    div { style:"display:grid;gap:10px;",
                        strong { "SHACL form" }
                        input { style:INPUT, placeholder:"Property value" }
                        textarea { style:"{INPUT}min-height:70px;", placeholder:"Description" }
                        button { style:"justify-self:start;padding:8px 12px;border:0;border-radius:8px;background:#0ea5e9;color:#06111f;font-weight:750;", "Validate" }
                        small { style:"color:#64748b;", "Preview only — bind a shape graph to generate fields." }
                    }
                },
                "qualia-sensor-data" => rsx! {
                    div {
                        strong { "Sensor stream" }
                        div { style:"height:80px;margin:12px 0;border-left:1px solid #334155;border-bottom:1px solid #334155;background:linear-gradient(155deg,transparent 48%,#38bdf8 49%,#38bdf8 51%,transparent 52%);" }
                        small { style:"color:#64748b;", "No device source bound" }
                    }
                },
                "qualia-web-module" => rsx! {
                    div { style:"height:100%;display:grid;place-items:center;text-align:center;",
                        div {
                            strong { "Web module" }
                            p { style:"color:#94a3b8;font-size:.72rem;line-height:1.5;", "Set an approved RPC or iframe source in the pane inspector." }
                        }
                    }
                },
                _ => rsx! {
                    div { style:"height:100%;display:grid;place-items:center;text-align:center;",
                        div {
                            strong { "Component not connected" }
                            p { style:"color:#94a3b8;font-size:.72rem;line-height:1.5;margin:7px 0;", "This catalog entry exists, but no render adapter is registered for it yet." }
                            code { style:"font-size:.67rem;color:#7dd3fc;", "{element_tag}" }
                        }
                    }
                },
            }
        }
    }
}
