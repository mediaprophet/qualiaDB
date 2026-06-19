use dioxus::prelude::*;

#[component]
pub fn PlaywritingQapp() -> Element {
    let mut dramatic_form = use_signal(|| "Full-Length".to_string());
    let mut structural_approach = use_signal(|| "Aristotelian".to_string());
    let mut dialogue_register = use_signal(|| "Naturalistic".to_string());
    let mut draft_pages = use_signal(|| 90u32);
    let mut protagonist_drive = use_signal(|| String::new());
    let mut inciting_incident = use_signal(|| String::new());
    let mut central_tension = use_signal(|| String::new());
    let mut notes = use_signal(|| String::new());

    let forms = [
        "One-Act",
        "Full-Length",
        "Ten-Minute",
        "Solo Show",
        "Musical Book",
        "Radio Play",
        "Screenplay Adaptation",
        "Verbatim",
    ];
    let approaches = [
        "Well-Made Play",
        "Brechtian Epic",
        "Absurdist",
        "Aristotelian",
        "Non-Linear",
        "Devised",
    ];
    let registers = [
        "Heightened",
        "Naturalistic",
        "Poetic",
        "Subtext-Heavy",
        "Presentational",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Playwriting" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Dramatic Form" }
                    select {
                        value: "{dramatic_form}",
                        onchange: move |e| dramatic_form.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in forms { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Structural Approach" }
                    select {
                        value: "{structural_approach}",
                        onchange: move |e| structural_approach.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in approaches { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Dialogue Register" }
                    select {
                        value: "{dialogue_register}",
                        onchange: move |e| dialogue_register.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in registers { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Protagonist Drive" }
                    input {
                        r#type: "text",
                        value: "{protagonist_drive}",
                        oninput: move |e| protagonist_drive.set(e.value()),
                        placeholder: "What does the protagonist want?",
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Inciting Incident" }
                    input {
                        r#type: "text",
                        value: "{inciting_incident}",
                        oninput: move |e| inciting_incident.set(e.value()),
                        placeholder: "What disrupts the world?",
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Central Tension" }
                    input {
                        r#type: "text",
                        value: "{central_tension}",
                        oninput: move |e| central_tension.set(e.value()),
                        placeholder: "The core dramatic question",
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Draft Pages: {draft_pages}" }
                input {
                    r#type: "range",
                    min: "0",
                    max: "150",
                    value: "{draft_pages}",
                    oninput: move |e| draft_pages.set(e.value().parse().unwrap_or(90)),
                    style: "width: 100%; margin-top: 4px;"
                }
            }

            div {
                style: "flex: 1; display: flex; flex-direction: column;",
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Notes" }
                textarea {
                    value: "{notes}",
                    oninput: move |e| notes.set(e.value()),
                    style: "flex: 1; width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; resize: none; box-sizing: border-box; min-height: 60px;"
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 12px 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent);",
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{dramatic_form} | {structural_approach} | {dialogue_register} | {draft_pages}pp" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → dramatic structure engine | dialogue analysis sieve | theatre anchor" }
            }
        }
    }
}
