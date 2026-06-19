use dioxus::prelude::*;

#[component]
pub fn MathematicalEconomicsQapp() -> Element {
    let mut framework = use_signal(|| "General Equilibrium".to_string());
    let mut market_structure = use_signal(|| "Perfect Competition".to_string());
    let mut equilibrium_type = use_signal(|| "Nash".to_string());
    let mut elasticity = use_signal(|| 1.0f64);
    let mut utility = use_signal(|| 50u32);
    let mut notes = use_signal(|| String::new());

    let frameworks = [
        "General Equilibrium",
        "Game Theory",
        "Mechanism Design",
        "Optimal Control",
        "Dynamic Programming",
        "Stochastic Calculus",
    ];
    let market_structures = [
        "Perfect Competition",
        "Monopoly",
        "Oligopoly",
        "Monopolistic",
        "Two-Sided Market",
    ];
    let equilibrium_types = [
        "Nash",
        "Walrasian",
        "Pareto Optimal",
        "Bayesian Nash",
        "Correlated",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",

            h2 {
                style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;",
                "Mathematical Economics"
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Framework" }
                select {
                    value: "{framework}",
                    onchange: move |e| framework.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in frameworks { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Market Structure" }
                select {
                    value: "{market_structure}",
                    onchange: move |e| market_structure.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in market_structures { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Equilibrium Type" }
                select {
                    value: "{equilibrium_type}",
                    onchange: move |e| equilibrium_type.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in equilibrium_types { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Elasticity: {elasticity():.2}" }
                input {
                    r#type: "range", min: "0", max: "100",
                    value: "{elasticity() * 20.0}",
                    oninput: move |e| elasticity.set(e.value().parse::<f64>().unwrap_or(20.0) / 20.0),
                    style: "width: 100%; margin-top: 4px;"
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Utility: {utility}" }
                input {
                    r#type: "range", min: "0", max: "100",
                    value: "{utility}",
                    oninput: move |e| utility.set(e.value().parse().unwrap_or(50)),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{framework} | {market_structure} | {equilibrium_type} | ε={elasticity():.2} | U={utility}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → knowledge engine | sieve | anchor" }
            }
        }
    }
}
