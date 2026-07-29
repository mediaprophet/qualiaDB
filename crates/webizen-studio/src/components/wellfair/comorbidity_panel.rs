//! Ontology-backed comorbidity panel for the Anatomy experience.

use super::host_client::{evaluate_comorbidity, ComorbidityReportDto};
use crate::Route;
use dioxus::prelude::*;

fn status_label(status: u8) -> &'static str {
    match status {
        0 => "Active",
        1 => "Isolated contradiction",
        2 => "Defeated",
        _ => "Unknown",
    }
}

#[component]
pub fn WellfairComorbidityPanel() -> Element {
    let mut organ = use_signal(|| "Whole body".to_string());
    let mut report = use_signal(|| None::<ComorbidityReportDto>);
    let mut status = use_signal(String::new);
    let mut busy = use_signal(|| false);

    let run = move |_| {
        let target = if organ() == "Whole body" {
            String::new()
        } else {
            organ()
        };
        busy.set(true);
        status.set("Reading imported health graph and evaluating interactions…".to_string());
        spawn(async move {
            match evaluate_comorbidity(&target).await {
                Ok(value) => {
                    let count = value.verdicts.len();
                    report.set(Some(value));
                    status.set(format!(
                        "Evaluation complete · {count} graph-supported verdict(s)."
                    ));
                }
                Err(error) => {
                    report.set(None);
                    status.set(format!("Comorbidity graph is unavailable: {error}"));
                }
            }
            busy.set(false);
        });
    };

    rsx! {
        section {
            style: "border:1px solid var(--qualia-border);border-radius:18px;background:var(--qualia-surface);padding:1.25rem;color:var(--qualia-text);",
            div { style: "display:flex;align-items:flex-start;justify-content:space-between;gap:1rem;flex-wrap:wrap;",
                div {
                    div { style: "font-size:.67rem;font-weight:800;text-transform:uppercase;letter-spacing:.1em;color:var(--qualia-accent);", "Ontology inference" }
                    h2 { style: "font-size:1.1rem;margin:.35rem 0 .3rem;", "Comorbidity and Anatomy context" }
                    p { style: "font-size:.75rem;line-height:1.5;color:var(--qualia-text-muted);max-width:46rem;margin:0;", "Evaluates conditions from your local graph against Anatomy concepts. Imported records can contribute FHIR, LOINC and ICD concepts; contradictory assertions are isolated rather than silently accepted." }
                }
                button {
                    r#type: "button",
                    disabled: busy(),
                    onclick: run,
                    style: "border:0;border-radius:10px;background:var(--qualia-accent);color:white;padding:.65rem .95rem;font:inherit;font-size:.75rem;font-weight:800;cursor:pointer;",
                    if busy() { "Evaluating…" } else { "Evaluate health graph" }
                }
            }
            div { style: "display:flex;align-items:center;gap:.4rem;flex-wrap:wrap;margin:1rem 0;",
                for label in ["Imported record", "FHIR / LOINC / ICD", "Qualia graph", "Comorbidity rules", "Anatomy"] {
                    span { style: "font-size:.65rem;padding:.3rem .5rem;border:1px solid var(--qualia-border);border-radius:999px;background:rgba(127,127,127,.05);", "{label}" }
                    if label != "Anatomy" {
                        span { style: "color:var(--qualia-text-muted);font-size:.7rem;", "→" }
                    }
                }
            }
            div { style: "display:flex;gap:.4rem;align-items:center;flex-wrap:wrap;margin-bottom:.85rem;",
                span { style: "font-size:.68rem;color:var(--qualia-text-muted);margin-right:.2rem;", "Anatomy focus" }
                for choice in ["Whole body", "Heart", "Liver", "Kidney", "Brain", "Lungs"] {
                    {
                        let selected = organ() == choice;
                        rsx! {
                            button {
                                r#type: "button",
                                onclick: move |_| organ.set(choice.to_string()),
                                style: if selected {
                                    "border:1px solid var(--qualia-accent);border-radius:999px;background:var(--qualia-accent-glow);color:var(--qualia-accent);padding:.3rem .55rem;font:inherit;font-size:.66rem;font-weight:750;cursor:pointer;"
                                } else {
                                    "border:1px solid var(--qualia-border);border-radius:999px;background:transparent;color:var(--qualia-text-muted);padding:.3rem .55rem;font:inherit;font-size:.66rem;font-weight:650;cursor:pointer;"
                                },
                                "{choice}"
                            }
                        }
                    }
                }
            }
            if !status().is_empty() {
                div { role: "status", style: "font-size:.7rem;color:var(--qualia-text-muted);margin-bottom:.8rem;", "{status}" }
            }
            if let Some(value) = report() {
                if value.verdicts.is_empty() {
                    div { style: "border:1px dashed var(--qualia-border);border-radius:12px;padding:1rem;",
                        div { style: "font-size:.76rem;font-weight:750;", "No graph-supported interactions found" }
                        p { style: "font-size:.69rem;line-height:1.5;color:var(--qualia-text-muted);margin:.35rem 0 .7rem;", "This is an empty result, not a claim that no comorbidity exists. Import structured health records or map document concepts in the Semantic Library first." }
                        Link { to: Route::LibraryRoute {}, style: "font-size:.7rem;color:var(--qualia-accent);font-weight:800;text-decoration:none;", "Open Health records in the Library →" }
                    }
                } else {
                    div { style: "display:grid;grid-template-columns:repeat(auto-fit,minmax(210px,1fr));gap:.55rem;",
                        for verdict in value.verdicts {
                            div { style: "border:1px solid var(--qualia-border);border-radius:11px;padding:.75rem;background:rgba(127,127,127,.035);",
                                div { style: "font-size:.71rem;font-weight:760;word-break:break-all;", "Condition {verdict.condition_hash}" }
                                div { style: "display:flex;justify-content:space-between;gap:.5rem;margin-top:.4rem;font-size:.65rem;color:var(--qualia-text-muted);",
                                    span { "{status_label(verdict.status)}" }
                                    span { "{verdict.compounded_risk_milli} / 1000 interaction weight" }
                                }
                            }
                        }
                    }
                }
            } else {
                div { style: "font-size:.68rem;color:var(--qualia-text-muted);padding:.7rem 0 0;border-top:1px solid var(--qualia-border);", "Run the evaluator to connect this Anatomy view to your current local health graph. This supports care reasoning; it does not diagnose." }
            }
        }
    }
}
