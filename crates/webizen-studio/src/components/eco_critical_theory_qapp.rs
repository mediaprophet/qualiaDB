use dioxus::prelude::*;

#[component]
pub fn EcoCriticalTheoryQapp() -> Element {
    let mut ecocritical_wave = use_signal(|| "First Wave Nature Writing".to_string());
    let mut theoretical_lens = use_signal(|| "Deep Ecology".to_string());
    let mut literary_form = use_signal(|| "Novel".to_string());
    let mut anthropocentrism_index = use_signal(|| 50u32);
    let mut nature_culture_dissolution = use_signal(|| 50u32);
    let mut notes = use_signal(|| String::new());

    let ecocritical_waves = [
        "First Wave Nature Writing",
        "Second Wave Environmental Justice",
        "Third Wave Material/Posthuman",
        "Fourth Wave Climate Fiction",
    ];
    let theoretical_lenses = [
        "Deep Ecology",
        "Social Ecology",
        "Ecofeminism",
        "New Materialism",
        "Multispecies",
    ];
    let literary_forms = [
        "Novel",
        "Poetry",
        "Film",
        "Science Writing",
        "Graphic Novel",
        "Game",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Eco-Critical Theory" }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Ecocritical Wave" }
                select {
                    value: "{ecocritical_wave}", onchange: move |e| ecocritical_wave.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in ecocritical_waves { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Theoretical Lens" }
                select {
                    value: "{theoretical_lens}", onchange: move |e| theoretical_lens.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in theoretical_lenses { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Literary/Cultural Form" }
                select {
                    value: "{literary_form}", onchange: move |e| literary_form.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in literary_forms { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Anthropocentrism Index: {anthropocentrism_index}" }
                input { r#type: "range", min: "0", max: "100", value: "{anthropocentrism_index}",
                    oninput: move |e| anthropocentrism_index.set(e.value().parse().unwrap_or(50)),
                    style: "width: 100%; margin-top: 4px;" }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Nature-Culture Binary Dissolution: {nature_culture_dissolution}" }
                input { r#type: "range", min: "0", max: "100", value: "{nature_culture_dissolution}",
                    oninput: move |e| nature_culture_dissolution.set(e.value().parse().unwrap_or(50)),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{ecocritical_wave} | {theoretical_lens} | {literary_form} | anthropo: {anthropocentrism_index}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → eco-critical theory engine | discourse sieve | anchor" }
            }
        }
    }
}
