use dioxus::prelude::*;

#[component]
pub fn BiomathematicsQapp() -> Element {
    let mut model_type = use_signal(|| "SIR Epidemiological".to_string());
    let mut system_type = use_signal(|| "Epidemiology".to_string());
    let mut parameter_r = use_signal(|| 1.2f64);
    let mut parameter_k = use_signal(|| 1000u32);
    let mut equilibrium_stable = use_signal(|| true);
    let mut bifurcation_parameter = use_signal(|| 1.0f64);
    let mut notes = use_signal(|| String::new());

    let models = [
        "Lotka-Volterra",
        "SIR Epidemiological",
        "Logistic Growth",
        "Turing Reaction-Diffusion",
        "Neural Field",
        "Hodgkin-Huxley",
        "Game Theory Evolutionary",
        "Markov Chain",
        "Bayesian Network",
    ];
    let systems = [
        "Population Dynamics",
        "Epidemiology",
        "Neuroscience",
        "Genetics",
        "Morphogenesis",
        "Ecological Network",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Biomathematics" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Model Type" }
                    select {
                        value: "{model_type}",
                        onchange: move |e| model_type.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in models { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "System Type" }
                    select {
                        value: "{system_type}",
                        onchange: move |e| system_type.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in systems { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Growth Rate r: {parameter_r:.2}" }
                    input {
                        r#type: "number",
                        step: "0.1",
                        value: "{parameter_r}",
                        oninput: move |e| parameter_r.set(e.value().parse().unwrap_or(1.2)),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Carrying Capacity K" }
                    input {
                        r#type: "number",
                        value: "{parameter_k}",
                        oninput: move |e| parameter_k.set(e.value().parse().unwrap_or(1000)),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Bifurcation Parameter (0.0–2.0): {bifurcation_parameter:.2}" }
                input {
                    r#type: "range",
                    min: "0.0",
                    max: "2.0",
                    step: "0.01",
                    value: "{bifurcation_parameter}",
                    oninput: move |e| bifurcation_parameter.set(e.value().parse().unwrap_or(1.0)),
                    style: "width: 100%; margin-top: 4px;"
                }
            }

            div {
                style: "display: flex; align-items: center; gap: 8px;",
                input {
                    r#type: "checkbox",
                    checked: "{equilibrium_stable}",
                    onchange: move |e| equilibrium_stable.set(e.checked()),
                    id: "eq_stable"
                }
                label { r#for: "eq_stable", style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Equilibrium Stable" }
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{model_type} | {system_type} | r={parameter_r:.2} | K={parameter_k} | Stable: {equilibrium_stable}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → ODE solver engine | bifurcation sieve | dynamical systems graph" }
            }
        }
    }
}
