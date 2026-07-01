//! Life & welfare — events and cases (LIF records-first).

use super::host_client::{add_case_task, add_life_event, add_welfare_case, fetch_health_records};
use super::host_dto::HealthRecordDto;
use dioxus::prelude::*;

fn welfare_case_uuid(record_id: &str) -> Option<String> {
    record_id.rsplit(':').next().map(str::to_string)
}

#[derive(Clone, Debug, Default)]
struct LifeUi {
    status: String,
    event_title: String,
    event_notes: String,
    case_title: String,
    case_summary: String,
    selected_case_id: String,
    task_title: String,
    records: Vec<HealthRecordDto>,
}

#[component]
pub fn WellfairLifePanel() -> Element {
    let mut ui = use_signal(LifeUi::default);

    let reload = move || {
        spawn(async move {
            if let Ok(list) = fetch_health_records(48).await {
                let life: Vec<_> = list
                    .into_iter()
                    .filter(|r| {
                        matches!(
                            r.kind.as_str(),
                            "life_event" | "welfare_case" | "case_task"
                        )
                    })
                    .collect();
                let prev_selected = ui.read().selected_case_id.clone();
                let default_case = life
                    .iter()
                    .find(|r| r.kind == "welfare_case")
                    .and_then(|r| welfare_case_uuid(&r.id));
                ui.write().records = life;
                if prev_selected.is_empty() {
                    if let Some(uuid) = default_case {
                        ui.write().selected_case_id = uuid;
                    }
                }
            }
        });
    };

    use_effect(move || {
        reload();
    });

    let welfare_cases: Vec<(String, String)> = ui()
        .records
        .iter()
        .filter(|r| r.kind == "welfare_case")
        .filter_map(|r| {
            let uuid = welfare_case_uuid(&r.id)?;
            let label = r.summary.as_deref().unwrap_or(&r.id).to_string();
            Some((uuid, label))
        })
        .collect();

    rsx! {
        section {
            aria_label: "WellFair life and welfare",
            style: "padding:0.85rem;border:1px solid var(--qualia-border,#ddd);border-radius:10px;background:var(--qualia-surface,#fafafa);margin-bottom:0.85rem;",
            h2 { style: "margin:0 0 0.5rem;font-size:1rem;", "Life events & welfare cases" }
            p {
                style: "margin:0 0 0.75rem;font-size:0.74rem;color:var(--qualia-text-muted,#666);",
                "Self-reported life context — not legal advice. Welfare cases are sanctuary-protected when locked."
            }
            p { style: "margin:0 0 0.5rem;font-size:0.76rem;", "{ui().status}" }

            h3 { style: "margin:0 0 0.35rem;font-size:0.88rem;", "Life event" }
            div {
                style: "display:grid;grid-template-columns:1fr 1fr;gap:0.5rem;margin-bottom:0.5rem;",
                input {
                    r#type: "text",
                    placeholder: "Event title",
                    value: "{ui().event_title}",
                    oninput: move |e| ui.write().event_title = e.value(),
                    style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;",
                }
                input {
                    r#type: "text",
                    placeholder: "Notes (optional)",
                    value: "{ui().event_notes}",
                    oninput: move |e| ui.write().event_notes = e.value(),
                    style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;",
                }
            }
            button {
                style: "margin-bottom:0.85rem;padding:0.4rem 0.75rem;border-radius:8px;border:none;background:var(--qualia-accent,#2a6f97);color:#fff;font-size:0.8rem;cursor:pointer;",
                onclick: move |_| {
                    let title = ui().event_title.trim().to_string();
                    if title.is_empty() {
                        ui.write().status = "Event title required.".into();
                        return;
                    }
                    let notes = ui().event_notes.trim().to_string();
                    spawn(async move {
                        ui.write().status = "Saving life event…".into();
                        match add_life_event(&title, if notes.is_empty() { None } else { Some(&notes) }).await {
                            Ok(_) => {
                                ui.write().status = "Life event saved.".into();
                                reload();
                            }
                            Err(e) => ui.write().status = format!("Failed: {e}"),
                        }
                    });
                },
                "Add life event"
            }

            h3 { style: "margin:0 0 0.35rem;font-size:0.88rem;", "Welfare case" }
            div {
                style: "display:grid;grid-template-columns:1fr 1fr;gap:0.5rem;margin-bottom:0.5rem;",
                input {
                    r#type: "text",
                    placeholder: "Case title",
                    value: "{ui().case_title}",
                    oninput: move |e| ui.write().case_title = e.value(),
                    style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;",
                }
                input {
                    r#type: "text",
                    placeholder: "Summary",
                    value: "{ui().case_summary}",
                    oninput: move |e| ui.write().case_summary = e.value(),
                    style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;",
                }
            }
            button {
                style: "padding:0.4rem 0.75rem;border-radius:8px;border:1px solid var(--qualia-border,#ccc);background:transparent;font-size:0.8rem;cursor:pointer;",
                onclick: move |_| {
                    let title = ui().case_title.trim().to_string();
                    if title.is_empty() {
                        ui.write().status = "Case title required.".into();
                        return;
                    }
                    let summary = ui().case_summary.trim().to_string();
                    spawn(async move {
                        ui.write().status = "Saving welfare case…".into();
                        match add_welfare_case(&title, if summary.is_empty() { None } else { Some(&summary) }).await {
                            Ok(_) => {
                                ui.write().status = "Welfare case saved.".into();
                                reload();
                            }
                            Err(e) => ui.write().status = format!("Failed: {e}"),
                        }
                    });
                },
                "Add welfare case"
            }

            h3 { style: "margin:0.85rem 0 0.35rem;font-size:0.88rem;", "Case task" }
            if welfare_cases.is_empty() {
                p {
                    style: "margin:0 0 0.5rem;font-size:0.74rem;color:var(--qualia-text-muted,#888);",
                    "Add a welfare case first to attach tasks."
                }
            } else {
                div {
                    style: "display:grid;grid-template-columns:1fr 1fr;gap:0.5rem;margin-bottom:0.5rem;",
                    label {
                        style: "display:flex;flex-direction:column;gap:0.2rem;font-size:0.75rem;",
                        "Linked case"
                        select {
                            value: "{ui().selected_case_id}",
                            onchange: move |e| ui.write().selected_case_id = e.value(),
                            style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;",
                            for (uuid, label) in welfare_cases.clone() {
                                option {
                                    value: "{uuid}",
                                    "{label}"
                                }
                            }
                        }
                    }
                    input {
                        r#type: "text",
                        placeholder: "Task title",
                        value: "{ui().task_title}",
                        oninput: move |e| ui.write().task_title = e.value(),
                        style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;",
                    }
                }
                button {
                    style: "padding:0.4rem 0.75rem;border-radius:8px;border:none;background:#457b9d;color:#fff;font-size:0.8rem;cursor:pointer;",
                    onclick: move |_| {
                        let mut case_id = ui().selected_case_id.trim().to_string();
                        if case_id.is_empty() {
                            if let Some((uuid, _)) = welfare_cases.first() {
                                case_id = uuid.clone();
                            }
                        }
                        let title = ui().task_title.trim().to_string();
                        if case_id.is_empty() || title.is_empty() {
                            ui.write().status = "Select a case and enter a task title.".into();
                            return;
                        }
                        spawn(async move {
                            ui.write().status = "Saving case task…".into();
                            match add_case_task(&case_id, &title).await {
                                Ok(_) => {
                                    ui.write().status = "Case task saved.".into();
                                    reload();
                                }
                                Err(e) => ui.write().status = format!("Failed: {e}"),
                            }
                        });
                    },
                    "Add case task"
                }
            }

            if !ui().records.is_empty() {
                h3 { style: "margin:0.85rem 0 0.35rem;font-size:0.88rem;", "Recent ({ui().records.len()})" }
                ul {
                    style: "margin:0;padding:0;list-style:none;display:flex;flex-direction:column;gap:0.35rem;",
                    for r in ui().records.clone() {
                        li {
                            key: "{r.id}",
                            style: "padding:0.4rem 0.5rem;border:1px solid var(--qualia-border,#eee);border-radius:6px;font-size:0.74rem;",
                            strong { "{r.kind}" }
                            span { style: "margin-left:0.35rem;color:var(--qualia-text-muted,#888);",
                                "{r.summary.as_deref().unwrap_or(\"—\")}"
                            }
                        }
                    }
                }
            }
        }
    }
}