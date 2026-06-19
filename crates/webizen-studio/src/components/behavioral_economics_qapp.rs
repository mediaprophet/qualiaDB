use dioxus::prelude::*;

#[component]
pub fn BehavioralEconomicsQapp() -> Element {
    let mut bias_or_heuristic = use_signal(|| "Loss Aversion".to_string());
    let mut nudge_intervention = use_signal(|| "Default Option".to_string());
    let mut experiment_type = use_signal(|| "Field".to_string());
    let mut loss_aversion_lambda = use_signal(|| 2.5f64);
    let mut discount_rate_pct = use_signal(|| 5.0f64);
    let mut sample_n = use_signal(|| 200u32);
    let mut notes = use_signal(|| String::new());

    let biases = [
        "Loss Aversion",
        "Anchoring",
        "Availability",
        "Representativeness",
        "Overconfidence",
        "Status Quo",
        "Hyperbolic Discounting",
        "Framing",
        "Sunk Cost",
        "Herding",
    ];
    let nudges = [
        "Default Option",
        "Social Norm",
        "Commitment Device",
        "Simplification",
        "Feedback",
        "Incentive",
        "Framing Change",
    ];
    let experiments = ["Lab", "Field", "Natural", "Randomised Controlled", "Online"];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Behavioral Economics" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Bias / Heuristic" }
                    select {
                        value: "{bias_or_heuristic}",
                        onchange: move |e| bias_or_heuristic.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in biases { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Nudge Intervention" }
                    select {
                        value: "{nudge_intervention}",
                        onchange: move |e| nudge_intervention.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in nudges { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Experiment Type" }
                    select {
                        value: "{experiment_type}",
                        onchange: move |e| experiment_type.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in experiments { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Loss Aversion λ (1.0–5.0): {loss_aversion_lambda:.2}" }
                    input {
                        r#type: "range",
                        min: "1.0",
                        max: "5.0",
                        step: "0.1",
                        value: "{loss_aversion_lambda}",
                        oninput: move |e| loss_aversion_lambda.set(e.value().parse().unwrap_or(2.5)),
                        style: "width: 100%; margin-top: 4px;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Discount Rate %: {discount_rate_pct:.1}" }
                    input {
                        r#type: "number",
                        value: "{discount_rate_pct}",
                        oninput: move |e| discount_rate_pct.set(e.value().parse().unwrap_or(5.0)),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Sample N" }
                input {
                    r#type: "number",
                    value: "{sample_n}",
                    oninput: move |e| sample_n.set(e.value().parse().unwrap_or(200)),
                    style: "width: 200px; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{bias_or_heuristic} | {nudge_intervention} | λ={loss_aversion_lambda:.2} | N={sample_n}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → prospect theory engine | nudge sieve | experiment design graph" }
            }
        }
    }
}
