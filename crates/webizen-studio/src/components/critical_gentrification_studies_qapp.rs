use dioxus::prelude::*;

#[component]
pub fn CriticalGentrificationStudiesQapp() -> Element {
    let mut gentrification_type = use_signal(|| "Classic".to_string());
    let mut driver = use_signal(|| "Real Estate Capital".to_string());
    let mut displacement_type = use_signal(|| "Direct".to_string());
    let mut rent_increase_pct = use_signal(|| 50u32);
    let mut displacement_rate = use_signal(|| 0.2f64);
    let mut notes = use_signal(|| String::new());

    let gentrification_types = [
        "Classic",
        "Super",
        "Commercial",
        "Rural",
        "Green",
        "Cognitive-Cultural",
        "Tourism",
    ];
    let drivers = [
        "Real Estate Capital",
        "Policy",
        "Amenity Clustering",
        "Creative Class",
        "Financialisation",
    ];
    let displacement_types = ["Direct", "Exclusionary", "Cultural", "Indirect"];

    let disp_display = (displacement_rate() * 100.0) as u32;

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Critical Gentrification Studies" }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Gentrification Type" }
                select {
                    value: "{gentrification_type}", onchange: move |e| gentrification_type.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in gentrification_types { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Driver" }
                select {
                    value: "{driver}", onchange: move |e| driver.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in drivers { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Displacement Type" }
                select {
                    value: "{displacement_type}", onchange: move |e| displacement_type.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in displacement_types { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Rent Increase %: {rent_increase_pct}" }
                input { r#type: "range", min: "0", max: "300", value: "{rent_increase_pct}",
                    oninput: move |e| rent_increase_pct.set(e.value().parse().unwrap_or(50)),
                    style: "width: 100%; margin-top: 4px;" }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Displacement Rate: {disp_display}%" }
                input { r#type: "range", min: "0", max: "100", value: "{disp_display}",
                    oninput: move |e| {
                        let v: u32 = e.value().parse().unwrap_or(20);
                        displacement_rate.set(v as f64 / 100.0);
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{gentrification_type} | {driver} | {displacement_type} | rent: +{rent_increase_pct}% | disp: {disp_display}%" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → gentrification engine | discourse sieve | anchor" }
            }
        }
    }
}
