use dioxus::prelude::*;

#[component]
pub fn SocialActivismQapp() -> Element {
    let mut movement_type = use_signal(|| "Labour".to_string());
    let mut tactic = use_signal(|| "Protest".to_string());
    let mut theoretical_basis = use_signal(|| "Freire".to_string());
    let mut mobilisation = use_signal(|| 10000u32);
    let mut success_rate = use_signal(|| 50u32);
    let mut notes = use_signal(|| String::new());

    let movements = [
        "Labour",
        "Civil Rights",
        "Environmental",
        "Feminist",
        "LGBTQ+",
        "Indigenous",
        "Housing",
        "Anti-War",
        "Digital Rights",
    ];
    let tactics = [
        "Protest",
        "Strike",
        "Boycott",
        "Civil Disobedience",
        "Legal Challenge",
        "Electoral",
        "Digital Campaigning",
        "Community Organising",
    ];
    let bases = ["Alinsky", "Freire", "Gene Sharp", "Anarchist", "Socialist"];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Social Activism" }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Movement Type" }
                select {
                    value: "{movement_type}",
                    onchange: move |e| movement_type.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in movements { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Tactic" }
                select {
                    value: "{tactic}",
                    onchange: move |e| tactic.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in tactics { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Theoretical Basis" }
                select {
                    value: "{theoretical_basis}",
                    onchange: move |e| theoretical_basis.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in bases { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Mobilisation: {mobilisation}" }
                input {
                    r#type: "range", min: "0", max: "1000000", value: "{mobilisation}",
                    oninput: move |e| mobilisation.set(e.value().parse().unwrap_or(10000)),
                    style: "width: 100%; margin-top: 4px;"
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Success Rate: {success_rate}" }
                input {
                    r#type: "range", min: "0", max: "100", value: "{success_rate}",
                    oninput: move |e| success_rate.set(e.value().parse().unwrap_or(50)),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{movement_type} | {tactic} | Mobilisation: {mobilisation} | Success: {success_rate}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → social activism engine | mobilisation sieve | power anchor" }
            }
        }
    }
}
