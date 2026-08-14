//! 3D Anatomy Qapp — the text surface (S4b/S6 first cut).
//!
//! A whole-person, systemic view of how a person's records, diet and lifestyle add up across body
//! systems. **Accessibility is the default:** the person lens is plain-language, one line per system,
//! with advanced detail behind a "Show detail" toggle. Everything shown is a **hypothesis to explore,
//! not a diagnosis or advice**; evidence provenance is disclosed. A clinician lens surfaces the same
//! data as structural OSCE-Prac *considerations*. (The native 3D body replaces this surface in S5.)

use super::host_client::{
    fetch_anatomy_view, get_physiological_state, reset_physiological_state,
    set_physiological_state, AnatomyViewReportDto,
};
use dioxus::prelude::*;

#[derive(Clone, Debug, Default, PartialEq)]
struct AnatomyUi {
    lens: String,
    report: Option<AnatomyViewReportDto>,
    status: String,
    loaded: bool,
    expanded: Option<String>,
    /// The person's declared physiological state (P6 — reproductive continuum). `None` = not yet
    /// fetched; `Some(None)` = not declared (Baseline); `Some(Some(state))` = declared.
    phys_state: Option<Option<serde_json::Value>>,
    /// Whether the state picker is open.
    state_picker_open: bool,
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

async fn load_phys_state(mut ui: Signal<AnatomyUi>) {
    match get_physiological_state().await {
        Ok(v) => {
            let declared = v.get("declared").and_then(|d| d.as_bool()).unwrap_or(false);
            let state = if declared {
                v.get("state").cloned()
            } else {
                None
            };
            ui.write().phys_state = Some(state);
        }
        Err(e) => ui.write().status = format!("Couldn't load your physiological state: {e}"),
    }
}

/// The human-readable label for a physiological state JSON value.
fn state_label(state: &serde_json::Value) -> String {
    // The PhysiologicalState enum serializes as either `"Baseline"` or
    // `{ "Reproductive": <ReproductiveState> }` where ReproductiveState is either a string variant
    // ("PreMenarche", "Postpartum", "Lactating", "Perimenopause", "PostMenopause") or
    // `{ "Cycling": <CyclePhase> }` / `{ "Pregnant": <Trimester> }`.
    if state.as_str() == Some("Baseline") || state.is_null() {
        return "Baseline (not declared)".to_string();
    }
    let repro = state.get("Reproductive");
    let Some(repro) = repro else {
        return "Unknown state".to_string();
    };
    if let Some(s) = repro.as_str() {
        return match s {
            "PreMenarche" => "Pre-menarche".to_string(),
            "Postpartum" => "Postpartum".to_string(),
            "Lactating" => "Lactating".to_string(),
            "Perimenopause" => "Perimenopause".to_string(),
            "PostMenopause" => "Post-menopause".to_string(),
            _ => s.to_string(),
        };
    }
    if let Some(cycling) = repro.get("Cycling").and_then(|c| c.as_str()) {
        return match cycling {
            "Menstrual" => "Cycling — menstrual phase".to_string(),
            "Follicular" => "Cycling — follicular phase".to_string(),
            "Ovulatory" => "Cycling — ovulatory phase".to_string(),
            "Luteal" => "Cycling — luteal phase".to_string(),
            _ => format!("Cycling — {cycling}"),
        };
    }
    if let Some(tri) = repro.get("Pregnant").and_then(|t| t.as_str()) {
        return match tri {
            "First" => "Pregnant — first trimester".to_string(),
            "Second" => "Pregnant — second trimester".to_string(),
            "Third" => "Pregnant — third trimester".to_string(),
            _ => format!("Pregnant — {tri}"),
        };
    }
    "Unknown state".to_string()
}

/// The set of declarable states as `(json_string, label)` pairs.
fn declarable_states() -> Vec<(String, String)> {
    vec![
        (r#""Baseline""#.to_string(), "Baseline".to_string()),
        (
            r#"{"Reproductive":"PreMenarche"}"#.to_string(),
            "Pre-menarche".to_string(),
        ),
        (
            r#"{"Reproductive":{"Cycling":"Menstrual"}}"#.to_string(),
            "Cycling — menstrual".to_string(),
        ),
        (
            r#"{"Reproductive":{"Cycling":"Follicular"}}"#.to_string(),
            "Cycling — follicular".to_string(),
        ),
        (
            r#"{"Reproductive":{"Cycling":"Ovulatory"}}"#.to_string(),
            "Cycling — ovulatory".to_string(),
        ),
        (
            r#"{"Reproductive":{"Cycling":"Luteal"}}"#.to_string(),
            "Cycling — luteal".to_string(),
        ),
        (
            r#"{"Reproductive":{"Pregnant":"First"}}"#.to_string(),
            "Pregnant — 1st trimester".to_string(),
        ),
        (
            r#"{"Reproductive":{"Pregnant":"Second"}}"#.to_string(),
            "Pregnant — 2nd trimester".to_string(),
        ),
        (
            r#"{"Reproductive":{"Pregnant":"Third"}}"#.to_string(),
            "Pregnant — 3rd trimester".to_string(),
        ),
        (
            r#"{"Reproductive":"Postpartum"}"#.to_string(),
            "Postpartum".to_string(),
        ),
        (
            r#"{"Reproductive":"Lactating"}"#.to_string(),
            "Lactating".to_string(),
        ),
        (
            r#"{"Reproductive":"Perimenopause"}"#.to_string(),
            "Perimenopause".to_string(),
        ),
        (
            r#"{"Reproductive":"PostMenopause"}"#.to_string(),
            "Post-menopause".to_string(),
        ),
    ]
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
    let mut ui = use_signal(|| AnatomyUi {
        lens: "person".to_string(),
        ..Default::default()
    });
    let mut init_done = use_signal(|| false);

    use_effect(move || {
        if init_done() {
            return;
        }
        init_done.set(true);
        spawn(load(ui));
        spawn(load_phys_state(ui));
    });

    let state = ui();
    let is_person = state.lens != "clinician";
    let declared_state = state.phys_state.clone().flatten();
    let state_label_str = declared_state
        .as_ref()
        .map(state_label)
        .unwrap_or_else(|| "Baseline (not declared)".to_string());

    rsx! {
        section {
            aria_label: "Whole-person anatomy view",
            style: "padding:0.85rem;border:1px solid var(--qualia-border,#ddd);border-radius:10px;background:var(--qualia-surface,#fafafa);margin-top:0.85rem;",
            super::shared::DomainChrome { domain: "Care", chip: "Body · anatomy overview", show_memory: true }
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
                        "padding:0.45rem 0.8rem;border:2px solid var(--qualia-accent,#2a6f97);border-radius:8px;background:var(--qualia-surface);color:var(--qualia-text);cursor:pointer;font-size:0.85rem;"
                    } else {
                        "padding:0.45rem 0.8rem;border:1px solid var(--qualia-border,#ccc);border-radius:8px;background:transparent;color:var(--qualia-text-muted);cursor:pointer;font-size:0.85rem;"
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
                        "padding:0.45rem 0.8rem;border:2px solid var(--qualia-accent,#2a6f97);border-radius:8px;background:var(--qualia-surface);color:var(--qualia-text);cursor:pointer;font-size:0.85rem;"
                    } else {
                        "padding:0.45rem 0.8rem;border:1px solid var(--qualia-border,#ccc);border-radius:8px;background:transparent;color:var(--qualia-text-muted);cursor:pointer;font-size:0.85rem;"
                    },
                    onclick: move |_| {
                        ui.write().lens = "clinician".to_string();
                        spawn(load(ui));
                    },
                    "Clinician view"
                }
            }

            // Physiological state selector (P6 — reproductive continuum).
            div {
                role: "group",
                aria_label: "Physiological state",
                style: "margin-bottom:0.7rem;padding:0.5rem 0.6rem;border:1px solid var(--qualia-border,#e2e2e2);border-radius:8px;background:var(--qualia-surface);color:var(--qualia-text);",
                div {
                    style: "display:flex;align-items:center;gap:0.5rem;flex-wrap:wrap;",
                    span {
                        style: "font-size:0.82rem;font-weight:600;",
                        "Where you are on the continuum:"
                    }
                    span {
                        style: "font-size:0.82rem;color:var(--qualia-text-muted,#555);",
                        "{state_label_str}"
                    }
                    button {
                        type: "button",
                        style: "margin-left:auto;padding:0.25rem 0.5rem;border:1px solid var(--qualia-border,#ccc);border-radius:6px;background:transparent;color:var(--qualia-text);cursor:pointer;font-size:0.75rem;",
                        onclick: move |_| {
                            let open = ui.read().state_picker_open;
                            ui.write().state_picker_open = !open;
                        },
                        if state.state_picker_open { "Close" } else { "Change" }
                    }
                }
                if state.state_picker_open {
                    p {
                        style: "margin:0.4rem 0 0.25rem;font-size:0.74rem;color:var(--qualia-text-muted,#666);line-height:1.4;",
                        "Your own statement of where your body is on the reproductive continuum. This is your inward knowledge (forum-internum, Sanctuary-class) — the score-card reads you at this life stage. You can change or clear it any time."
                    }
                    div {
                        style: "display:flex;flex-wrap:wrap;gap:0.3rem;margin-top:0.3rem;",
                        for (json, label) in declarable_states() {
                            button {
                                key: "{json}",
                                type: "button",
                                style: "padding:0.3rem 0.55rem;border:1px solid var(--qualia-border,#ccc);border-radius:6px;background:var(--qualia-surface);color:var(--qualia-text);cursor:pointer;font-size:0.75rem;",
                                onclick: move |_| {
                                    let json_clone = json.clone();
                                    spawn(async move {
                                        if let Err(e) = set_physiological_state(&json_clone).await {
                                            ui.write().status = format!("Couldn't set state: {e}");
                                        } else {
                                            spawn(load_phys_state(ui));
                                            spawn(load(ui));
                                        }
                                        ui.write().state_picker_open = false;
                                    });
                                },
                                "{label}"
                            }
                        }
                    }
                    button {
                        type: "button",
                        style: "margin-top:0.4rem;padding:0.25rem 0.5rem;border:1px solid var(--qualia-border,#ccc);border-radius:6px;background:#f6f6f6;cursor:pointer;font-size:0.74rem;color:var(--qualia-text-muted,#666);",
                        onclick: move |_| {
                            spawn(async move {
                                if let Err(e) = reset_physiological_state().await {
                                    ui.write().status = format!("Couldn't clear state: {e}");
                                } else {
                                    spawn(load_phys_state(ui));
                                    spawn(load(ui));
                                }
                                ui.write().state_picker_open = false;
                            });
                        },
                        "Clear (back to baseline)"
                    }
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
            super::anatomy_constitution_form::AnatomyConstitutionForm { model: "male".to_string() }
        }
    }
}
