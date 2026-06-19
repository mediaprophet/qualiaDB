use dioxus::prelude::*;

#[component]
pub fn CriminologyAndCriminalJusticeQapp() -> Element {
    let mut theoretical_tradition = use_signal(|| "Classical".to_string());
    let mut crime_type = use_signal(|| "Violent".to_string());
    let mut justice_model = use_signal(|| "Rehabilitative".to_string());
    let mut recidivism_rate = use_signal(|| 0.35f64);
    let mut sentence_years = use_signal(|| 5u32);
    let mut deterrence_certainty = use_signal(|| 50u32);
    let mut notes = use_signal(|| String::new());

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box; overflow-y: auto;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Criminology & Criminal Justice QApp" }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Theoretical Tradition" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| theoretical_tradition.set(e.value()),
                    option { selected: true, "Classical" }
                    option { "Positivist" }
                    option { "Strain Theory" }
                    option { "Social Bond" }
                    option { "Labelling" }
                    option { "Critical" }
                    option { "Left Realism" }
                    option { "Feminist" }
                    option { "Routine Activities" }
                    option { "Environmental Criminology" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Crime Type" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| crime_type.set(e.value()),
                    option { selected: true, "Violent" }
                    option { "Property" }
                    option { "White-Collar" }
                    option { "Organised" }
                    option { "Cybercrime" }
                    option { "State Crime" }
                    option { "Hate Crime" }
                    option { "Drug Offence" }
                    option { "Sexual Violence" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Justice Model" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| justice_model.set(e.value()),
                    option { "Retributive" }
                    option { selected: true, "Rehabilitative" }
                    option { "Restorative" }
                    option { "Incapacitative" }
                    option { "Transformative" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Recidivism Rate: {recidivism_rate():.2}" }
                input {
                    r#type: "range",
                    min: "0.0",
                    max: "1.0",
                    step: "0.01",
                    value: "{recidivism_rate()}",
                    style: "width: 100%; box-sizing: border-box; accent-color: var(--qualia-accent);",
                    oninput: move |e| recidivism_rate.set(e.value().parse().unwrap_or(0.35)),
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Sentence Years" }
                input {
                    r#type: "number",
                    min: "0",
                    max: "999",
                    value: "{sentence_years()}",
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    oninput: move |e| sentence_years.set(e.value().parse().unwrap_or(5)),
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Deterrence Certainty (0–100): {deterrence_certainty()}%" }
                input {
                    r#type: "range",
                    min: "0",
                    max: "100",
                    step: "1",
                    value: "{deterrence_certainty()}",
                    style: "width: 100%; box-sizing: border-box; accent-color: var(--qualia-accent);",
                    oninput: move |e| deterrence_certainty.set(e.value().parse().unwrap_or(50)),
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Notes" }
                textarea {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box; min-height: 60px; resize: vertical;",
                    placeholder: "Research notes...",
                    oninput: move |e| notes.set(e.value()),
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent); flex: 1;",
                h3 { style: "margin-top: 0; color: var(--qualia-accent); font-size: 0.9rem;", "QualiaDB Output" }
                div { style: "font-size: 0.8rem; color: var(--qualia-text-muted); display: flex; flex-direction: column; gap: 4px;",
                    div { "Tradition: {theoretical_tradition()}" }
                    div { "Crime: {crime_type()}" }
                    div { "Justice Model: {justice_model()}" }
                    div { style: "color: if recidivism_rate() > 0.5 { \"var(--qualia-accent)\" } else { \"var(--qualia-accent)\" };", "Recidivism: {recidivism_rate():.2}" }
                    div { "Deterrence Certainty: {deterrence_certainty()}%" }
                }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 8px;", "QualiaDB → deontic logic | graph theory | statistical engine" }
            }
        }
    }
}
