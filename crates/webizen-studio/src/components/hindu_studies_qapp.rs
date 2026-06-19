use dioxus::prelude::*;

#[component]
pub fn HinduStudiesQapp() -> Element {
    let mut tradition = use_signal(|| "Vedanta".to_string());
    let mut primary_text = use_signal(|| String::new());
    let mut philosophical_concept = use_signal(|| "Brahman".to_string());
    let mut historical_period = use_signal(|| "Classical".to_string());
    let mut methodological_approach = use_signal(|| "Philological".to_string());
    let mut notes = use_signal(|| String::new());

    let traditions = [
        "Vedanta",
        "Samkhya",
        "Yoga",
        "Nyaya",
        "Vaisheshika",
        "Mimamsa",
        "Shaivism",
        "Vaishnavism",
        "Shaktism",
        "Tantra",
        "Reform Hinduism",
    ];
    let concepts = [
        "Brahman", "Atman", "Maya", "Karma", "Dharma", "Moksha", "Samsara", "Ahimsa",
    ];
    let periods = [
        "Vedic",
        "Upanishadic",
        "Classical",
        "Medieval Bhakti",
        "Colonial",
        "Contemporary",
    ];
    let approaches = [
        "Philological",
        "Anthropological",
        "Phenomenological",
        "Feminist",
        "Dalit Critique",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Hindu Studies" }

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
                    placeholder: "e.g. Bhagavad Gita, Brihadaranyaka Upanishad, Ramayana",
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
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → Hindu corpus engine | dharmic sieve | philosophical anchor" }
            }
        }
    }
}
