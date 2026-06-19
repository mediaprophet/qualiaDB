use dioxus::prelude::*;

#[component]
pub fn GenderStudiesQapp() -> Element {
    let mut theoretical_framework = use_signal(|| "Liberal Feminism".to_string());
    let mut gender_dimension = use_signal(|| "Identity".to_string());
    let mut methodology = use_signal(|| "Discourse Analysis".to_string());
    let mut site = use_signal(|| "Workplace".to_string());
    let mut gender_equity_index = use_signal(|| 50u32);
    let mut notes = use_signal(|| String::new());

    let theoretical_frameworks = [
        "Liberal Feminism",
        "Radical Feminism",
        "Socialist Feminism",
        "Poststructuralist",
        "Intersectionality",
        "Queer Theory",
        "Trans Studies",
    ];
    let gender_dimensions = [
        "Identity",
        "Expression",
        "Performance",
        "Embodiment",
        "Labour",
        "Violence",
        "Representation",
    ];
    let methodologies = [
        "Discourse Analysis",
        "Ethnographic",
        "Survey",
        "Archival",
        "Autoethnographic",
    ];
    let sites = [
        "Workplace",
        "Family",
        "Media",
        "Law",
        "Medicine",
        "Education",
        "Digital",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Gender Studies" }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Theoretical Framework" }
                select {
                    value: "{theoretical_framework}", onchange: move |e| theoretical_framework.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in theoretical_frameworks { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Gender Dimension" }
                select {
                    value: "{gender_dimension}", onchange: move |e| gender_dimension.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in gender_dimensions { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Methodology" }
                select {
                    value: "{methodology}", onchange: move |e| methodology.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in methodologies { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Site" }
                select {
                    value: "{site}", onchange: move |e| site.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in sites { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Gender Equity Index: {gender_equity_index}" }
                input { r#type: "range", min: "0", max: "100", value: "{gender_equity_index}",
                    oninput: move |e| gender_equity_index.set(e.value().parse().unwrap_or(50)),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{theoretical_framework} | {gender_dimension} | {methodology} | {site} | equity: {gender_equity_index}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → gender studies engine | discourse sieve | anchor" }
            }
        }
    }
}
