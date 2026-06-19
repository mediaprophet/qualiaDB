use dioxus::prelude::*;

#[component]
pub fn PhilanthropyAndNonprofitStudiesQapp() -> Element {
    let mut organisation_type = use_signal(|| "Foundation".to_string());
    let mut giving_model = use_signal(|| "Direct Giving".to_string());
    let mut cause_area = use_signal(|| "Health".to_string());
    let mut grant_size = use_signal(|| 250.0f64);
    let mut impact_metric = use_signal(|| 50u32);
    let mut notes = use_signal(|| String::new());

    let org_types = [
        "Foundation",
        "Charity",
        "Social Enterprise",
        "NGO",
        "Community Foundation",
        "Corporate Philanthropy",
    ];
    let giving_models = [
        "Direct Giving",
        "Venture Philanthropy",
        "Impact Investing",
        "Crowdfunding",
        "Endowment",
    ];
    let cause_areas = [
        "Health",
        "Education",
        "Environment",
        "Human Rights",
        "Arts",
        "Poverty Alleviation",
        "Research",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",

            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Philanthropy & Nonprofit Studies" }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Organisation Type" }
                select {
                    value: "{organisation_type}",
                    onchange: move |e| organisation_type.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in org_types { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Giving Model" }
                select {
                    value: "{giving_model}",
                    onchange: move |e| giving_model.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in giving_models { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Cause Area" }
                select {
                    value: "{cause_area}",
                    onchange: move |e| cause_area.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in cause_areas { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Grant Size $K: {grant_size:.0}" }
                input {
                    r#type: "range", min: "0", max: "100",
                    value: "{grant_size() / 50.0}",
                    oninput: move |e| grant_size.set(e.value().parse::<f64>().unwrap_or(5.0) * 50.0),
                    style: "width: 100%; margin-top: 4px;"
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Impact Metric: {impact_metric}%" }
                input {
                    r#type: "range", min: "0", max: "100",
                    value: "{impact_metric}",
                    oninput: move |e| impact_metric.set(e.value().parse().unwrap_or(50)),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{organisation_type} | {giving_model} | {cause_area} | Grant: ${grant_size:.0}K | Impact: {impact_metric}%" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → knowledge engine | sieve | anchor" }
            }
        }
    }
}
