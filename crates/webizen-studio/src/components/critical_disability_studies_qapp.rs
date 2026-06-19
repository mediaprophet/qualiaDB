use dioxus::prelude::*;

#[component]
pub fn CriticalDisabilityStudiesQapp() -> Element {
    let mut model = use_signal(|| "Social Model".to_string());
    let mut impairment_type = use_signal(|| "Physical".to_string());
    let mut access_barrier = use_signal(|| "Physical".to_string());
    let mut legal_framework = use_signal(|| "CRPD".to_string());
    let mut accessibility_score = use_signal(|| 50u32);
    let mut notes = use_signal(|| String::new());

    let models = [
        "Social Model",
        "Cultural Model",
        "Crip Theory",
        "Posthuman Disability",
        "Biopsychosocial",
        "Human Rights",
    ];
    let impairment_types = [
        "Physical",
        "Sensory",
        "Cognitive",
        "Psychiatric",
        "Chronic Illness",
        "Multiple",
    ];
    let access_barriers = [
        "Physical",
        "Attitudinal",
        "Communication",
        "Institutional",
        "Digital",
    ];
    let legal_frameworks = ["ADA", "CRPD", "Equality Act", "NDIS", "Other"];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Critical Disability Studies" }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Model" }
                select {
                    value: "{model}", onchange: move |e| model.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in models { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Impairment Type" }
                select {
                    value: "{impairment_type}", onchange: move |e| impairment_type.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in impairment_types { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Access Barrier" }
                select {
                    value: "{access_barrier}", onchange: move |e| access_barrier.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in access_barriers { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Legal Framework" }
                select {
                    value: "{legal_framework}", onchange: move |e| legal_framework.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in legal_frameworks { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Accessibility Score: {accessibility_score}" }
                input { r#type: "range", min: "0", max: "100", value: "{accessibility_score}",
                    oninput: move |e| accessibility_score.set(e.value().parse().unwrap_or(50)),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{model} | {impairment_type} | {access_barrier} | {legal_framework} | score: {accessibility_score}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → disability studies engine | discourse sieve | anchor" }
            }
        }
    }
}
