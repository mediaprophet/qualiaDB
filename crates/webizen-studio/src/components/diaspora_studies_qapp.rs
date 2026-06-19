use dioxus::prelude::*;

#[component]
pub fn DiasporaStudiesQapp() -> Element {
    let mut diaspora_type = use_signal(|| "Labour Migration".to_string());
    let mut origin_region = use_signal(|| "Asia".to_string());
    let mut host_region = use_signal(|| "Europe".to_string());
    let mut generation = use_signal(|| "1st".to_string());
    let mut cultural_retention = use_signal(|| 60u32);
    let mut remittance_flow = use_signal(|| 5.0f64);
    let mut notes = use_signal(|| String::new());

    let diaspora_types = [
        "Colonial",
        "Labour Migration",
        "Refugee",
        "Voluntary",
        "Forced Displacement",
        "Transnational",
    ];
    let regions = [
        "Africa",
        "Asia",
        "Americas",
        "Europe",
        "Middle East",
        "Oceania",
        "Multiple",
    ];
    let generations = ["1st", "2nd", "3rd+", "Transnational"];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",

            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Diaspora Studies" }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Diaspora Type" }
                select {
                    value: "{diaspora_type}",
                    onchange: move |e| diaspora_type.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in diaspora_types { option { value: "{x}", "{x}" } }
                }
            }

            div {
                style: "display: flex; gap: 12px;",
                div {
                    style: "flex: 1;",
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Origin Region" }
                    select {
                        value: "{origin_region}",
                        onchange: move |e| origin_region.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in regions { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    style: "flex: 1;",
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Host Region" }
                    select {
                        value: "{host_region}",
                        onchange: move |e| host_region.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in regions { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Generation" }
                select {
                    value: "{generation}",
                    onchange: move |e| generation.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in generations { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Cultural Retention: {cultural_retention}%" }
                input {
                    r#type: "range", min: "0", max: "100",
                    value: "{cultural_retention}",
                    oninput: move |e| cultural_retention.set(e.value().parse().unwrap_or(60)),
                    style: "width: 100%; margin-top: 4px;"
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Remittance Flow $B: {remittance_flow:.1}" }
                input {
                    r#type: "range", min: "0", max: "100",
                    value: "{remittance_flow() * 2.0}",
                    oninput: move |e| remittance_flow.set(e.value().parse::<f64>().unwrap_or(10.0) / 2.0),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{diaspora_type} | {origin_region} → {host_region} | Gen: {generation} | Retention: {cultural_retention}% | ${remittance_flow:.1}B" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → knowledge engine | sieve | anchor" }
            }
        }
    }
}
