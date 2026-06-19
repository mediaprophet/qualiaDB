use dioxus::prelude::*;

#[component]
pub fn CapitalStudiesQapp() -> Element {
    let mut capital_form = use_signal(|| "Economic".to_string());
    let mut theorist = use_signal(|| "Bourdieu".to_string());
    let mut accumulation_regime = use_signal(|| "Industrial".to_string());
    let mut inequality_index = use_signal(|| 0.5f64);
    let mut gdp_growth = use_signal(|| 2.5f64);
    let mut notes = use_signal(|| String::new());

    let capital_forms = [
        "Economic", "Cultural", "Social", "Symbolic", "Human", "Bodily", "Digital",
    ];
    let theorists = ["Bourdieu", "Marx", "Becker", "Putnam", "Harvey", "Sassen"];
    let accumulation_regimes = [
        "Industrial",
        "Financial",
        "Cognitive",
        "Platform",
        "Rentier",
    ];

    let ineq_display = (inequality_index() * 100.0) as u32;
    let gdp_display = (gdp_growth() * 10.0) as u32;

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Capital Studies" }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Capital Form" }
                select {
                    value: "{capital_form}", onchange: move |e| capital_form.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in capital_forms { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Theorist" }
                select {
                    value: "{theorist}", onchange: move |e| theorist.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in theorists { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Accumulation Regime" }
                select {
                    value: "{accumulation_regime}", onchange: move |e| accumulation_regime.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in accumulation_regimes { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Inequality Index (Gini): {ineq_display}%" }
                input { r#type: "range", min: "0", max: "100", value: "{ineq_display}",
                    oninput: move |e| {
                        let v: u32 = e.value().parse().unwrap_or(50);
                        inequality_index.set(v as f64 / 100.0);
                    },
                    style: "width: 100%; margin-top: 4px;" }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "GDP Growth (0–10%): {gdp_display}" }
                input { r#type: "range", min: "0", max: "100", value: "{gdp_display}",
                    oninput: move |e| {
                        let v: u32 = e.value().parse().unwrap_or(25);
                        gdp_growth.set(v as f64 / 10.0);
                    },
                    style: "width: 100%; margin-top: 4px;" }
            }
            div {
                style: "flex: 1; display: flex; flex-direction: column;",
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Notes" }
                textarea { value: "{notes}", oninput: move |e| notes.set(e.value()),
                    style: "flex: 1; width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; resize: none; box-sizing: border-box; min-height: 60px;" }
            }
            div {
                style: "background: var(--qualia-bg); padding: 12px 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent);",
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{capital_form} | {theorist} | {accumulation_regime} | Gini: {ineq_display}%" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → capital studies engine | discourse sieve | anchor" }
            }
        }
    }
}
