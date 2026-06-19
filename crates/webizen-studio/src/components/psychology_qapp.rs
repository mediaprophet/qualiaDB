use dioxus::prelude::*;

#[component]
pub fn PsychologyQapp() -> Element {
    let mut subfield = use_signal(|| "Cognitive Psychology".to_string());
    let mut paradigm = use_signal(|| "Information Processing".to_string());
    let mut method = use_signal(|| "Experiment (RCT)".to_string());
    let mut construct = use_signal(|| String::new());
    let mut sample_n = use_signal(|| 100u32);
    let mut effect_size = use_signal(|| 0.5f64);
    let mut p_value = use_signal(|| 0.05f64);
    let mut dsm_category = use_signal(|| "None / Healthy Population".to_string());
    let mut analysis_notes = use_signal(|| String::new());

    let subfields = [
        "Cognitive Psychology",
        "Clinical Psychology",
        "Developmental Psychology",
        "Social Psychology",
        "Personality Psychology",
        "Neuropsychology",
        "Health Psychology",
        "Forensic Psychology",
        "Industrial-Organisational",
        "Positive Psychology",
        "Evolutionary Psychology",
    ];
    let paradigms = [
        "Information Processing",
        "Behaviourism",
        "Psychoanalytic / Psychodynamic",
        "Humanistic",
        "Cognitive-Behavioural (CBT)",
        "Ecological / Gibsonian",
        "Computational / Connectionist",
        "Embodied / Enactivist",
        "Evolutionary",
        "Social Constructionist",
    ];
    let methods = [
        "Experiment (RCT)",
        "Quasi-Experiment",
        "Case Study",
        "Longitudinal Survey",
        "Cross-Sectional Survey",
        "Meta-Analysis",
        "Qualitative / Thematic Analysis",
        "fMRI Neuroimaging",
        "EEG",
        "Twin Study",
        "Naturalistic Observation",
    ];
    let dsm_cats = [
        "None / Healthy Population",
        "Anxiety Disorders",
        "Depressive Disorders",
        "Trauma & Stressor-Related",
        "OCD & Related",
        "Schizophrenia Spectrum",
        "Bipolar & Related",
        "Neurodevelopmental (ADHD, ASD)",
        "Personality Disorders",
        "Substance Use Disorders",
        "Neurocognitive Disorders",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Psychology" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Subfield" }
                    select {
                        value: "{subfield}",
                        onchange: move |e| subfield.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for x in subfields { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Theoretical Paradigm" }
                    select {
                        value: "{paradigm}",
                        onchange: move |e| paradigm.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for x in paradigms { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Research Method" }
                    select {
                        value: "{method}",
                        onchange: move |e| method.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for x in methods { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Psychological Construct / Variable" }
                input {
                    r#type: "text", placeholder: "e.g. working memory capacity, cognitive dissonance, attachment style…",
                    value: "{construct}",
                    oninput: move |e| construct.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                }
            }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Sample N" }
                    input {
                        r#type: "number", min: "1",
                        value: "{sample_n}",
                        oninput: move |e| sample_n.set(e.value().parse().unwrap_or(100)),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Effect Size (d/r/η²)" }
                    input {
                        r#type: "number", step: "0.01", min: "0.0", max: "2.0",
                        value: "{effect_size}",
                        oninput: move |e| effect_size.set(e.value().parse().unwrap_or(0.5)),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "p-value" }
                    input {
                        r#type: "number", step: "0.001", min: "0.0", max: "1.0",
                        value: "{p_value}",
                        oninput: move |e| p_value.set(e.value().parse().unwrap_or(0.05)),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "DSM-5 Category" }
                    select {
                        value: "{dsm_category}",
                        onchange: move |e| dsm_category.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for x in dsm_cats { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                style: "flex: 1; display: flex; flex-direction: column;",
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Analysis Notes" }
                textarea {
                    value: "{analysis_notes}",
                    oninput: move |e| analysis_notes.set(e.value()),
                    style: "flex: 1; width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; resize: none; box-sizing: border-box; min-height: 60px;"
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 12px 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent); display: flex; gap: 16px; flex-wrap: wrap;",
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{subfield}" }
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "N={sample_n}" }
                span { style: "font-size: 0.8rem; color: if p_value() < 0.05 { \"var(--qualia-accent)\" } else { \"var(--qualia-accent)\" };", "p={p_value:.3}" }
                span { style: "font-size: 0.8rem; color: var(--qualia-accent);", "d={effect_size:.2}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); width: 100%;", "QualiaDB → statistical engine | neuro-symbolic sieve | epistemic certainty" }
            }
        }
    }
}
