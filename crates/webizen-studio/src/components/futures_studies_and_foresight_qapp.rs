use dioxus::prelude::*;

#[component]
pub fn FuturesStudiesAndForesightQapp() -> Element {
    let mut foresight_method = use_signal(|| "Delphi".to_string());
    let mut time_horizon = use_signal(|| "Short-term 1–5yr".to_string());
    let mut certainty_level = use_signal(|| 50u32);
    let mut desirability = use_signal(|| 50u32);
    let mut signal_type = use_signal(|| "Emerging Issue".to_string());
    let mut notes = use_signal(|| String::new());

    let methods = [
        "Delphi",
        "Scenario Planning",
        "Horizon Scanning",
        "Causal Layered Analysis",
        "Backcasting",
        "Technology Roadmapping",
    ];
    let horizons = [
        "Short-term 1–5yr",
        "Medium 5–20yr",
        "Long-term 20–50yr",
        "Transformative 50yr+",
    ];
    let signals = ["Emerging Issue", "Wild Card", "Megatrend", "Weak Signal"];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",

            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Futures Studies & Foresight" }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Foresight Method" }
                select {
                    value: "{foresight_method}",
                    onchange: move |e| foresight_method.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in methods { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Time Horizon" }
                select {
                    value: "{time_horizon}",
                    onchange: move |e| time_horizon.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in horizons { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Certainty Level: {certainty_level}%" }
                input {
                    r#type: "range", min: "0", max: "100",
                    value: "{certainty_level}",
                    oninput: move |e| certainty_level.set(e.value().parse().unwrap_or(50)),
                    style: "width: 100%; margin-top: 4px;"
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Desirability: {desirability}%" }
                input {
                    r#type: "range", min: "0", max: "100",
                    value: "{desirability}",
                    oninput: move |e| desirability.set(e.value().parse().unwrap_or(50)),
                    style: "width: 100%; margin-top: 4px;"
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Signal Type" }
                select {
                    value: "{signal_type}",
                    onchange: move |e| signal_type.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in signals { option { value: "{x}", "{x}" } }
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Horizon: {time_horizon} | Method: {foresight_method} | Certainty: {certainty_level}%" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → knowledge engine | sieve | anchor" }
            }
        }
    }
}
