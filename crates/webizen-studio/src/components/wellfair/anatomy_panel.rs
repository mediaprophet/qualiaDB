//! 3D Anatomy Qapp — the text surface (S4b/S6 first cut).
//!
//! A whole-person, systemic view of how a person's records, diet and lifestyle add up across body
//! systems. **Accessibility is the default:** the person lens is plain-language, one line per system,
//! with advanced detail behind a "Show detail" toggle. Everything shown is a **hypothesis to explore,
//! not a diagnosis or advice**; evidence provenance is disclosed. A clinician lens surfaces the same
//! data as structural OSCE-Prac *considerations*. (The native 3D body replaces this surface in S5.)

use super::host_client::{fetch_anatomy_view, AnatomyViewReportDto};
use dioxus::prelude::*;

#[derive(Clone, Debug, Default, PartialEq)]
struct AnatomyUi {
    lens: String,
    report: Option<AnatomyViewReportDto>,
    status: String,
    loaded: bool,
    expanded: Option<String>,
}

async fn load(mut ui: Signal<AnatomyUi>) {
    ui.write().status = "Loading your whole-body picture…".to_string();
    let lens = ui.read().lens.clone();
    match fetch_anatomy_view(&lens, 2).await {
        Ok(report) => {
            ui.write().report = Some(report);
            ui.write().status.clear();
        }
        Err(e) => ui.write().status = format!("Couldn't load the anatomy view: {e}"),
    }
    ui.write().loaded = true;
}

fn level_style(level: &str) -> &'static str {
    match level {
        "under_strain" => "background:#e63946;",
        "worth_watching" => "background:#e9a23b;",
        _ => "background:#4a9d5b;",
    }
}

fn level_word(level: &str) -> &'static str {
    match level {
        "under_strain" => "under strain",
        "worth_watching" => "worth watching",
        _ => "settled",
    }
}

