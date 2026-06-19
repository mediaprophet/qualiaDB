use dioxus::prelude::*;

#[component]
pub fn PhenomenologyQapp() -> Element {
    let mut tradition = use_signal(|| "Husserlian Transcendental".to_string());
    let mut method = use_signal(|| "Epoché".to_string());
    let mut experiential_domain = use_signal(|| "Embodiment".to_string());
    let mut phenomenal_quality = use_signal(|| String::new());
    let mut notes = use_signal(|| String::new());

    let traditions = [
        "Husserlian Transcendental",
        "Heideggerian Hermeneutic",
        "Merleau-Ponty Embodied",
        "Sartrean Existential",
        "Levinas Ethical",
        "Ricoeur Narrative",
        "Feminist Phenomenology",
        "New Phenomenology",
    ];
    let methods = [
        "Epoché",
        "Eidetic Reduction",
        "Hermeneutic Circle",
        "Thick Description",
        "Imaginative Variation",
    ];
    let domains = [
        "Perception",
        "Temporality",
        "Embodiment",
        "Intersubjectivity",
        "Mood & Attunement",
        "Language",
        "Death",
        "Love",
        "Boredom",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Phenomenology" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Phenomenological Tradition" }
                    select {
                        value: "{tradition}",
                        onchange: move |e| tradition.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in traditions { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Method" }
                    select {
                        value: "{method}",
                        onchange: move |e| method.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in methods { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Experiential Domain" }
                    select {
                        value: "{experiential_domain}",
                        onchange: move |e| experiential_domain.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in domains { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                style: "flex: 1; display: flex; flex-direction: column;",
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Phenomenal Quality Description" }
                textarea {
                    value: "{phenomenal_quality}",
                    oninput: move |e| phenomenal_quality.set(e.value()),
                    placeholder: "Describe the qualia or phenomenal character under investigation...",
                    style: "flex: 1; width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; resize: none; box-sizing: border-box; min-height: 60px;"
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{tradition} | {method} | {experiential_domain}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → phenomenological engine | intentionality sieve | lived-experience anchor" }
            }
        }
    }
}
