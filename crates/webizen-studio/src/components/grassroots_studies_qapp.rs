use dioxus::prelude::*;

#[component]
pub fn GrassrootsStudiesQapp() -> Element {
    let mut movement_type = use_signal(|| "Community Organising".to_string());
    let mut tactic = use_signal(|| "Direct Action".to_string());
    let mut power_analysis = use_signal(|| "Local".to_string());
    let mut participant_count = use_signal(|| 1000u32);
    let mut victory_index = use_signal(|| 50u32);
    let mut notes = use_signal(|| String::new());

    let movement_types = [
        "Community Organising",
        "Labour Movement",
        "Environmental",
        "Civil Rights",
        "Indigenous Rights",
        "Housing",
        "Food Sovereignty",
    ];
    let tactics = [
        "Direct Action",
        "Coalition Building",
        "Mutual Aid",
        "Legal Advocacy",
        "Cultural Production",
        "Digital Organising",
    ];
    let power_analyses = ["Local", "Regional", "National", "Transnational"];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Grassroots Studies" }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Movement Type" }
                select {
                    value: "{movement_type}", onchange: move |e| movement_type.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in movement_types { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Tactic" }
                select {
                    value: "{tactic}", onchange: move |e| tactic.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in tactics { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Power Analysis" }
                select {
                    value: "{power_analysis}", onchange: move |e| power_analysis.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in power_analyses { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Participant Count: {participant_count}" }
                input { r#type: "range", min: "0", max: "1000000", value: "{participant_count}",
                    oninput: move |e| participant_count.set(e.value().parse().unwrap_or(1000)),
                    style: "width: 100%; margin-top: 4px;" }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Victory Index: {victory_index}" }
                input { r#type: "range", min: "0", max: "100", value: "{victory_index}",
                    oninput: move |e| victory_index.set(e.value().parse().unwrap_or(50)),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{movement_type} | {tactic} | {power_analysis} | participants: {participant_count} | victory: {victory_index}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → grassroots studies engine | discourse sieve | anchor" }
            }
        }
    }
}
