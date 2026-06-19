use dioxus::prelude::*;

#[component]
pub fn BuddhistStudiesQapp() -> Element {
    let mut tradition = use_signal(|| "Theravada".to_string());
    let mut philosophical_concept = use_signal(|| "Dependent Origination".to_string());
    let mut primary_text = use_signal(|| String::new());
    let mut historical_period = use_signal(|| "Classical Mahayana".to_string());
    let mut methodological_approach = use_signal(|| "Philological".to_string());
    let mut notes = use_signal(|| String::new());

    let traditions = [
        "Theravada",
        "Mahayana",
        "Vajrayana",
        "Zen",
        "Pure Land",
        "Tibetan",
        "Chan",
        "Korean Son",
        "Japanese Tendai",
        "Nichiren",
    ];
    let concepts = [
        "Sunyata",
        "Dependent Origination",
        "Four Noble Truths",
        "Eightfold Path",
        "Bodhisattva Ideal",
        "Buddha Nature",
        "Karma",
        "Nirvana",
        "Madhyamaka",
        "Yogacara",
    ];
    let periods = [
        "Early Buddhism",
        "Sectarian",
        "Classical Mahayana",
        "Esoteric",
        "Contemporary Engaged Buddhism",
    ];
    let approaches = [
        "Philological",
        "Phenomenological",
        "Comparative",
        "Anthropological",
        "Feminist",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Buddhist Studies" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Tradition" }
                    select {
                        value: "{tradition}",
                        onchange: move |e| tradition.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in traditions { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Philosophical Concept" }
                    select {
                        value: "{philosophical_concept}",
                        onchange: move |e| philosophical_concept.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in concepts { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Historical Period" }
                    select {
                        value: "{historical_period}",
                        onchange: move |e| historical_period.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in periods { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Methodological Approach" }
                    select {
                        value: "{methodological_approach}",
                        onchange: move |e| methodological_approach.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in approaches { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Primary Text" }
                input {
                    r#type: "text",
                    value: "{primary_text}",
                    oninput: move |e| primary_text.set(e.value()),
                    placeholder: "e.g. Dhammapada, Heart Sutra, Lotus Sutra",
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                }
            }

            div {
                style: "flex: 1; display: flex; flex-direction: column;",
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Notes" }
                textarea {
                    value: "{notes}",
                    oninput: move |e| notes.set(e.value()),
                    style: "flex: 1; width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; resize: none; box-sizing: border-box; min-height: 80px;"
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 12px 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent);",
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{tradition} | {philosophical_concept} | {historical_period} | {methodological_approach}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → buddhist corpus engine | dharma sieve | tradition anchor" }
            }
        }
    }
}
