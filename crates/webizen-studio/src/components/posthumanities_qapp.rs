use dioxus::prelude::*;

#[component]
pub fn PosthumanitiesQapp() -> Element {
    let mut posthuman_strand = use_signal(|| "Transhumanism".to_string());
    let mut theoretical_touchstone = use_signal(|| "Haraway".to_string());
    let mut human_nonhuman_axis = use_signal(|| "Human-Animal".to_string());
    let mut ontological_status = use_signal(|| "Embodied".to_string());
    let mut agency = use_signal(|| 50u32);
    let mut notes = use_signal(|| String::new());

    let strands = [
        "Transhumanism",
        "Critical Posthumanism",
        "Feminist Posthumanism",
        "Multispecies",
        "Techno-Human",
        "Cyborg Studies",
    ];
    let touchstones = ["Haraway", "Hayles", "Braidotti", "Wolfe", "Stiegler"];
    let axes = [
        "Human-Animal",
        "Human-Machine",
        "Human-Environment",
        "Human-Microbiome",
    ];
    let statuses = [
        "Embodied",
        "Distributed",
        "Hybrid",
        "Networked",
        "Post-Biological",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Posthumanities" }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Posthuman Strand" }
                select {
                    value: "{posthuman_strand}",
                    onchange: move |e| posthuman_strand.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in strands { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Theoretical Touchstone" }
                select {
                    value: "{theoretical_touchstone}",
                    onchange: move |e| theoretical_touchstone.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in touchstones { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Human-Nonhuman Axis" }
                select {
                    value: "{human_nonhuman_axis}",
                    onchange: move |e| human_nonhuman_axis.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in axes { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Ontological Status" }
                select {
                    value: "{ontological_status}",
                    onchange: move |e| ontological_status.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in statuses { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Agency: {agency}" }
                input {
                    r#type: "range", min: "0", max: "100", value: "{agency}",
                    oninput: move |e| agency.set(e.value().parse().unwrap_or(50)),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{posthuman_strand} | {theoretical_touchstone} | {human_nonhuman_axis} | Agency: {agency}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → posthumanities engine | cyborg sieve | agency anchor" }
            }
        }
    }
}
