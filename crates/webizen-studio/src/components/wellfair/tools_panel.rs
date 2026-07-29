//! WellFair Tools — companion bundle ingest (primary) + folder import (dev fallback).

use dioxus::prelude::*;

#[derive(Clone, Debug, Default, PartialEq)]
struct ImportUiState {
    bundle_json: String,
    folder_path: String,
    show_dev_fallback: bool,
    status: String,
    last_report: String,
}

#[component]
pub fn WellfairToolsPanel() -> Element {
    let mut state = use_signal(ImportUiState::default);

    rsx! {
        section {
            aria_label: "WellFair tools",
            style: "padding:0.85rem;border:1px solid var(--qualia-border,#ddd);border-radius:10px;background:var(--qualia-surface,#fafafa);",
            div { style: "display:flex;align-items:center;gap:0.4rem;flex-wrap:wrap;margin-bottom:0.35rem;",
                span {
                    style: "font-size:0.62rem;font-weight:800;letter-spacing:0.06em;text-transform:uppercase;color:#94a3b8;",
                    "Instruments"
                }
                span {
                    style: "font-size:0.62rem;padding:0.1rem 0.4rem;border-radius:999px;border:1px solid #475569;background:rgba(71,85,105,0.2);color:#64748b;font-weight:700;",
                    "Partial · not a peer"
                }
                Link {
                    to: crate::Route::LibraryRoute {},
                    style: "font-size:0.68rem;font-weight:700;color:#7c3aed;text-decoration:none;margin-left:auto;",
                    "→ Memory"
                }
            }
            h2 { style: "margin:0 0 0.5rem;font-size:1rem;", "Tools — Samsung Health sync" }
            p {
                style: "margin:0 0 0.75rem;font-size:0.78rem;color:var(--qualia-text-muted,#666);",
                "Samsung Health data lives on your phone. Use the WellFair companion PWA to export CSVs, then paste or share the bundle JSON here. Your desktop vault is the authoritative store. Instrument path only — not a social peer."
            }
            label {
                style: "display:block;font-size:0.78rem;margin-bottom:0.25rem;font-weight:600;",
                "Companion bundle JSON (from phone)"
            }
            textarea {
                value: "{state.read().bundle_json}",
                placeholder: "Paste JSON from companion Share / Copy on your phone…",
                rows: "6",
                style: "width:100%;padding:0.45rem 0.55rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;font-family:monospace;resize:vertical;",
                oninput: move |e| {
                    state.write().bundle_json = e.value();
                },
            }
            div {
                style: "display:flex;gap:0.5rem;margin-top:0.65rem;flex-wrap:wrap;",
                button {
                    style: "padding:0.4rem 0.75rem;border-radius:6px;border:none;background:var(--qualia-accent,#2a6f97);color:#fff;font-size:0.82rem;cursor:pointer;",
                    onclick: move |_| {
                        let json = state.read().bundle_json.clone();
                        if json.trim().is_empty() {
                            state.write().status = "Paste a companion bundle JSON from your phone first.".into();
                            return;
                        }
                        state.write().status = "Ingesting companion bundle…".into();
                        spawn(async move {
                            let result = super::host_client::ingest_companion_health(&json).await;
                            let mut s = state.write();
                            match result {
                                Ok(report_json) => {
                                    s.status = "Companion ingest complete.".into();
                                    s.last_report = report_json;
                                }
                                Err(e) => {
                                    s.status = format!("Ingest failed: {e}");
                                    s.last_report.clear();
                                }
                            }
                        });
                    },
                    "Ingest from phone"
                }
                button {
                    style: "padding:0.4rem 0.75rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);background:transparent;color:var(--qualia-text-muted,#555);font-size:0.82rem;cursor:pointer;",
                    onclick: move |_| {
                        let mut s = state.write();
                        s.show_dev_fallback = !s.show_dev_fallback;
                    },
                    if state.read().show_dev_fallback { "Hide dev fallback" } else { "Dev: folder import" }
                }
            }
            if state.read().show_dev_fallback {
                div {
                    style: "margin-top:0.85rem;padding-top:0.75rem;border-top:1px dashed var(--qualia-border,#ddd);",
                    p {
                        style: "margin:0 0 0.5rem;font-size:0.72rem;color:var(--qualia-text-muted,#888);",
                        "Developer/testing only — not the production path when data is on a phone."
                    }
                    label {
                        style: "display:block;font-size:0.78rem;margin-bottom:0.25rem;",
                        "Folder path"
                    }
                    input {
                        r#type: "text",
                        value: "{state.read().folder_path}",
                        placeholder: "C:\\Users\\…\\samsung_health_export",
                        style: "width:100%;padding:0.45rem 0.55rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.82rem;",
                        oninput: move |e| {
                            state.write().folder_path = e.value();
                        },
                    }
                    button {
                        style: "margin-top:0.5rem;padding:0.35rem 0.65rem;border-radius:6px;border:none;background:#666;color:#fff;font-size:0.78rem;cursor:pointer;",
                        onclick: move |_| {
                            let path = state.read().folder_path.clone();
                            if path.trim().is_empty() {
                                state.write().status = "Enter a folder path first.".into();
                                return;
                            }
                            state.write().status = "Importing folder (dev)…".into();
                            spawn(async move {
                                let result = super::host_client::import_samsung_folder(&path).await;
                                let mut s = state.write();
                                match result {
                                    Ok(report_json) => {
                                        s.status = "Folder import complete.".into();
                                        s.last_report = report_json;
                                    }
                                    Err(e) => {
                                        s.status = format!("Import failed: {e}");
                                        s.last_report.clear();
                                    }
                                }
                            });
                        },
                        "Import folder (dev)"
                    }
                }
            }
            if !state.read().status.is_empty() {
                p {
                    style: "margin:0.65rem 0 0;font-size:0.78rem;color:var(--qualia-text-muted,#555);",
                    "{state.read().status}"
                }
            }
            if !state.read().last_report.is_empty() {
                pre {
                    style: "margin:0.5rem 0 0;padding:0.5rem;font-size:0.72rem;overflow:auto;max-height:160px;background:#111;color:#e8e8e8;border-radius:6px;",
                    "{state.read().last_report}"
                }
            }

            div {
                style: "margin-top:1rem;padding-top:0.75rem;border-top:1px solid var(--qualia-border,#eee);",
                h3 { style: "margin:0 0 0.35rem;font-size:0.88rem;", "Export — standards-readable package" }
                p {
                    style: "margin:0 0 0.5rem;font-size:0.76rem;color:var(--qualia-text-muted,#666);",
                    "Produces Turtle + typed assurance manifest bound to the vault checkpoint (§8.1 step 9)."
                }
                button {
                    style: "padding:0.4rem 0.75rem;border-radius:6px;border:none;background:#2a6f97;color:#fff;font-size:0.82rem;cursor:pointer;",
                    onclick: move |_| {
                        state.write().status = "Building export package…".into();
                        spawn(async move {
                            match super::host_client::export_health_package(256).await {
                                Ok(json) => {
                                    state.write().status = "Export complete — inspect receipt in Receipts panel.".into();
                                    state.write().last_report = json;
                                }
                                Err(e) => state.write().status = format!("Export failed: {e}"),
                            }
                        });
                    },
                    "Export health records"
                }
            }
        }
    }
}