#[component]
pub fn WellfairAnatomyPanel() -> Element {
    let mut ui = use_signal(|| AnatomyUi { lens: "person".to_string(), ..Default::default() });

    use_effect(move || {
        spawn(load(ui));
    });

    let state = ui();
    let is_person = state.lens != "clinician";

    rsx! {
        section {
            aria_label: "Whole-person anatomy view",
            style: "padding:0.85rem;border:1px solid var(--qualia-border,#ddd);border-radius:10px;background:var(--qualia-surface,#fafafa);margin-top:0.85rem;",
            h2 { style: "margin:0 0 0.35rem;font-size:1rem;", "Your body, overall" }
            p {
                style: "margin:0 0 0.6rem;font-size:0.78rem;color:var(--qualia-text-muted,#666);",
                "A simple picture of how the things you've logged — conditions, medicines, food and habits — seem to add up across your body. It is a general guide to explore with a clinician, not a diagnosis."
            }

            // Lens toggle.
            div {
                role: "group",
                aria_label: "View",
                style: "display:flex;gap:0.4rem;margin-bottom:0.7rem;",
                button {
                    type: "button",
                    aria_pressed: "{is_person}",
                    style: if is_person {
                        "padding:0.45rem 0.8rem;border:2px solid var(--qualia-accent,#2a6f97);border-radius:8px;background:#fff;cursor:pointer;font-size:0.85rem;"
                    } else {
                        "padding:0.45rem 0.8rem;border:1px solid var(--qualia-border,#ccc);border-radius:8px;background:#fff;cursor:pointer;font-size:0.85rem;"
                    },
                    onclick: move |_| {
                        ui.write().lens = "person".to_string();
                        spawn(load(ui));
                    },
                    "Simple view"
                }
                button {
                    type: "button",
                    aria_pressed: "{!is_person}",
                    style: if !is_person {
                        "padding:0.45rem 0.8rem;border:2px solid var(--qualia-accent,#2a6f97);border-radius:8px;background:#fff;cursor:pointer;font-size:0.85rem;"
                    } else {
                        "padding:0.45rem 0.8rem;border:1px solid var(--qualia-border,#ccc);border-radius:8px;background:#fff;cursor:pointer;font-size:0.85rem;"
                    },
                    onclick: move |_| {
                        ui.write().lens = "clinician".to_string();
                        spawn(load(ui));
                    },
                    "Clinician view"
                }
            }

            if !state.status.is_empty() {
                p { style: "margin:0 0 0.5rem;font-size:0.8rem;", "{state.status}" }
            }

            if let Some(report) = state.report.clone() {
                // Overall summary.
                p {
                    style: "margin:0 0 0.6rem;font-size:0.95rem;font-weight:600;line-height:1.4;",
                    "{report.view.summary}"
                }

                // Per-system cards.
                if report.view.systems.is_empty() {
                    p {
                        style: "margin:0 0 0.6rem;font-size:0.85rem;color:var(--qualia-text-muted,#666);",
                        "Nothing is standing out from what you've logged so far."
                    }
                } else {
                    div {
                        style: "display:flex;flex-direction:column;gap:0.5rem;margin-bottom:0.7rem;",
                        for sys in report.view.systems.clone() {
                            div {
                                key: "{sys.system_id}",
                                style: "padding:0.6rem 0.7rem;border:1px solid var(--qualia-border,#e2e2e2);border-radius:9px;background:#fff;",
                                div {
                                    style: "display:flex;align-items:center;gap:0.5rem;",
                                    span {
                                        title: "{level_word(&sys.level)}",
                                        style: "flex:0 0 auto;width:0.8rem;height:0.8rem;border-radius:50%;{level_style(&sys.level)}",
                                    }
                                    strong {
                                        style: "font-size:0.95rem;text-transform:capitalize;",
                                        if is_person { "{sys.plain_label}" } else { "{sys.system_label}" }
                                    }
                                    span {
                                        style: "margin-left:auto;font-size:0.72rem;color:var(--qualia-text-muted,#777);",
                                        "{level_word(&sys.level)}"
                                    }
                                }
                                p {
                                    style: "margin:0.35rem 0 0;font-size:0.88rem;line-height:1.4;",
                                    "{sys.headline}"
                                }
                                // Progressive disclosure — advanced detail hidden by default.
                                if !sys.detail.is_empty() {
                                    button {
                                        type: "button",
                                        style: "margin-top:0.3rem;padding:0.15rem 0.4rem;border:1px solid var(--qualia-border,#ddd);border-radius:6px;background:#f6f6f6;cursor:pointer;font-size:0.72rem;",
                                        onclick: {
                                            let id = sys.system_id.clone();
                                            move |_| {
                                                let cur = ui.read().expanded.clone();
                                                ui.write().expanded = if cur.as_deref() == Some(id.as_str()) { None } else { Some(id.clone()) };
                                            }
                                        },
                                        if state.expanded.as_deref() == Some(sys.system_id.as_str()) { "Hide detail" } else { "Show detail" }
                                    }
                                    if state.expanded.as_deref() == Some(sys.system_id.as_str()) {
                                        ul {
                                            style: "margin:0.35rem 0 0;padding-left:1.1rem;font-size:0.78rem;color:var(--qualia-text-muted,#555);",
                                            for line in sys.detail.clone() {
                                                li { style: "margin-bottom:0.15rem;", "{line}" }
                                            }
                                            li {
                                                style: "margin-bottom:0.15rem;list-style:none;color:#888;",
                                                "Evidence: {sys.dominant_evidence}."
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Hard boundary — always shown, prominent.
                p {
                    role: "note",
                    style: "margin:0 0 0.5rem;padding:0.5rem 0.65rem;background:#2a6f9711;border:1px solid #2a6f9733;border-radius:8px;font-size:0.8rem;line-height:1.4;",
                    "{report.view.boundary}"
                }
                p {
                    style: "margin:0 0 0.4rem;font-size:0.72rem;color:var(--qualia-text-muted,#777);",
                    "{report.view.uncertainty_note}"
                }

                // What did not map + provenance disclosure.
                if !report.unmapped.is_empty() {
                    details {
                        style: "margin:0 0 0.4rem;font-size:0.76rem;color:var(--qualia-text-muted,#666);",
                        summary { "{report.unmapped.len()} logged item(s) we don't have a mapping for yet" }
                        ul {
                            style: "margin:0.3rem 0 0;padding-left:1.1rem;",
                            for u in report.unmapped.clone() {
                                li { key: "{u.kind}:{u.label}", "{u.label} ({u.kind})" }
                            }
                        }
                    }
                }
                p {
                    style: "margin:0;font-size:0.7rem;color:var(--qualia-text-muted,#888);line-height:1.4;",
                    "{report.mapped_count} of {report.total_records} items mapped. {report.disclosure}"
                }
            }
        }
    }
}
