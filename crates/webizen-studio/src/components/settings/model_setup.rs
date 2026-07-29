use super::host::invoke_json;
use super::types::AgentQaModelProbe;
use dioxus::prelude::*;

fn model_label(model: &serde_json::Value) -> String {
    for key in ["name", "filename", "file_name", "model_id", "path"] {
        if let Some(value) = model.get(key).and_then(serde_json::Value::as_str) {
            if !value.is_empty() {
                return value.to_string();
            }
        }
    }
    "Local model".to_string()
}

#[component]
pub fn ModelSetupPanel() -> Element {
    let mut models = use_signal(Vec::<serde_json::Value>::new);
    let mut selected = use_signal(String::new);
    let mut active = use_signal(String::new);
    let mut status =
        use_signal(|| "Choose an existing model or find one on this computer.".to_string());
    let mut busy = use_signal(|| false);
    let mut probe = use_signal(|| Option::<AgentQaModelProbe>::None);
    let mut show_download = use_signal(|| false);
    let mut download_url = use_signal(String::new);
    let mut download_filename = use_signal(String::new);
    let mut download_id = use_signal(String::new);

    let mut scan = move || {
        busy.set(true);
        status.set("Looking for local GGUF and P64 models…".to_string());
        spawn(async move {
            let active_result =
                invoke_json::<Option<String>>("get_active_model", serde_json::json!({})).await;
            if let Ok(Some(name)) = active_result {
                active.set(name);
            }
            match invoke_json::<Vec<serde_json::Value>>("discover_models", serde_json::json!({}))
                .await
            {
                Ok(list) => {
                    if selected().is_empty() {
                        if let Some(first) = list.first() {
                            selected.set(model_label(first));
                        }
                    }
                    status.set(match list.len() {
                        0 => "No models found in Webizen’s model locations.".to_string(),
                        1 => {
                            "One local model found. Inspect it, then activate and test.".to_string()
                        }
                        count => format!("{count} local models found."),
                    });
                    models.set(list);
                }
                Err(error) => status.set(format!("Model scan failed: {error}")),
            }
            busy.set(false);
        });
    };

    use_hook(move || scan());

    rsx! {
        section { style: "display:grid;gap:16px;",
            div { style: "display:flex;align-items:flex-start;justify-content:space-between;gap:16px;flex-wrap:wrap;",
                div {
                    h2 { style: "margin:0;font-size:1.3rem;", "Choose a local AI instrument" }
                    p { style: "margin:.35rem 0 0;color:var(--qualia-text-muted);font-size:.8rem;line-height:1.55;max-width:46rem;",
                        "Models are instruments under your control, never people. Activation maps the selected file into Webizen; the private readiness test proves that it can answer before Chat depends on it."
                    }
                }
                button {
                    r#type: "button",
                    style: "{super::SECONDARY_BUTTON}",
                    disabled: busy(),
                    onclick: move |_| scan(),
                    "Scan this computer"
                }
            }

            div { style: "display:grid;grid-template-columns:repeat(auto-fit,minmax(230px,1fr));gap:12px;",
                button {
                    r#type: "button",
                    style: "{super::CHOICE_CARD}",
                    onclick: move |_| {
                        busy.set(true);
                        status.set("Opening the file picker…".to_string());
                        spawn(async move {
                            match invoke_json::<Option<String>>("wellfair_pick_file_path", serde_json::json!({})).await {
                                Ok(Some(path)) => {
                                    let lower = path.to_ascii_lowercase();
                                    if !lower.ends_with(".gguf") && !lower.ends_with(".p64") {
                                        status.set("Choose a .gguf or .p64 model file.".to_string());
                                    } else {
                                        selected.set(path);
                                        status.set("Model selected. Activate it when ready.".to_string());
                                    }
                                }
                                Ok(None) => status.set("No file selected.".to_string()),
                                Err(error) => status.set(format!("File picker failed: {error}")),
                            }
                            busy.set(false);
                        });
                    },
                    span { style: "font-size:1.25rem;color:var(--qualia-accent);", "◇" }
                    strong { "Use a model on this computer" }
                    small { "Choose an existing GGUF or P64 file" }
                }
                button {
                    r#type: "button",
                    style: "{super::CHOICE_CARD}",
                    onclick: move |_| show_download.toggle(),
                    span { style: "font-size:1.25rem;color:#a7f3d0;", "↓" }
                    strong { "Download a suitable model" }
                    small { "Use a direct HTTPS model URL and an explicit local filename" }
                }
                div { style: "{super::CHOICE_CARD}",
                    span { style: "font-size:1.25rem;color:#c4b5fd;", "◎" }
                    strong { "Connect Ollama" }
                    small { "Select Ollama as the inference backend under technical settings" }
                }
            }

            if show_download() {
                form {
                    style: "{super::PANEL} display:grid;gap:11px;",
                    onsubmit: move |event| {
                        event.prevent_default();
                        let url = download_url().trim().to_string();
                        let filename = download_filename().trim().to_string();
                        let model_id = if download_id().trim().is_empty() {
                            filename.trim_end_matches(".gguf").trim_end_matches(".p64").to_string()
                        } else {
                            download_id().trim().to_string()
                        };
                        if !url.starts_with("https://") {
                            status.set("Use an HTTPS download address.".to_string());
                            return;
                        }
                        if filename.is_empty() || (!filename.to_ascii_lowercase().ends_with(".gguf") && !filename.to_ascii_lowercase().ends_with(".p64")) {
                            status.set("The saved filename must end in .gguf or .p64.".to_string());
                            return;
                        }
                        busy.set(true);
                        status.set(format!("Starting the bounded download for {filename}…"));
                        spawn(async move {
                            match invoke_json::<String>(
                                "download_model",
                                serde_json::json!({ "url": url, "filename": filename, "modelId": model_id }),
                            ).await {
                                Ok(download) => status.set(format!("Download started: {download}. Progress is available in Advanced Technical settings.")),
                                Err(error) => status.set(format!("Download failed: {error}")),
                            }
                            busy.set(false);
                        });
                    },
                    h3 { style: "margin:0;font-size:.92rem;", "Download a model" }
                    p { style: "margin:0;color:var(--qualia-text-muted);font-size:.7rem;line-height:1.5;", "Webizen downloads into its configured model store. Verify the publisher, licence and expected file size before starting." }
                    label { style: "display:grid;gap:5px;font-size:.7rem;font-weight:700;",
                        "HTTPS address"
                        input {
                            r#type: "url",
                            required: true,
                            value: "{download_url}",
                            placeholder: "https://…/model.gguf",
                            style: "{super::FIELD}",
                            oninput: move |event| download_url.set(event.value()),
                        }
                    }
                    div { style: "display:grid;grid-template-columns:repeat(auto-fit,minmax(210px,1fr));gap:10px;",
                        label { style: "display:grid;gap:5px;font-size:.7rem;font-weight:700;",
                            "Saved filename"
                            input {
                                required: true,
                                value: "{download_filename}",
                                placeholder: "model-name.gguf",
                                style: "{super::FIELD}",
                                oninput: move |event| download_filename.set(event.value()),
                            }
                        }
                        label { style: "display:grid;gap:5px;font-size:.7rem;font-weight:700;",
                            "Local label (optional)"
                            input {
                                value: "{download_id}",
                                placeholder: "Derived from filename",
                                style: "{super::FIELD}",
                                oninput: move |event| download_id.set(event.value()),
                            }
                        }
                    }
                    button { r#type: "submit", style: "{super::PRIMARY_BUTTON}", disabled: busy(), "Start download" }
                }
            }

            if !models().is_empty() {
                div { style: "display:grid;gap:8px;",
                    for model in models() {
                        {
                            let label = model_label(&model);
                            let is_selected = selected() == label;
                            rsx! {
                                button {
                                    r#type: "button",
                                    style: if is_selected { super::SELECTED_ROW } else { super::ROW },
                                    onclick: move |_| selected.set(label.clone()),
                                    span { style: "font-weight:750;", "{label}" }
                                    span { style: "margin-left:auto;color:var(--qualia-text-muted);font-size:.7rem;",
                                        if active() == label { "Active" } else { "Available" }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if !selected().is_empty() {
                div { style: "padding:16px;border:1px solid var(--qualia-border);border-radius:14px;background:color-mix(in srgb,var(--qualia-surface) 92%,transparent);",
                    div { style: "font-size:.68rem;color:var(--qualia-text-muted);text-transform:uppercase;letter-spacing:.08em;", "Selected model" }
                    div { style: "margin-top:5px;font-weight:780;overflow-wrap:anywhere;", "{selected}" }
                    div { style: "display:flex;gap:8px;margin-top:14px;flex-wrap:wrap;",
                        button {
                            r#type: "button",
                            style: "{super::PRIMARY_BUTTON}",
                            disabled: busy(),
                            onclick: move |_| {
                                let model = selected();
                                busy.set(true);
                                status.set(format!("Activating {model}…"));
                                spawn(async move {
                                    match invoke_json::<()>("set_active_model", serde_json::json!({ "modelName": model.clone() })).await {
                                        Ok(()) => {
                                            active.set(model);
                                            probe.set(None);
                                            status.set("Model active. Run the private test before relying on it.".to_string());
                                        }
                                        Err(error) => status.set(format!("Activation failed: {error}")),
                                    }
                                    busy.set(false);
                                });
                            },
                            "Activate"
                        }
                        button {
                            r#type: "button",
                            style: "{super::SECONDARY_BUTTON}",
                            disabled: busy() || active().is_empty(),
                            onclick: move |_| {
                                busy.set(true);
                                status.set("Running a private, reversible model readiness test…".to_string());
                                spawn(async move {
                                    match invoke_json::<AgentQaModelProbe>("agent_qa_test_active_model", serde_json::json!({})).await {
                                        Ok(result) => {
                                            status.set(if result.passed {
                                                format!("Model readiness passed in {} ms.", result.duration_ms)
                                            } else {
                                                format!("Model readiness did not pass: {}", result.block_reason.clone().unwrap_or_else(|| "empty or uncommitted output".to_string()))
                                            });
                                            probe.set(Some(result));
                                        }
                                        Err(error) => status.set(format!("Readiness test failed: {error}")),
                                    }
                                    busy.set(false);
                                });
                            },
                            "Run private test"
                        }
                    }
                }
            }

            div {
                role: "status",
                style: "padding:11px 13px;border-radius:10px;background:var(--qualia-accent-glow);color:var(--qualia-text);font-size:.75rem;line-height:1.45;",
                "{status}"
            }
            if let Some(result) = probe() {
                div { style: if result.passed { super::SUCCESS_CARD } else { super::WARNING_CARD },
                    strong { if result.passed { "Ready for Chat" } else { "Needs attention" } }
                    div { style: "margin-top:5px;font-size:.72rem;line-height:1.5;",
                        "Committed: {result.committed} · Cleanup: {result.cleanup_succeeded} · {result.duration_ms} ms"
                    }
                    if !result.output_sample.is_empty() {
                        pre { style: "margin:10px 0 0;white-space:pre-wrap;font-size:.68rem;color:inherit;", "{result.output_sample}" }
                    }
                }
            }
        }
    }
}
