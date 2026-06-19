use dioxus::prelude::*;

#[component]
pub fn GriefStudiesQapp() -> Element {
    let mut grief_type = use_signal(|| "Bereavement".to_string());
    let mut theoretical_model = use_signal(|| "Kübler-Ross Stages".to_string());
    let mut cultural_context = use_signal(|| "Western".to_string());
    let mut expression_mode = use_signal(|| "Ritual".to_string());
    let mut intensity = use_signal(|| 50u32);
    let mut duration_weeks = use_signal(|| 52u32);
    let mut notes = use_signal(|| String::new());

    let grief_types = [
        "Bereavement",
        "Anticipatory",
        "Complicated",
        "Collective",
        "Ecological",
        "Disenfranchised",
        "Traumatic",
    ];
    let theoretical_models = [
        "Kübler-Ross Stages",
        "Continuing Bonds",
        "Dual Process",
        "Meaning Reconstruction",
        "Narrative",
    ];
    let cultural_contexts = [
        "Western",
        "Non-Western",
        "Religious",
        "Secular",
        "Indigenous",
    ];
    let expression_modes = ["Ritual", "Art", "Community", "Private", "Spiritual"];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Grief Studies" }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Grief Type" }
                select {
                    value: "{grief_type}", onchange: move |e| grief_type.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in grief_types { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Theoretical Model" }
                select {
                    value: "{theoretical_model}", onchange: move |e| theoretical_model.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in theoretical_models { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Cultural Context" }
                select {
                    value: "{cultural_context}", onchange: move |e| cultural_context.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in cultural_contexts { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Expression Mode" }
                select {
                    value: "{expression_mode}", onchange: move |e| expression_mode.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in expression_modes { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Intensity: {intensity}" }
                input { r#type: "range", min: "0", max: "100", value: "{intensity}",
                    oninput: move |e| intensity.set(e.value().parse().unwrap_or(50)),
                    style: "width: 100%; margin-top: 4px;" }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Duration (weeks): {duration_weeks}" }
                input { r#type: "range", min: "0", max: "520", value: "{duration_weeks}",
                    oninput: move |e| duration_weeks.set(e.value().parse().unwrap_or(52)),
                    style: "width: 100%; margin-top: 4px;" }
            }
            div {
                style: "flex: 1; display: flex; flex-direction: column;",
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Notes" }
                textarea { value: "{notes}", oninput: move |e| notes.set(e.value()),
                    style: "flex: 1; width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; resize: none; box-sizing: border-box; min-height: 60px;" }
            }
            div {
                style: "background: var(--qualia-bg); padding: 12px 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent);",
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{grief_type} | {theoretical_model} | {cultural_context} | intensity: {intensity} | {duration_weeks}wk" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → grief studies engine | discourse sieve | anchor" }
            }
        }
    }
}
