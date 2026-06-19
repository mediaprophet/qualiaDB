use dioxus::prelude::*;

#[component]
pub fn PeaceAndConflictStudiesQapp() -> Element {
    let mut conflict_type = use_signal(|| "Civil War".to_string());
    let mut peace_type = use_signal(|| "Positive Peace".to_string());
    let mut actors = use_signal(|| "State vs Non-State".to_string());
    let mut conflict_intensity = use_signal(|| 3u32);
    let mut mediation_mechanism = use_signal(|| "UN Peacekeeping".to_string());
    let mut notes = use_signal(|| String::new());

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box; overflow-y: auto;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Peace & Conflict Studies QApp" }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Conflict Type" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| conflict_type.set(e.value()),
                    option { "Interstate War" }
                    option { selected: true, "Civil War" }
                    option { "Ethnic Conflict" }
                    option { "Terrorism" }
                    option { "Structural Violence" }
                    option { "Cultural Violence" }
                    option { "Environmental Conflict" }
                    option { "Hybrid Warfare" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Peace Type" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| peace_type.set(e.value()),
                    option { "Negative Peace" }
                    option { selected: true, "Positive Peace" }
                    option { "Liberal Peace" }
                    option { "Hybrid Peace" }
                    option { "Just Peace" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Actors" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| actors.set(e.value()),
                    option { "State vs State" }
                    option { selected: true, "State vs Non-State" }
                    option { "Non-State vs Non-State" }
                    option { "Transnational" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Conflict Intensity (0-5): {conflict_intensity()}" }
                input {
                    r#type: "range",
                    min: "0",
                    max: "5",
                    step: "1",
                    value: "{conflict_intensity()}",
                    style: "width: 100%; box-sizing: border-box; accent-color: if conflict_intensity() > 3 { \"var(--qualia-accent)\" } else { \"var(--qualia-accent)\" };",
                    oninput: move |e| conflict_intensity.set(e.value().parse().unwrap_or(3)),
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Mediation Mechanism" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| mediation_mechanism.set(e.value()),
                    option { selected: true, "UN Peacekeeping" }
                    option { "Regional Organisation" }
                    option { "Track II Diplomacy" }
                    option { "Restorative Justice" }
                    option { "Truth & Reconciliation" }
                    option { "Arbitration" }
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
                    div { "Conflict: {conflict_type()}" }
                    div { "Peace Model: {peace_type()}" }
                    div { "Mechanism: {mediation_mechanism()}" }
                    div { style: "color: if conflict_intensity() > 3 { \"var(--qualia-accent)\" } else { \"var(--qualia-accent)\" };", "Intensity: {conflict_intensity()}/5" }
                }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 8px;", "QualiaDB → deontic logic | Allen Interval | graph theory conflict network" }
            }
        }
    }
}
