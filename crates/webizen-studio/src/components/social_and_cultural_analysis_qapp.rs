use dioxus::prelude::*;

#[component]
pub fn SocialAndCulturalAnalysisQapp() -> Element {
    let mut analytical_lens = use_signal(|| "Cultural Studies".to_string());
    let mut object_of_analysis = use_signal(|| "Text".to_string());
    let mut methodological_approach = use_signal(|| "Discourse Analysis".to_string());
    let mut scale = use_signal(|| "Meso".to_string());
    let mut cultural_context = use_signal(|| String::new());
    let mut notes = use_signal(|| String::new());

    let lenses = [
        "Cultural Studies",
        "Critical Theory",
        "Poststructuralism",
        "Political Economy",
        "Affect Theory",
        "Assemblage Theory",
        "Practice Theory (Bourdieu)",
        "New Materialism",
    ];
    let objects = [
        "Text",
        "Institution",
        "Practice",
        "Identity",
        "Space",
        "Technology",
        "Body",
        "Media",
        "Event",
    ];
    let approaches = [
        "Discourse Analysis",
        "Ethnography",
        "Archive",
        "Network Analysis",
        "Genealogy (Foucault)",
        "Deconstruction",
        "Quantitative-Qualitative Mixed",
    ];
    let scales = ["Micro", "Meso", "Macro"];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Social & Cultural Analysis" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Analytical Lens" }
                    select {
                        value: "{analytical_lens}",
                        onchange: move |e| analytical_lens.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in lenses { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Object of Analysis" }
                    select {
                        value: "{object_of_analysis}",
                        onchange: move |e| object_of_analysis.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in objects { option { value: "{x}", "{x}" } }
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
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Scale" }
                    select {
                        value: "{scale}",
                        onchange: move |e| scale.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in scales { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Cultural Context" }
                input {
                    r#type: "text",
                    value: "{cultural_context}",
                    oninput: move |e| cultural_context.set(e.value()),
                    placeholder: "e.g. neoliberal USA, post-Mao China, postcolonial Nigeria",
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{analytical_lens} | {object_of_analysis} | {methodological_approach} | {scale}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → cultural analysis engine | discourse sieve | social theory anchor" }
            }
        }
    }
}
