use dioxus::prelude::*;

#[component]
pub fn EconomicsQapp() -> Element {
    let mut school = use_signal(|| "Keynesian".to_string());
    let mut model_type = use_signal(|| "Macro: Aggregate Demand".to_string());
    let mut gdp = use_signal(|| 0.0f64);
    let mut inflation_rate = use_signal(|| 2.5f64);
    let mut unemployment_rate = use_signal(|| 4.0f64);
    let mut interest_rate = use_signal(|| 5.0f64);
    let mut elasticity_type = use_signal(|| "Price Elasticity of Demand".to_string());
    let mut elasticity_val = use_signal(|| -1.2f64);
    let mut analysis_notes = use_signal(|| String::new());

    let schools = [
        "Classical / Smithian",
        "Marxian / Radical",
        "Keynesian",
        "Monetarist (Friedman)",
        "New Classical",
        "New Keynesian",
        "Austrian",
        "Institutional",
        "Behavioural",
        "Post-Keynesian",
        "Modern Monetary Theory (MMT)",
    ];
    let models = [
        "Macro: Aggregate Demand",
        "Macro: IS-LM",
        "Macro: AS-AD",
        "Macro: Solow Growth Model",
        "Micro: Supply & Demand",
        "Micro: Game Theory",
        "Micro: Auction Theory",
        "Micro: General Equilibrium (Walrasian)",
        "Trade: Comparative Advantage",
        "Trade: Heckscher-Ohlin",
        "Labour: Monopsony",
        "Finance: CAPM",
        "Finance: Black-Scholes",
    ];
    let elasticities = [
        "Price Elasticity of Demand",
        "Price Elasticity of Supply",
        "Income Elasticity",
        "Cross-Price Elasticity",
        "Wage Elasticity of Labour Supply",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Economics" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "School of Thought" }
                    select {
                        value: "{school}",
                        onchange: move |e| school.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for x in schools { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Economic Model" }
                    select {
                        value: "{model_type}",
                        onchange: move |e| model_type.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for x in models { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                style: "display: grid; grid-template-columns: repeat(4, 1fr); gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "GDP ($B)" }
                    input {
                        r#type: "number", step: "0.1",
                        value: "{gdp}",
                        oninput: move |e| gdp.set(e.value().parse().unwrap_or(0.0)),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Inflation (%)" }
                    input {
                        r#type: "number", step: "0.1",
                        value: "{inflation_rate}",
                        oninput: move |e| inflation_rate.set(e.value().parse().unwrap_or(2.5)),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Unemployment (%)" }
                    input {
                        r#type: "number", step: "0.1",
                        value: "{unemployment_rate}",
                        oninput: move |e| unemployment_rate.set(e.value().parse().unwrap_or(4.0)),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Interest Rate (%)" }
                    input {
                        r#type: "number", step: "0.25",
                        value: "{interest_rate}",
                        oninput: move |e| interest_rate.set(e.value().parse().unwrap_or(5.0)),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
            }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Elasticity Type" }
                    select {
                        value: "{elasticity_type}",
                        onchange: move |e| elasticity_type.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for x in elasticities { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Elasticity Value (ε)" }
                    input {
                        r#type: "number", step: "0.01",
                        value: "{elasticity_val}",
                        oninput: move |e| elasticity_val.set(e.value().parse().unwrap_or(-1.2)),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{school}" }
                span { style: "font-size: 0.8rem; color: var(--qualia-accent);", "GDP: ${gdp:.1}B" }
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "π={inflation_rate:.1}% u={unemployment_rate:.1}% r={interest_rate:.2}%" }
                span { style: "font-size: 0.8rem; color: var(--qualia-accent);", "ε={elasticity_val:.2}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); width: 100%;", "QualiaDB → numerical solver | Allen Interval time series | graph theory equilibrium" }
            }

            // Pilot: live QualiaDB analysis contract (stub in browser, real on desktop).
            crate::components::qapp_engine::EnginePanel {
                discipline: "Economics".to_string(),
                fields: vec![
                    ("School".to_string(), school()),
                    ("Model Type".to_string(), model_type()),
                    ("GDP (B)".to_string(), format!("{:.1}", gdp())),
                    ("Inflation".to_string(), format!("{:.1}%", inflation_rate())),
                    ("Unemployment".to_string(), format!("{:.1}%", unemployment_rate())),
                    ("Interest Rate".to_string(), format!("{:.2}%", interest_rate())),
                    ("Elasticity Type".to_string(), elasticity_type()),
                    ("Elasticity".to_string(), format!("{:.2}", elasticity_val())),
                ],
                notes: analysis_notes(),
            }
        }
    }
}
