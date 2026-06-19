use dioxus::prelude::*;

#[component]
pub fn CybercultureStudiesQapp() -> Element {
    let mut phenomenon = use_signal(|| "Online Community".to_string());
    let mut theoretical_lens = use_signal(|| "Actor-Network Theory".to_string());
    let mut platform_type = use_signal(|| "Social Media".to_string());
    let mut participation_mode = use_signal(|| "Active Contributor".to_string());
    let mut toxicity_index = use_signal(|| 20u32);
    let mut notes = use_signal(|| String::new());

    let phenomena = [
        "Online Community",
        "Meme Culture",
        "Gaming Culture",
        "Hacktivism",
        "Digital Identity",
        "Datafication",
        "AI Society",
    ];
    let theoretical_lenses = [
        "Actor-Network Theory",
        "Posthumanism",
        "Surveillance Studies",
        "Platform Studies",
        "Feminist Technoscience",
        "Critical Data Studies",
    ];
    let platform_types = [
        "Social Media",
        "Forum",
        "Gaming Platform",
        "Dark Web",
        "Metaverse",
        "Messaging App",
    ];
    let participation_modes = [
        "Lurker",
        "Active Contributor",
        "Creator",
        "Moderator",
        "Troll",
        "Bot",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",

            h2 {
                style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;",
                "Cyberculture Studies"
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Phenomenon" }
                select {
                    value: "{phenomenon}",
                    onchange: move |e| phenomenon.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in phenomena { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Theoretical Lens" }
                select {
                    value: "{theoretical_lens}",
                    onchange: move |e| theoretical_lens.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in theoretical_lenses { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Platform Type" }
                select {
                    value: "{platform_type}",
                    onchange: move |e| platform_type.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in platform_types { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Participation Mode" }
                select {
                    value: "{participation_mode}",
                    onchange: move |e| participation_mode.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in participation_modes { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Toxicity Index: {toxicity_index}" }
                input {
                    r#type: "range", min: "0", max: "100",
                    value: "{toxicity_index}",
                    oninput: move |e| toxicity_index.set(e.value().parse().unwrap_or(20)),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{phenomenon} | {theoretical_lens} | {platform_type} | {participation_mode} | tox {toxicity_index}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → knowledge engine | sieve | anchor" }
            }
        }
    }
}
