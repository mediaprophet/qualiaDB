//! Clinical documents — manual reports + attachment metadata (Phase 3 / CLI).
//!
//! Records-first: the body is stored exactly as typed. There is no parsing here, and an
//! unconfirmed report is never presented as clinician-verified.

use super::host_client::{add_clinical_report, fetch_health_records};
use super::host_dto::HealthRecordDto;
use dioxus::prelude::*;

#[derive(Clone, Debug)]
struct ClinicalUi {
    status: String,
    title: String,
    report_type: String,
    author_label: String,
    body: String,
    records: Vec<HealthRecordDto>,
}

impl Default for ClinicalUi {
    fn default() -> Self {
        Self {
            status: String::new(),
            title: String::new(),
            report_type: "pathology".into(),
            author_label: String::new(),
            body: String::new(),
            records: Vec::new(),
        }
    }
}

#[component]
pub fn WellfairClinicalPanel() -> Element {
    let mut ui = use_signal(ClinicalUi::default);

    let reload = move || {
        spawn(async move {
            if let Ok(list) = fetch_health_records(64).await {
                ui.write().records = list
                    .into_iter()
                    .filter(|r| r.kind == "clinical_report")
                    .collect();
            }
        });
    };

    use_effect(move || {
        reload();
    });

    rsx! {
        section {
            aria_label: "WellFair clinical documents",
            style: "padding:0.85rem;border:1px solid var(--qualia-border,#ddd);border-radius:10px;background:var(--qualia-surface,#fafafa);margin-bottom:0.85rem;",
            h2 { style: "margin:0 0 0.5rem;font-size:1rem;", "Clinical documents" }
            p {
                style: "margin:0 0 0.75rem;font-size:0.74rem;color:var(--qualia-text-muted,#666);",
                "Manually entered — the text is stored as typed, with no automatic parsing. A report is your own account until a clinician confirms it; it is never labelled clinician-verified on its own."
            }
            p { style: "margin:0 0 0.5rem;font-size:0.76rem;", "{ui().status}" }

            div {
                style: "display:grid;grid-template-columns:2fr 1fr 1fr;gap:0.5rem;margin-bottom:0.5rem;",
                input {
                    r#type: "text",
                    placeholder: "Title (e.g. Full blood count)",
                    value: "{ui().title}",
                    oninput: move |e| ui.write().title = e.value(),
                    style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;",
                }
                label {
                    style: "display:flex;flex-direction:column;gap:0.2rem;font-size:0.75rem;",
                    "Type"
                    select {
                        value: "{ui().report_type}",
                        onchange: move |e| ui.write().report_type = e.value(),
                        style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;",
                        option { value: "pathology", "Pathology" }
                        option { value: "imaging", "Imaging" }
                        option { value: "discharge", "Discharge" }
                        option { value: "referral", "Referral" }
                        option { value: "other", "Other" }
                    }
                }
                input {
                    r#type: "text",
                    placeholder: "Author label (optional)",
                    value: "{ui().author_label}",
                    oninput: move |e| ui.write().author_label = e.value(),
                    style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;",
                }
            }
            textarea {
                placeholder: "Report body (as written)",
                value: "{ui().body}",
                oninput: move |e| ui.write().body = e.value(),
                style: "width:100%;min-height:4rem;padding:0.4rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;margin-bottom:0.5rem;box-sizing:border-box;",
            }
            button {
                style: "margin-bottom:0.85rem;padding:0.4rem 0.75rem;border-radius:8px;border:none;background:var(--qualia-accent,#2a6f97);color:#fff;font-size:0.8rem;cursor:pointer;",
                onclick: move |_| {
                    let title = ui().title.trim().to_string();
                    let body = ui().body.trim().to_string();
                    if title.is_empty() || body.is_empty() {
                        ui.write().status = "Title and body are required.".into();
                        return;
                    }
                    let report_type = ui().report_type.clone();
                    let author = ui().author_label.trim().to_string();
                    spawn(async move {
                        ui.write().status = "Saving clinical report…".into();
                        let author_ref = if author.is_empty() { None } else { Some(author.as_str()) };
                        match add_clinical_report(&title, &report_type, &body, author_ref).await {
                            Ok(_) => {
                                ui.write().status = "Clinical report saved (draft).".into();
                                ui.write().title = String::new();
                                ui.write().body = String::new();
                                ui.write().author_label = String::new();
                                reload();
                            }
                            Err(e) => ui.write().status = format!("Failed: {e}"),
                        }
                    });
                },
                "Add clinical report"
            }

            if !ui().records.is_empty() {
                h3 { style: "margin:0 0 0.35rem;font-size:0.88rem;", "Reports ({ui().records.len()})" }
                ul {
                    style: "margin:0;padding:0;list-style:none;display:flex;flex-direction:column;gap:0.35rem;",
                    for r in ui().records.clone() {
                        li {
                            key: "{r.id}",
                            style: "padding:0.4rem 0.5rem;border:1px solid var(--qualia-border,#eee);border-radius:6px;font-size:0.74rem;",
                            span { style: "color:var(--qualia-text-muted,#888);",
                                "{r.summary.as_deref().unwrap_or(\"—\")}"
                            }
                        }
                    }
                }
            }
        }
    }
}
