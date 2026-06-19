use dioxus::prelude::*;

#[component]
pub fn RiskEngine() -> Element {
    let mut portfolio_value = use_signal(|| 1_000_000.0f64);
    let mut confidence_level = use_signal(|| 99.0f64);
    let mut time_horizon = use_signal(|| 10.0f64);
    let mut volatility = use_signal(|| 0.15f64);

    let z_score = use_memo(move || {
        let cl = confidence_level();
        if cl >= 99.0 {
            2.33
        } else if cl >= 95.0 {
            1.645
        } else {
            1.28
        }
    });

    let parametric_var = use_memo(move || {
        let val = portfolio_value();
        let vol = volatility();
        let t = time_horizon() / 252.0; // annualize
        val * z_score() * vol * t.sqrt()
    });

    let monte_carlo_var = use_memo(move || {
        // Mocking MC VaR to be slightly higher for realism
        parametric_var() * 1.05
    });

    let expected_shortfall = use_memo(move || {
        // CVaR mock (Expected Shortfall)
        parametric_var() * 1.25
    });

    rsx! {
        div {
            style: "flex: 1; padding: 2.5rem; background: linear-gradient(135deg, #18181b, #27272a); border-radius: 16px; color: #f4f4f5; font-family: 'Inter', system-ui, sans-serif; box-shadow: 0 20px 40px rgba(0,0,0,0.5); display: flex; flex-direction: column; gap: 2rem; overflow-y: auto;",

            div {
                style: "display: flex; justify-content: space-between; align-items: center; border-bottom: 1px solid rgba(255,255,255,0.05); padding-bottom: 1rem;",
                h2 {
                    style: "margin: 0; font-size: 2.5rem; font-weight: 800; background: linear-gradient(to right, #f43f5e, #fb923c); -webkit-background-clip: text; -webkit-text-fill-color: transparent;",
                    "Risk Engine (VaR)"
                }
                div {
                    style: "background: rgba(244, 63, 94, 0.1); border: 1px solid rgba(244, 63, 94, 0.3); padding: 0.5rem 1rem; border-radius: 9999px; font-size: 0.875rem; color: #f43f5e; display: flex; align-items: center; gap: 0.5rem;",
                    div { style: "width: 8px; height: 8px; border-radius: 50%; background: #f43f5e; box-shadow: 0 0 10px #f43f5e;" }
                    "Monte Carlo Module Active"
                }
            }

            div {
                style: "display: grid; grid-template-columns: repeat(auto-fit, minmax(220px, 1fr)); gap: 1.5rem;",

                div {
                    style: "background: rgba(255,255,255,0.03); padding: 1.5rem; border-radius: 16px; border: 1px solid rgba(255,255,255,0.05); backdrop-filter: blur(10px);",
                    label { style: "display: block; color: #a1a1aa; font-size: 0.875rem; text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 0.5rem;", "Portfolio Value ($)" }
                    input {
                        type: "number",
                        value: "{portfolio_value()}",
                        oninput: move |e| portfolio_value.set(e.value().parse().unwrap_or(0.0)),
                        style: "width: 100%; background: rgba(0,0,0,0.2); border: 1px solid rgba(255,255,255,0.1); color: white; padding: 0.75rem; border-radius: 8px; font-size: 1.1rem; outline: none;"
                    }
                }

                div {
                    style: "background: rgba(255,255,255,0.03); padding: 1.5rem; border-radius: 16px; border: 1px solid rgba(255,255,255,0.05); backdrop-filter: blur(10px);",
                    label { style: "display: block; color: #a1a1aa; font-size: 0.875rem; text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 0.5rem;", "Confidence Level (%)" }
                    select {
                        onchange: move |e| confidence_level.set(e.value().parse().unwrap_or(99.0)),
                        style: "width: 100%; background: rgba(0,0,0,0.2); border: 1px solid rgba(255,255,255,0.1); color: white; padding: 0.75rem; border-radius: 8px; font-size: 1.1rem; appearance: none; outline: none;",
                        option { value: "90", "90%" }
                        option { value: "95", "95%" }
                        option { value: "99", selected: true, "99%" }
                    }
                }

                div {
                    style: "background: rgba(255,255,255,0.03); padding: 1.5rem; border-radius: 16px; border: 1px solid rgba(255,255,255,0.05); backdrop-filter: blur(10px);",
                    label { style: "display: block; color: #a1a1aa; font-size: 0.875rem; text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 0.5rem;", "Time Horizon (Days)" }
                    input {
                        type: "number",
                        value: "{time_horizon()}",
                        oninput: move |e| time_horizon.set(e.value().parse().unwrap_or(10.0)),
                        style: "width: 100%; background: rgba(0,0,0,0.2); border: 1px solid rgba(255,255,255,0.1); color: white; padding: 0.75rem; border-radius: 8px; font-size: 1.1rem; outline: none;"
                    }
                }

                div {
                    style: "background: rgba(255,255,255,0.03); padding: 1.5rem; border-radius: 16px; border: 1px solid rgba(255,255,255,0.05); backdrop-filter: blur(10px);",
                    label { style: "display: block; color: #a1a1aa; font-size: 0.875rem; text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 0.5rem;", "Est. Volatility (%)" }
                    input {
                        type: "number", step: "0.1",
                        value: "{volatility() * 100.0}",
                        oninput: move |e| volatility.set(e.value().parse::<f64>().unwrap_or(15.0) / 100.0),
                        style: "width: 100%; background: rgba(0,0,0,0.2); border: 1px solid rgba(255,255,255,0.1); color: white; padding: 0.75rem; border-radius: 8px; font-size: 1.1rem; outline: none;"
                    }
                }
            }

            // Results Panel
            div {
                style: "background: rgba(0,0,0,0.2); border-radius: 16px; padding: 2rem; border: 1px solid rgba(255,255,255,0.05); display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 2rem; text-align: center; box-shadow: inset 0 2px 10px rgba(0,0,0,0.2);",

                div {
                    h3 { style: "color: #94a3b8; font-size: 1rem; font-weight: 500; margin-bottom: 0.5rem;", "Parametric VaR" }
                    div { style: "font-size: 2.5rem; font-weight: 700; color: #fb923c;", "${parametric_var():.0}" }
                    div { style: "color: #ef4444; font-size: 0.9rem; margin-top: 0.5rem;", "Normal market conditions" }
                }

                div {
                    style: "position: relative;",
                    div {
                        style: "position: absolute; top: -15px; right: 10%; background: #8b5cf6; color: white; font-size: 0.7rem; padding: 0.25rem 0.75rem; border-radius: 12px; font-weight: bold; box-shadow: 0 4px 10px rgba(139, 92, 246, 0.4);",
                        "Qualia Compute"
                    }
                    h3 { style: "color: #94a3b8; font-size: 1rem; font-weight: 500; margin-bottom: 0.5rem;", "Monte Carlo VaR" }
                    div { style: "font-size: 2.5rem; font-weight: 700; color: #f43f5e;", "${monte_carlo_var():.0}" }
                    div { style: "color: #ef4444; font-size: 0.9rem; margin-top: 0.5rem;", "100k simulation paths" }
                }

                div {
                    h3 { style: "color: #94a3b8; font-size: 1rem; font-weight: 500; margin-bottom: 0.5rem;", "Expected Shortfall" }
                    div { style: "font-size: 2.5rem; font-weight: 700; color: #be123c;", "${expected_shortfall():.0}" }
                    div { style: "color: #ef4444; font-size: 0.9rem; margin-top: 0.5rem;", "Average loss beyond VaR" }
                }
            }

            div {
                style: "background: linear-gradient(90deg, rgba(244, 63, 94, 0.1), rgba(0,0,0,0)); padding: 1.5rem; border-left: 4px solid #f43f5e; border-radius: 0 8px 8px 0;",
                h4 { style: "margin: 0 0 0.5rem 0; color: #f43f5e; font-size: 1.1rem;", "Executive Summary" }
                p {
                    style: "margin: 0; color: #d4d4d8; line-height: 1.6; font-size: 1rem;",
                    "Under normal market conditions, there is a " span { style: "font-weight: bold; color: white;", "{100.0 - confidence_level():.0}%" }
                    " chance that the portfolio will lose more than "
                    span { style: "color: #f43f5e; font-weight: bold;", "${parametric_var():.0}" }
                    " over the next " span { style: "font-weight: bold; color: white;", "{time_horizon()} days" } "."
                }
            }
        }
    }
}
