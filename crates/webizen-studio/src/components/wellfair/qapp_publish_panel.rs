//! WP2 — Package & Publish: author a qapp and generate its installable PWA bundle.
//!
//! This is the P0/WP2 authoring surface of the companion-PWA feature: describe a qapp (identity,
//! kind, least-privilege capabilities, wasm entry) and write a standards-compliant PWA scaffold
//! (manifest + service worker + loader) to a chosen folder. Serving it over a secure origin so a
//! phone can install it is the next stage (P1) — see the companion-PWA plan.

use super::host_client::{pick_directory, publish_qapp_pwa};
use dioxus::prelude::*;

const KINDS: &[(&str, &str)] = &[
    ("cooperative", "Cooperative"),
    ("health", "Health"),
    ("journal", "Journal"),
    ("directory", "Directory"),
];

const CAPS: &[(&str, &str)] = &[
    ("read_records", "Read records"),
    ("write_records", "Write records"),
    ("sync", "Sync"),
    ("blob_store", "Blob store"),
    ("notifications", "Notifications"),
    ("camera", "Camera"),
];

#[derive(Clone, Debug)]
struct PublishUi {
    status: String,
    id: String,
    name: String,
    kind: String,
    description: String,
    wasm_filename: String,
    caps: Vec<String>,
    target_dir: String,
    written: Vec<String>,
}

impl Default for PublishUi {
    fn default() -> Self {
        Self {
            status: String::new(),
            id: "coop.qualia.myqapp".into(),
            name: "My Qapp".into(),
            kind: "cooperative".into(),
            description: String::new(),
            wasm_filename: "app.wasm".into(),
            caps: vec!["read_records".into()],
            target_dir: String::new(),
            written: Vec::new(),
        }
    }
}

