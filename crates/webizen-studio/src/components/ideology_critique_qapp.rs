use dioxus::prelude::*;

#[component]
pub fn IdeologyCritiqueQapp() -> Element {
    let mut ideological_form = use_signal(|| "Naturalization".to_string());
    let mut theorist = use_signal(|| "Althusser".to_string());
    let mut site = use_signal(|| "Media".to_string());
    let mut critique_method = use_signal(|| "Immanent Critique".to_string());
    let mut ideology_density = use_signal(|| 50u32);
    let mut notes = use_signal(|| String::new());

    let ideological_forms = [
        "Naturalization",
        "Universalization",
        "Legitimation",
        "Reification",
        "Hegemony",
        "Interpellation",
        "Spectacle",
    ];
    let theorists = [
        "Althusser",
        "Gramsci",
        "Žižek",
        "Debord",
        "Hall",
        "Eagleton",
        "Butler",
    ];
    let sites = [
        "Media",
        "Education",
        "Law",
        "Religion",
        "Science",
        "Consumer Culture",
    ];
    let critique_methods = [
        "Immanent Critique",
        "Symptomatic Reading",
        "Discourse Analysis",
        "Deconstruction",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Ideology Critique" }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Ideological Form" }
                select {
                    value: "{ideological_form}", onchange: move |e| ideological_form.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in ideological_forms { option { value: "{x}", "{x}" } }
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
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Site" }
                select {
                    value: "{site}", onchange: move |e| site.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in sites { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Critique Method" }
                select {
                    value: "{critique_method}", onchange: move |e| critique_method.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in critique_methods { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Ideology Density: {ideology_density}" }
                input { r#type: "range", min: "0", max: "100", value: "{ideology_density}",
                    oninput: move |e| ideology_density.set(e.value().parse().unwrap_or(50)),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{ideological_form} | {theorist} | {site} | {critique_method} | density: {ideology_density}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → ideology critique engine | discourse sieve | anchor" }
            }
        }
    }
}
