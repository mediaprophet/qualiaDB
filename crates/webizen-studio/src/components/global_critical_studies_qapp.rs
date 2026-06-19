use dioxus::prelude::*;

#[component]
pub fn GlobalCriticalStudiesQapp() -> Element {
    let mut critical_approach = use_signal(|| "World-Systems Analysis".to_string());
    let mut global_order = use_signal(|| "Liberal International".to_string());
    let mut inequality_metric = use_signal(|| "Gini Global".to_string());
    let mut power_asymmetry = use_signal(|| 50u32);
    let mut south_north_transfer = use_signal(|| 100.0f64);
    let mut notes = use_signal(|| String::new());

    let critical_approaches = [
        "World-Systems Analysis",
        "Postcolonial",
        "Dependency Theory",
        "Global Governance Critique",
        "Transnational Feminism",
        "Cosmopolitan Critique",
    ];
    let global_orders = [
        "Unipolar",
        "Multipolar",
        "Hegemonic Stability",
        "Liberal International",
        "Post-Western",
    ];
    let inequality_metrics = [
        "Gini Global",
        "HDI Variance",
        "Trade Imbalance",
        "Debt Ratio",
    ];

    let transfer_display = (south_north_transfer() * 10.0) as u32;

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Global Critical Studies" }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Critical Approach" }
                select {
                    value: "{critical_approach}", onchange: move |e| critical_approach.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in critical_approaches { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Global Order" }
                select {
                    value: "{global_order}", onchange: move |e| global_order.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in global_orders { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Inequality Metric" }
                select {
                    value: "{inequality_metric}", onchange: move |e| inequality_metric.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in inequality_metrics { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Power Asymmetry: {power_asymmetry}" }
                input { r#type: "range", min: "0", max: "100", value: "{power_asymmetry}",
                    oninput: move |e| power_asymmetry.set(e.value().parse().unwrap_or(50)),
                    style: "width: 100%; margin-top: 4px;" }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "South-North Transfer $B: {transfer_display}" }
                input { r#type: "range", min: "0", max: "5000", value: "{transfer_display}",
                    oninput: move |e| {
                        let v: u32 = e.value().parse().unwrap_or(1000);
                        south_north_transfer.set(v as f64 / 10.0);
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{critical_approach} | {global_order} | {inequality_metric} | asymmetry: {power_asymmetry}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → global critical studies engine | discourse sieve | anchor" }
            }
        }
    }
}
