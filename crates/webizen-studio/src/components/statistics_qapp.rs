use dioxus::prelude::*;

#[component]
pub fn StatisticsQapp() -> Element {
    let mut paradigm = use_signal(|| "Frequentist".to_string());
    let mut test_type = use_signal(|| "t-test".to_string());
    let mut alpha = use_signal(|| 0.05f64);
    let mut power = use_signal(|| 0.80f64);
    let mut sample_n = use_signal(|| 100u32);
    let mut effect_size_f = use_signal(|| 0.5f64);
    let mut confidence_level = use_signal(|| "95".to_string());
    let mut distribution = use_signal(|| "Normal".to_string());
    let mut notes = use_signal(|| String::new());

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Statistics QApp" }

            div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Statistical Paradigm" }
                    select {
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        onchange: move |e| paradigm.set(e.value()),
                        option { "Frequentist" }
                        option { "Bayesian" }
                        option { "Fiducial" }
                        option { "Likelihoodist" }
                        option { "Robust" }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Test Type" }
                    select {
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        onchange: move |e| test_type.set(e.value()),
                        option { "t-test" }
                        option { "ANOVA" }
                        option { "Chi-Square" }
                        option { "Mann-Whitney" }
                        option { "Kolmogorov-Smirnov" }
                        option { "Pearson r" }
                        option { "Spearman ρ" }
                        option { "Regression" }
                        option { "Factor Analysis" }
                        option { "SEM" }
                        option { "Mixed Models" }
                        option { "Survival Analysis" }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Confidence Level" }
                    select {
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        onchange: move |e| confidence_level.set(e.value()),
                        option { "90" }
                        option { "95" }
                        option { "99" }
                        option { "99.9" }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Distribution" }
                    select {
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        onchange: move |e| distribution.set(e.value()),
                        option { "Normal" }
                        option { "Binomial" }
                        option { "Poisson" }
                        option { "Exponential" }
                        option { "Beta" }
                        option { "Gamma" }
                        option { "t" }
                        option { "F" }
                        option { "χ²" }
                        option { "Dirichlet" }
                        option { "Cauchy" }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "α (significance): {alpha:.3}" }
                    input {
                        r#type: "range",
                        min: "0.001",
                        max: "0.2",
                        step: "0.001",
                        value: "{alpha}",
                        style: "width: 100%; margin-top: 8px;",
                        oninput: move |e| alpha.set(e.value().parse().unwrap_or(0.05)),
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Power (1-β): {power:.2}" }
                    input {
                        r#type: "range",
                        min: "0.50",
                        max: "0.999",
                        step: "0.001",
                        value: "{power}",
                        style: "width: 100%; margin-top: 8px;",
                        oninput: move |e| power.set(e.value().parse().unwrap_or(0.80)),
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Sample N" }
                    input {
                        r#type: "number",
                        value: "{sample_n}",
                        min: "2",
                        step: "1",
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        oninput: move |e| sample_n.set(e.value().parse().unwrap_or(100)),
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Effect Size (f / Cohen's d): {effect_size_f:.3}" }
                    input {
                        r#type: "range",
                        min: "0.0",
                        max: "2.0",
                        step: "0.01",
                        value: "{effect_size_f}",
                        style: "width: 100%; margin-top: 8px;",
                        oninput: move |e| effect_size_f.set(e.value().parse().unwrap_or(0.5)),
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Notes" }
                textarea {
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; min-height: 55px; box-sizing: border-box; resize: vertical;",
                    placeholder: "Data description, assumptions, prior distributions, software used...",
                    oninput: move |e| notes.set(e.value()),
                    "{notes}"
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent); flex: 1;",
                h3 { style: "margin-top: 0; color: var(--qualia-accent); font-size: 0.9rem;", "Output / Analysis" }
                div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 8px; font-size: 0.8rem;",
                    div { style: "color: var(--qualia-text-muted);", "Paradigm:" }
                    div { style: "color: var(--qualia-text);", "{paradigm}" }
                    div { style: "color: var(--qualia-text-muted);", "Test:" }
                    div { style: "color: var(--qualia-text);", "{test_type}" }
                    div { style: "color: var(--qualia-text-muted);", "α / Power:" }
                    div { style: "color: var(--qualia-text);", "{alpha:.3} / {power:.2}" }
                    div { style: "color: var(--qualia-text-muted);", "N:" }
                    div { style: "color: var(--qualia-text);", "{sample_n}" }
                    div { style: "color: var(--qualia-text-muted);", "Effect Size:" }
                    div { style: "color: var(--qualia-text);", "{effect_size_f:.3}" }
                    div { style: "color: var(--qualia-text-muted);", "CI / Distribution:" }
                    div { style: "color: var(--qualia-text);", "{confidence_level}% / {distribution}" }
                }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 12px; border-top: 1px solid var(--qualia-border); padding-top: 8px;",
                    "QualiaDB → statistical_analysis engine | Bayesian epistemic | ODE numerical solver"
                }
            }
        }
    }
}
