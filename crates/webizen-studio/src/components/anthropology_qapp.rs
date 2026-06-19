use dioxus::prelude::*;

#[component]
pub fn AnthropologyQapp() -> Element {
    let mut subfield = use_signal(|| "Cultural Anthropology".to_string());
    let mut method = use_signal(|| "Ethnography".to_string());
    let mut fieldsite = use_signal(|| String::new());
    let mut kinship_structure = use_signal(|| "Nuclear Family".to_string());
    let mut economic_mode = use_signal(|| "Foraging / Hunter-Gatherer".to_string());
    let mut temporal_depth = use_signal(|| "Contemporary".to_string());
    let mut field_notes = use_signal(|| String::new());

    let subfields = [
        "Cultural Anthropology",
        "Physical / Biological Anthropology",
        "Linguistic Anthropology",
        "Archaeological Anthropology",
        "Medical Anthropology",
        "Visual Anthropology",
        "Cognitive Anthropology",
        "Environmental Anthropology",
        "Digital Anthropology",
    ];
    let methods = [
        "Ethnography",
        "Participant Observation",
        "Structured Interview",
        "Semi-Structured Interview",
        "Survey",
        "Life History",
        "Cross-Cultural Comparison",
        "Network Analysis",
        "Archival Research",
    ];
    let kinships = [
        "Nuclear Family",
        "Extended Family",
        "Lineage (Patrilineal)",
        "Lineage (Matrilineal)",
        "Bilateral / Cognatic",
        "Clan / Moiety",
        "Fictive Kinship",
        "Blended / Reconstituted",
    ];
    let economies = [
        "Foraging / Hunter-Gatherer",
        "Pastoral / Herding",
        "Horticulture",
        "Agriculture (Subsistence)",
        "Agriculture (Market)",
        "Industrial",
        "Post-Industrial / Service",
        "Gift Economy",
        "Mixed",
    ];
    let temporals = [
        "Contemporary",
        "20th Century",
        "19th Century",
        "Pre-Colonial",
        "Colonial",
        "Post-Colonial",
        "Deep Time (Archaeological)",
        "Longitudinal",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Anthropology" }

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
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Research Method" }
                    select {
                        value: "{method}",
                        onchange: move |e| method.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for x in methods { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Temporal Depth" }
                    select {
                        value: "{temporal_depth}",
                        onchange: move |e| temporal_depth.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for x in temporals { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Kinship Structure" }
                    select {
                        value: "{kinship_structure}",
                        onchange: move |e| kinship_structure.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for x in kinships { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Economic Mode of Production" }
                    select {
                        value: "{economic_mode}",
                        onchange: move |e| economic_mode.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for x in economies { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Field Site / Community" }
                input {
                    r#type: "text", placeholder: "e.g. rural Oaxaca, urban Jakarta, online gaming community…",
                    value: "{fieldsite}",
                    oninput: move |e| fieldsite.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                }
            }

            div {
                style: "flex: 1; display: flex; flex-direction: column;",
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Field Notes / Thick Description" }
                textarea {
                    value: "{field_notes}",
                    oninput: move |e| field_notes.set(e.value()),
                    style: "flex: 1; width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; resize: none; box-sizing: border-box; min-height: 80px;"
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 12px 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent); display: flex; gap: 16px; flex-wrap: wrap;",
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{subfield}" }
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Method: {method}" }
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Kinship: {kinship_structure}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); width: 100%;", "QualiaDB → graph theory | Allen Interval | neuro-symbolic ethnographic sieve" }
            }

            // Pilot: live QualiaDB analysis contract (stub in browser, real on desktop).
            crate::components::qapp_engine::EnginePanel {
                discipline: "Anthropology".to_string(),
                fields: vec![
                    ("Subfield".to_string(), subfield()),
                    ("Research Method".to_string(), method()),
                    ("Kinship Structure".to_string(), kinship_structure()),
                    ("Economic Mode".to_string(), economic_mode()),
                    ("Temporal Depth".to_string(), temporal_depth()),
                    ("Field Site".to_string(), fieldsite()),
                ],
                notes: field_notes(),
            }
        }
    }
}
