//! Phase 2 Tools — Samsung Health folder import (HLT-01).

use dioxus::prelude::*;

#[derive(Clone, Debug, Default, PartialEq)]
struct ImportUiState {
    folder_path: String,
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
            h2 { style: "margin:0 0 0.5rem;font-size:1rem;", "Tools — Samsung Health import" }
            p {
                style: "margin:0 0 0.75rem;font-size:0.78rem;color:var(--qualia-text-muted,#666);",
                "Select a folder containing Samsung Health CSV exports. Records compile to canonical envelopes and commit through WebizenHostApi → VaultService WAL."
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
                    let mut s = state.write();
                    s.folder_path = e.value();
                },
            }
            div {
                style: "display:flex;gap:0.5rem;margin-top:0.65rem;flex-wrap:wrap;",
                button {
                    style: "padding:0.4rem 0.75rem;border-radius:6px;border:none;background:var(--qualia-accent,#2a6f97);color:#fff;font-size:0.82rem;cursor:pointer;",
                    onclick: move |_| {
                        let path = state.read().folder_path.clone();
                        if path.trim().is_empty() {
                            state.write().status = "Enter a folder path first.".into();
                            return;
                        }
                        state.write().status = "Importing…".into();
                        spawn(async move {
                            let result = super::host_client::import_samsung_folder(&path).await;
                            let mut s = state.write();
                            match result {
                                Ok(report_json) => {
                                    s.status = "Import complete.".into();
                                    s.last_report = report_json;
                                }
                                Err(e) => {
                                    s.status = format!("Import failed: {e}");
                                    s.last_report.clear();
                                }
                            }
                        });
                    },
                    "Import folder"
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
        }
    }
}