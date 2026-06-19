use dioxus::prelude::*;

#[component]
pub fn LeisureStudiesQapp() -> Element {
    let mut leisure_domain = use_signal(|| "Outdoor Recreation".to_string());
    let mut activity_type = use_signal(|| "Active".to_string());
    let mut participation_frequency = use_signal(|| "Weekly".to_string());
    let mut wellbeing_impact = use_signal(|| 50u32);
    let mut economic_value = use_signal(|| 25.0f64);
    let mut notes = use_signal(|| String::new());

    let domains = [
        "Outdoor Recreation",
        "Sport",
        "Tourism",
        "Games and Play",
        "Arts and Culture",
        "Community Events",
        "Digital Leisure",
    ];
    let activity_types = [
        "Active",
        "Passive",
        "Social",
        "Solitary",
        "Competitive",
        "Creative",
    ];
    let frequencies = ["Daily", "Weekly", "Monthly", "Seasonal"];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",

            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Leisure Studies" }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Leisure Domain" }
                select {
                    value: "{leisure_domain}",
                    onchange: move |e| leisure_domain.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in domains { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Activity Type" }
                select {
                    value: "{activity_type}",
                    onchange: move |e| activity_type.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in activity_types { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Participation Frequency" }
                select {
                    value: "{participation_frequency}",
                    onchange: move |e| participation_frequency.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in frequencies { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Wellbeing Impact: {wellbeing_impact}%" }
                input {
                    r#type: "range", min: "0", max: "100",
                    value: "{wellbeing_impact}",
                    oninput: move |e| wellbeing_impact.set(e.value().parse().unwrap_or(50)),
                    style: "width: 100%; margin-top: 4px;"
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Economic Value $M: {economic_value:.1}" }
                input {
                    r#type: "range", min: "0", max: "100",
                    value: "{economic_value() * 0.2}",
                    oninput: move |e| economic_value.set(e.value().parse::<f64>().unwrap_or(5.0) * 5.0),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Domain: {leisure_domain} | {activity_type} | {participation_frequency} | Wellbeing: {wellbeing_impact}%" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → knowledge engine | sieve | anchor" }
            }
        }
    }
}
