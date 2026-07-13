//! Welfare support — assistance needs, welfare streams, government letters (Phase 3 / LIF-08..14).

use super::host_client::{
    add_assistance_need, add_government_letter, add_government_letter_attachment_from_path,
    add_welfare_stream, export_attachment, fetch_health_records,
};
use super::host_dto::HealthRecordDto;
use dioxus::prelude::*;

#[derive(Clone, Debug)]
struct WelfareUi {
    status: String,
    need_category: String,
    need_description: String,
    need_urgency: String,
    stream_program: String,
    stream_reference: String,
    stream_status: String,
    letter_sender: String,
    letter_subject: String,
    letter_action: bool,
    letter_path: String,
    export_path: String,
    records: Vec<HealthRecordDto>,
}

impl Default for WelfareUi {
    fn default() -> Self {
        Self {
            status: String::new(),
            need_category: String::new(),
            need_description: String::new(),
            need_urgency: "moderate".into(),
            stream_program: String::new(),
            stream_reference: String::new(),
            stream_status: "applied".into(),
            letter_sender: String::new(),
            letter_subject: String::new(),
            letter_action: false,
            letter_path: String::new(),
            export_path: String::new(),
            records: Vec::new(),
        }
    }
}

#[component]
pub fn WellfairWelfarePanel() -> Element {
    let mut ui = use_signal(WelfareUi::default);

    let reload = move || {
        spawn(async move {
            if let Ok(list) = fetch_health_records(96).await {
                ui.write().records = list
                    .into_iter()
                    .filter(|r| {
                        matches!(
                            r.kind.as_str(),
                            "assistance_need" | "welfare_stream" | "government_letter"
                        )
                    })
                    .collect();
            }
        });
    };

    let mut loaded = use_signal(|| false);

    use_effect(move || {
        if loaded() { return; }
        loaded.set(true);
        reload();
    });

    rsx! {
        section {
            aria_label: "WellFair welfare support",
            style: "padding:0.85rem;border:1px solid var(--qualia-border,#ddd);border-radius:10px;background:var(--qualia-surface,#fafafa);margin-bottom:0.85rem;",
            h2 { style: "margin:0 0 0.5rem;font-size:1rem;", "Welfare support" }
            p {
                style: "margin:0 0 0.75rem;font-size:0.74rem;color:var(--qualia-text-muted,#666);",
                "Assistance needs, benefit streams, and government letters — self-reported welfare paperwork."
            }
            p { style: "margin:0 0 0.5rem;font-size:0.76rem;", "{ui().status}" }

            h3 { style: "margin:0 0 0.35rem;font-size:0.88rem;", "Assistance need" }
            div {
                style: "display:grid;grid-template-columns:1fr 2fr 1fr;gap:0.5rem;margin-bottom:0.5rem;",
                input {
                    r#type: "text",
                    placeholder: "Category (housing, food…)",
                    value: "{ui().need_category}",
                    oninput: move |e| ui.write().need_category = e.value(),
                    style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;",
                }
                input {
                    r#type: "text",
                    placeholder: "Description",
                    value: "{ui().need_description}",
                    oninput: move |e| ui.write().need_description = e.value(),
                    style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;",
                }
                select {
                    value: "{ui().need_urgency}",
                    onchange: move |e| ui.write().need_urgency = e.value(),
                    style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;",
                    option { value: "low", "Low" }
                    option { value: "moderate", "Moderate" }
                    option { value: "high", "High" }
                    option { value: "critical", "Critical" }
                }
            }
            button {
                style: "margin-bottom:0.85rem;padding:0.4rem 0.75rem;border-radius:8px;border:none;background:var(--qualia-accent,#2a6f97);color:#fff;font-size:0.8rem;cursor:pointer;",
                onclick: move |_| {
                    let category = ui().need_category.trim().to_string();
                    let description = ui().need_description.trim().to_string();
                    if category.is_empty() || description.is_empty() {
                        ui.write().status = "Category and description required.".into();
                        return;
                    }
                    let urgency = ui().need_urgency.clone();
                    spawn(async move {
                        ui.write().status = "Saving assistance need…".into();
                        match add_assistance_need(&category, &description, &urgency).await {
                            Ok(_) => {
                                ui.write().status = "Assistance need saved.".into();
                                ui.write().need_category = String::new();
                                ui.write().need_description = String::new();
                                reload();
                            }
                            Err(e) => ui.write().status = format!("Failed: {e}"),
                        }
                    });
                },
                "Add assistance need"
            }

            h3 { style: "margin:0 0 0.35rem;font-size:0.88rem;", "Welfare stream" }
            div {
                style: "display:grid;grid-template-columns:1fr 1fr 1fr;gap:0.5rem;margin-bottom:0.5rem;",
                input {
                    r#type: "text",
                    placeholder: "Program name",
                    value: "{ui().stream_program}",
                    oninput: move |e| ui.write().stream_program = e.value(),
                    style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;",
                }
                input {
                    r#type: "text",
                    placeholder: "Reference (optional)",
                    value: "{ui().stream_reference}",
                    oninput: move |e| ui.write().stream_reference = e.value(),
                    style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;",
                }
                select {
                    value: "{ui().stream_status}",
                    onchange: move |e| ui.write().stream_status = e.value(),
                    style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;",
                    option { value: "applied", "Applied" }
                    option { value: "active", "Active" }
                    option { value: "suspended", "Suspended" }
                    option { value: "ceased", "Ceased" }
                    option { value: "rejected", "Rejected" }
                }
            }
            button {
                style: "margin-bottom:0.85rem;padding:0.4rem 0.75rem;border-radius:8px;border:1px solid var(--qualia-border,#ccc);background:transparent;font-size:0.8rem;cursor:pointer;",
                onclick: move |_| {
                    let program = ui().stream_program.trim().to_string();
                    if program.is_empty() {
                        ui.write().status = "Program name required.".into();
                        return;
                    }
                    let reference = ui().stream_reference.trim().to_string();
                    let status = ui().stream_status.clone();
                    spawn(async move {
                        ui.write().status = "Saving welfare stream…".into();
                        let r = if reference.is_empty() { None } else { Some(reference.as_str()) };
                        match add_welfare_stream(&program, r, &status).await {
                            Ok(_) => {
                                ui.write().status = "Welfare stream saved.".into();
                                ui.write().stream_program = String::new();
                                ui.write().stream_reference = String::new();
                                reload();
                            }
                            Err(e) => ui.write().status = format!("Failed: {e}"),
                        }
                    });
                },
                "Add welfare stream"
            }

            h3 { style: "margin:0 0 0.35rem;font-size:0.88rem;", "Government letter" }
            div {
                style: "display:grid;grid-template-columns:1fr 2fr;gap:0.5rem;margin-bottom:0.5rem;",
                input {
                    r#type: "text",
                    placeholder: "Sender (agency)",
                    value: "{ui().letter_sender}",
                    oninput: move |e| ui.write().letter_sender = e.value(),
                    style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;",
                }
                input {
                    r#type: "text",
                    placeholder: "Subject",
                    value: "{ui().letter_subject}",
                    oninput: move |e| ui.write().letter_subject = e.value(),
                    style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;",
                }
            }
            div {
                style: "display:flex;align-items:center;gap:0.75rem;flex-wrap:wrap;margin-bottom:0.5rem;",
                label {
                    style: "display:flex;align-items:center;gap:0.4rem;font-size:0.76rem;",
                    input {
                        r#type: "checkbox",
                        checked: ui().letter_action,
                        onchange: move |e| ui.write().letter_action = e.checked(),
                    }
                    "Action required"
                }
                input {
                    r#type: "text",
                    placeholder: "Attach document file path (optional)",
                    value: "{ui().letter_path}",
                    oninput: move |e| ui.write().letter_path = e.value(),
                    style: "flex:1;min-width:12rem;padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;",
                }
            }
            button {
                style: "margin-bottom:0.85rem;padding:0.4rem 0.75rem;border-radius:8px;border:1px solid var(--qualia-border,#ccc);background:transparent;font-size:0.8rem;cursor:pointer;",
                onclick: move |_| {
                    let sender = ui().letter_sender.trim().to_string();
                    let subject = ui().letter_subject.trim().to_string();
                    if sender.is_empty() || subject.is_empty() {
                        ui.write().status = "Sender and subject required.".into();
                        return;
                    }
                    let action = ui().letter_action;
                    let path = ui().letter_path.trim().to_string();
                    spawn(async move {
                        ui.write().status = "Saving government letter…".into();
                        let result = if path.is_empty() {
                            add_government_letter(&sender, &subject, action).await
                        } else {
                            add_government_letter_attachment_from_path(&sender, &subject, action, &path).await
                        };
                        match result {
                            Ok(_) => {
                                ui.write().status = "Government letter saved.".into();
                                ui.write().letter_sender = String::new();
                                ui.write().letter_subject = String::new();
                                ui.write().letter_action = false;
                                ui.write().letter_path = String::new();
                                reload();
                            }
                            Err(e) => ui.write().status = format!("Failed: {e}"),
                        }
                    });
                },
                "Add government letter"
            }

            if !ui().records.is_empty() {
                h3 { style: "margin:0 0 0.35rem;font-size:0.88rem;", "Records ({ui().records.len()})" }
                input {
                    r#type: "text",
                    placeholder: "Export destination path (for letters with an attachment)",
                    value: "{ui().export_path}",
                    oninput: move |e| ui.write().export_path = e.value(),
                    style: "width:100%;box-sizing:border-box;padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.76rem;margin-bottom:0.4rem;",
                }
                ul {
                    style: "margin:0;padding:0;list-style:none;display:flex;flex-direction:column;gap:0.35rem;",
                    for r in ui().records.clone() {
                        li {
                            key: "{r.id}",
                            style: "display:flex;justify-content:space-between;align-items:center;gap:0.5rem;padding:0.4rem 0.5rem;border:1px solid var(--qualia-border,#eee);border-radius:6px;font-size:0.74rem;",
                            span {
                                strong { "{r.kind}" }
                                span { style: "margin-left:0.35rem;color:var(--qualia-text-muted,#888);",
                                    "{r.summary.as_deref().unwrap_or(\"—\")}"
                                }
                            }
                            if r.kind == "government_letter" && r.blob_hash.is_some() {
                                button {
                                    style: "padding:0.25rem 0.55rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);background:transparent;font-size:0.72rem;cursor:pointer;white-space:nowrap;",
                                    onclick: {
                                        let id = r.id.clone();
                                        move |_| {
                                            let id = id.clone();
                                            let dest = ui().export_path.trim().to_string();
                                            if dest.is_empty() {
                                                ui.write().status = "Enter an export destination path first.".into();
                                                return;
                                            }
                                            spawn(async move {
                                                ui.write().status = "Exporting…".into();
                                                match export_attachment(&id, &dest).await {
                                                    Ok(_) => ui.write().status = format!("Exported to {dest}"),
                                                    Err(e) => ui.write().status = format!("Failed: {e}"),
                                                }
                                            });
                                        }
                                    },
                                    "Export"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
