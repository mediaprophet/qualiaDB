//! Wellbeing self-assessment (T2.2) — PHQ-9 / GAD-7.
//!
//! A self-monitoring aid, **not a diagnosis**. The instrument's own disclaimer is shown prominently,
//! and any safety flag (e.g. a self-harm response on PHQ-9) is surfaced regardless of the total.

use super::host_client::{
    fetch_assessment_instruments, fetch_assessments, record_assessment, AssessmentInstrumentDto,
    AssessmentResultDto,
};
use dioxus::prelude::*;

#[derive(Clone, Debug, Default)]
struct AssessUi {
    status: String,
    instruments: Vec<AssessmentInstrumentDto>,
    selected: String,
    responses: Vec<u8>,
    last: Option<AssessmentResultDto>,
    history: Vec<AssessmentResultDto>,
    loaded: bool,
}

fn selected_instrument(ui: &AssessUi) -> Option<AssessmentInstrumentDto> {
    ui.instruments.iter().find(|i| i.id == ui.selected).cloned()
}

async fn load(mut ui: Signal<AssessUi>) {
    match fetch_assessment_instruments().await {
        Ok(list) => {
            if ui.read().selected.is_empty() {
                if let Some(first) = list.first() {
                    ui.write().selected = first.id.clone();
                    ui.write().responses = vec![0u8; first.items.len()];
                }
            }
            ui.write().instruments = list;
        }
        Err(e) => ui.write().status = format!("Couldn't load instruments: {e}"),
    }
    if let Ok(h) = fetch_assessments().await {
        ui.write().history = h;
    }
    ui.write().loaded = true;
}

#[component]
pub fn WellfairAssessmentPanel() -> Element {
    let mut ui = use_signal(AssessUi::default);
    let mut init_done = use_signal(|| false);
    use_effect(move || {
        if init_done() { return; }
        init_done.set(true);
        spawn(load(ui));
    });

    let inst = selected_instrument(&ui());

    rsx! {
        section {
            aria_label: "Wellbeing self-assessment",
            style: "padding:0.85rem;border:1px solid var(--qualia-border,#ddd);border-radius:10px;background:var(--qualia-surface,#fafafa);margin-top:0.85rem;",
            h2 { style: "margin:0 0 0.35rem;font-size:1rem;", "Wellbeing check-in" }
            p {
                style: "margin:0 0 0.6rem;font-size:0.76rem;color:var(--qualia-text-muted,#666);",
                "A short, private self-check you can repeat over time to notice patterns. It is a self-monitoring aid — not a diagnosis."
            }
            if !ui().status.is_empty() {
                p { style: "margin:0 0 0.5rem;font-size:0.76rem;", "{ui().status}" }
            }

            // Instrument picker.
            label {
                style: "display:flex;flex-direction:column;gap:0.2rem;font-size:0.75rem;margin-bottom:0.5rem;max-width:24rem;",
                "Check-in"
                select {
                    value: "{ui().selected}",
                    onchange: move |e| {
                        let id = e.value();
                        let len = ui().instruments.iter().find(|i| i.id == id).map(|i| i.items.len()).unwrap_or(0);
                        let mut w = ui.write();
                        w.selected = id;
                        w.responses = vec![0u8; len];
                        w.last = None;
                    },
                    style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);",
                    for i in ui().instruments.clone() {
                        option { key: "{i.id}", value: "{i.id}", "{i.name}" }
                    }
                }
            }

            if let Some(inst) = inst.clone() {
                // Prominent, honest disclaimer.
                p {
                    role: "note",
                    style: "margin:0 0 0.7rem;padding:0.5rem 0.6rem;font-size:0.73rem;border:1px solid #e9c46a55;background:#e9c46a14;border-radius:8px;color:#7a5c12;",
                    "{inst.disclaimer}"
                }
                p { style: "margin:0 0 0.5rem;font-size:0.8rem;font-weight:600;", "{inst.prompt}" }

                // One row per item.
                ol {
                    style: "margin:0 0 0.6rem;padding-left:1.1rem;display:flex;flex-direction:column;gap:0.5rem;",
                    for (i, item) in inst.items.iter().cloned().enumerate() {
                        li {
                            key: "{i}",
                            style: "font-size:0.78rem;",
                            div { style: "margin-bottom:0.2rem;", "{item}" }
                            select {
                                value: "{ui().responses.get(i).copied().unwrap_or(0)}",
                                onchange: move |e| {
                                    if let Ok(v) = e.value().parse::<u8>() {
                                        if let Some(slot) = ui.write().responses.get_mut(i) {
                                            *slot = v;
                                        }
                                    }
                                },
                                style: "padding:0.3rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.76rem;",
                                for (val, label) in inst.options.clone() {
                                    option { key: "{val}", value: "{val}", "{label}" }
                                }
                            }
                        }
                    }
                }

                button {
                    style: "padding:0.4rem 0.8rem;border-radius:8px;border:none;background:var(--qualia-accent,#2a6f97);color:#fff;font-size:0.8rem;cursor:pointer;",
                    onclick: move |_| {
                        let s = ui();
                        let id = s.selected.clone();
                        let csv = s.responses.iter().map(|r| r.to_string()).collect::<Vec<_>>().join(",");
                        if id.is_empty() { return; }
                        spawn(async move {
                            ui.write().status = "Scoring…".into();
                            match record_assessment(&id, &csv).await {
                                Ok(res) => {
                                    ui.write().last = Some(res);
                                    ui.write().status = "Recorded — kept privately in your space.".into();
                                    if let Ok(h) = fetch_assessments().await {
                                        ui.write().history = h;
                                    }
                                }
                                Err(e) => ui.write().status = format!("Failed: {e}"),
                            }
                        });
                    },
                    "Score this check-in"
                }

                // Latest result.
                if let Some(res) = ui().last.clone() {
                    div {
                        style: "margin-top:0.7rem;padding:0.6rem;border:1px solid var(--qualia-border,#ddd);border-radius:8px;background:var(--qualia-surface,#fff);",
                        p {
                            style: "margin:0 0 0.3rem;font-size:0.82rem;font-weight:600;",
                            "Score: {res.total} / {inst.max_score} — {res.band_label}"
                        }
                        p { style: "margin:0 0 0.3rem;font-size:0.76rem;color:var(--qualia-text-muted,#555);", "{res.interpretation}" }
                        for flag in res.flags.clone() {
                            p {
                                role: "alert",
                                style: "margin:0.3rem 0 0;padding:0.5rem;font-size:0.76rem;border:1px solid #e6394655;background:#e6394611;border-radius:8px;color:#a52834;",
                                "{flag}"
                            }
                        }
                    }
                }
            }

            // History.
            if !ui().history.is_empty() {
                h3 { style: "margin:0.85rem 0 0.35rem;font-size:0.88rem;", "Past check-ins ({ui().history.len()})" }
                ul {
                    style: "margin:0;padding:0;list-style:none;display:flex;flex-direction:column;gap:0.3rem;",
                    for r in ui().history.clone() {
                        li {
                            key: "{r.id}",
                            style: "padding:0.35rem 0.5rem;border:1px solid var(--qualia-border,#eee);border-radius:6px;font-size:0.75rem;",
                            span { style: "font-weight:600;", "{r.instrument_id}" }
                            span { " — {r.total} ({r.band_label})" }
                            if !r.flags.is_empty() {
                                span { style: "color:#a52834;", " ⚑" }
                            }
                        }
                    }
                }
            }
        }
    }
}
