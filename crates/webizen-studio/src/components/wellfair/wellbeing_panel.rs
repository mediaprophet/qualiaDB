//! Mental wellbeing — observations and therapy notes (no licensed instruments).

use super::host_client::{add_therapy_note, add_wellbeing_observation, fetch_health_records};
use super::host_dto::HealthRecordDto;
use dioxus::prelude::*;

#[component]
pub fn WellfairWellbeingPanel() -> Element {
    let mut mood = use_signal(|| "neutral".to_string());
    let mut intensity = use_signal(String::new);
    let mut therapy_notes = use_signal(String::new);
    let mut records = use_signal(Vec::<HealthRecordDto>::new);
    let mut status =
        use_signal(|| "Self-reported wellbeing — not a diagnostic instrument.".to_string());

    let reload = move || {
        spawn(async move {
            if let Ok(list) = fetch_health_records(48).await {
                let wb: Vec<_> = list
                    .into_iter()
                    .filter(|r| matches!(r.kind.as_str(), "wellbeing_observation" | "therapy_note"))
                    .collect();
                records.set(wb);
            }
        });
    };

    let mut loaded = use_signal(|| false);

    use_effect(move || {
        if loaded() {
            return;
        }
        loaded.set(true);
        reload();
    });

    rsx! {
        section {
            aria_label: "WellFair mental wellbeing",
            style: "padding:0.85rem;border:1px solid var(--qualia-border,#ddd);border-radius:10px;background:var(--qualia-surface,#fafafa);margin-top:0.75rem;",
            super::shared::DomainChrome { domain: "Care", chip: "Body · wellbeing · not diagnosis", show_memory: true }
            h2 { style: "margin:0 0 0.35rem;font-size:1rem;", "Mental wellbeing" }
            p {
                role: "note",
                style: "margin:0 0 0.75rem;font-size:0.72rem;color:#8a6d1d;background:#e9c46a18;padding:0.4rem;border-radius:6px;",
                "Screening only — not diagnosis. Licensed questionnaires (PHQ-9, DASS-21, …) require separate review."
            }
            p { style: "margin:0 0 0.5rem;font-size:0.76rem;color:var(--qualia-text-muted,#666);", "{status()}" }
            div {
                style: "display:grid;grid-template-columns:repeat(auto-fit,minmax(140px,1fr));gap:0.5rem;margin-bottom:0.5rem;",
                label {
                    style: "display:flex;flex-direction:column;gap:0.2rem;font-size:0.75rem;",
                    "Mood"
                    input {
                        r#type: "text",
                        value: "{mood()}",
                        oninput: move |e| mood.set(e.value()),
                        style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);",
                    }
                }
                label {
                    style: "display:flex;flex-direction:column;gap:0.2rem;font-size:0.75rem;",
                    "Intensity (1–10)"
                    input {
                        r#type: "number",
                        value: "{intensity()}",
                        oninput: move |e| intensity.set(e.value()),
                        style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);",
                    }
                }
            }
            button {
                style: "margin-bottom:0.75rem;padding:0.4rem 0.75rem;border-radius:8px;border:none;background:var(--qualia-accent,#2a6f97);color:#fff;font-size:0.8rem;cursor:pointer;",
                onclick: move |_| {
                    let m = mood().trim().to_string();
                    let i = intensity().trim().parse::<u8>().ok();
                    spawn(async move {
                        match add_wellbeing_observation(&m, i).await {
                            Ok(_) => { status.set("Observation saved.".into()); reload(); }
                            Err(e) => status.set(format!("Failed: {e}")),
                        }
                    });
                },
                "Save observation"
            }
            label {
                style: "display:flex;flex-direction:column;gap:0.2rem;font-size:0.75rem;",
                "Therapy session note (Classified — Sanctuary-protected)"
                textarea {
                    rows: "3",
                    value: "{therapy_notes()}",
                    oninput: move |e| therapy_notes.set(e.value()),
                    style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;",
                }
            }
            button {
                style: "margin:0.5rem 0 0.75rem;padding:0.4rem 0.75rem;border-radius:8px;border:1px solid #457b9d;background:#457b9d18;color:#457b9d;font-size:0.8rem;cursor:pointer;",
                onclick: move |_| {
                    let notes = therapy_notes().trim().to_string();
                    if notes.is_empty() { return; }
                    spawn(async move {
                        match add_therapy_note(&notes, None).await {
                            Ok(_) => { status.set("Therapy note saved (Sanctuary-protected).".into()); therapy_notes.set(String::new()); reload(); }
                            Err(e) => status.set(format!("Failed: {e}")),
                        }
                    });
                },
                "Save therapy note"
            }
            if !records.read().is_empty() {
                ul {
                    style: "margin:0;padding:0;list-style:none;font-size:0.74rem;",
                    for r in records.read().clone() {
                        li {
                            key: "{r.id}",
                            style: "padding:0.35rem 0;border-bottom:1px solid var(--qualia-border,#eee);",
                            "{r.kind}"
                        }
                    }
                }
            }
        }
    }
}
