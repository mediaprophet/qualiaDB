use dioxus::prelude::*;

#[component]
pub fn LeadershipStudiesQapp() -> Element {
    let mut leadership_theory = use_signal(|| "Transformational".to_string());
    let mut context = use_signal(|| "Corporate".to_string());
    let mut decision_style = use_signal(|| "Consultative".to_string());
    let mut emotional_intelligence_eq = use_signal(|| 70u32);
    let mut team_size = use_signal(|| 12u32);
    let mut crisis_mode = use_signal(|| false);
    let mut notes = use_signal(|| String::new());

    let theories = [
        "Transformational",
        "Transactional",
        "Servant Leadership",
        "Authentic",
        "Situational (Hersey-Blanchard)",
        "Distributed",
        "Adaptive",
        "Complexity Leadership",
        "Followership Theory",
        "Critical Leadership Studies",
    ];
    let contexts = [
        "Corporate",
        "Public Sector",
        "Military",
        "NGO",
        "Education",
        "Healthcare",
        "Community",
        "Social Movement",
    ];
    let styles = [
        "Autocratic",
        "Consultative",
        "Democratic",
        "Delegative",
        "Consensus",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Leadership Studies" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Leadership Theory" }
                    select {
                        value: "{leadership_theory}",
                        onchange: move |e| leadership_theory.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in theories { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Context" }
                    select {
                        value: "{context}",
                        onchange: move |e| context.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in contexts { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Decision Style" }
                    select {
                        value: "{decision_style}",
                        onchange: move |e| decision_style.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in styles { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Team Size" }
                    input {
                        r#type: "number",
                        value: "{team_size}",
                        oninput: move |e| team_size.set(e.value().parse().unwrap_or(12)),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Emotional Intelligence EQ (0–100): {emotional_intelligence_eq}" }
                input {
                    r#type: "range",
                    min: "0",
                    max: "100",
                    value: "{emotional_intelligence_eq}",
                    oninput: move |e| emotional_intelligence_eq.set(e.value().parse().unwrap_or(70)),
                    style: "width: 100%; margin-top: 4px;"
                }
            }

            div {
                style: "display: flex; align-items: center; gap: 8px;",
                input {
                    r#type: "checkbox",
                    checked: "{crisis_mode}",
                    onchange: move |e| crisis_mode.set(e.checked()),
                    id: "crisis_mode_cb"
                }
                label { r#for: "crisis_mode_cb", style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Crisis Mode Active" }
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{leadership_theory} | {context} | {decision_style} | EQ={emotional_intelligence_eq} | Crisis={crisis_mode}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → leadership theory engine | organisational sieve | adaptive systems graph" }
            }
        }
    }
}