#[component]
pub fn WellfairQappPublishPanel() -> Element {
    let mut ui = use_signal(PublishUi::default);

    rsx! {
        section {
            aria_label: "Package and publish a qapp",
            style: "padding:0.85rem;border:1px solid var(--qualia-border,#ddd);border-radius:10px;background:var(--qualia-surface,#fafafa);margin-bottom:0.85rem;",
            h2 { style: "margin:0 0 0.5rem;font-size:1rem;", "Package & Publish a qapp" }
            p {
                style: "margin:0 0 0.75rem;font-size:0.74rem;color:var(--qualia-text-muted,#666);",
                "Describe a qapp and generate an installable PWA scaffold (manifest + service worker + loader) into a folder. Capabilities are least-privilege — the qapp only gets what you tick. Serving the bundle over a secure origin so a phone can install it is the next step."
            }
            p { style: "margin:0 0 0.5rem;font-size:0.76rem;", "{ui().status}" }

            div {
                style: "display:grid;grid-template-columns:1fr 1fr;gap:0.5rem;margin-bottom:0.5rem;",
                label {
                    style: "display:flex;flex-direction:column;gap:0.2rem;font-size:0.74rem;",
                    "App id (reverse-DNS)"
                    input {
                        r#type: "text",
                        value: "{ui().id}",
                        oninput: move |e| ui.write().id = e.value(),
                        style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;",
                    }
                }
                label {
                    style: "display:flex;flex-direction:column;gap:0.2rem;font-size:0.74rem;",
                    "Display name"
                    input {
                        r#type: "text",
                        value: "{ui().name}",
                        oninput: move |e| ui.write().name = e.value(),
                        style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;",
                    }
                }
                label {
                    style: "display:flex;flex-direction:column;gap:0.2rem;font-size:0.74rem;",
                    "Kind"
                    select {
                        value: "{ui().kind}",
                        onchange: move |e| ui.write().kind = e.value(),
                        style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;",
                        for (val, label) in KINDS.iter() {
                            option { value: "{val}", "{label}" }
                        }
                    }
                }
                label {
                    style: "display:flex;flex-direction:column;gap:0.2rem;font-size:0.74rem;",
                    "Wasm entry filename"
                    input {
                        r#type: "text",
                        value: "{ui().wasm_filename}",
                        oninput: move |e| ui.write().wasm_filename = e.value(),
                        style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;",
                    }
                }
            }

            label {
                style: "display:flex;flex-direction:column;gap:0.2rem;font-size:0.74rem;margin-bottom:0.5rem;",
                "Description"
                input {
                    r#type: "text",
                    value: "{ui().description}",
                    oninput: move |e| ui.write().description = e.value(),
                    style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;",
                }
            }

            fieldset {
                style: "border:1px solid var(--qualia-border,#eee);border-radius:8px;padding:0.5rem;margin-bottom:0.6rem;",
                legend { style: "font-size:0.72rem;color:var(--qualia-text-muted,#666);", "Capabilities (least privilege)" }
                div {
                    style: "display:flex;flex-wrap:wrap;gap:0.6rem;",
                    for (val, label) in CAPS.iter() {
                        label {
                            key: "{val}",
                            style: "display:flex;gap:0.3rem;align-items:center;font-size:0.74rem;",
                            input {
                                r#type: "checkbox",
                                checked: ui().caps.iter().any(|c| c == val),
                                onchange: move |e| {
                                    let on = e.value() == "true";
                                    let v = val.to_string();
                                    let mut u = ui.write();
                                    if on {
                                        if !u.caps.contains(&v) { u.caps.push(v); }
                                    } else {
                                        u.caps.retain(|c| c != &v);
                                    }
                                },
                            }
                            "{label}"
                        }
                    }
                }
            }

            div {
                style: "display:flex;gap:0.5rem;align-items:center;flex-wrap:wrap;margin-bottom:0.5rem;",
                button {
                    style: "padding:0.4rem 0.7rem;border-radius:8px;border:1px solid var(--qualia-border,#ccc);background:transparent;font-size:0.78rem;cursor:pointer;",
                    onclick: move |_| {
                        spawn(async move {
                            match pick_directory().await {
                                Ok(Some(dir)) => {
                                    ui.write().target_dir = dir;
                                    ui.write().status = "Output folder chosen.".into();
                                }
                                Ok(None) => {}
                                Err(e) => ui.write().status = format!("Folder picker: {e}"),
                            }
                        });
                    },
                    "Choose output folder…"
                }
                span {
                    style: "font-size:0.72rem;color:var(--qualia-text-muted,#777);word-break:break-all;",
                    if ui().target_dir.is_empty() { "No folder chosen" } else { "{ui().target_dir}" }
                }
            }

            button {
                style: "padding:0.45rem 0.8rem;border-radius:8px;border:none;background:var(--qualia-accent,#2a6f97);color:#fff;font-size:0.82rem;cursor:pointer;",
                onclick: move |_| {
                    let s = ui();
                    if s.id.trim().is_empty() || s.name.trim().is_empty() {
                        ui.write().status = "Enter an app id and a name.".into();
                        return;
                    }
                    if s.target_dir.trim().is_empty() {
                        ui.write().status = "Choose an output folder first.".into();
                        return;
                    }
                    let caps_csv = s.caps.join(",");
                    spawn(async move {
                        ui.write().status = "Generating PWA bundle…".into();
                        match publish_qapp_pwa(
                            &s.target_dir, &s.id, &s.name, &s.kind,
                            &s.description, &caps_csv, &s.wasm_filename,
                        ).await {
                            Ok(files) => {
                                let n = files.len();
                                ui.write().written = files;
                                ui.write().status = format!("Generated {n} files. Drop your wasm ({}) and an icon-512.png alongside them, then serve over a secure origin to install.", s.wasm_filename);
                            }
                            Err(e) => ui.write().status = format!("Failed: {e}"),
                        }
                    });
                },
                "Generate installable PWA"
            }

            if !ui().written.is_empty() {
                div {
                    style: "margin-top:0.6rem;border:1px solid var(--qualia-border,#eee);border-radius:8px;padding:0.5rem;background:var(--qualia-surface,#fff);",
                    h4 { style: "margin:0 0 0.35rem;font-size:0.78rem;", "Written files" }
                    ul {
                        style: "margin:0;padding-left:1.1rem;font-size:0.74rem;",
                        for f in ui().written.clone() {
                            li { key: "{f}", code { "{f}" } }
                        }
                    }
                }
            }
        }
    }
}
