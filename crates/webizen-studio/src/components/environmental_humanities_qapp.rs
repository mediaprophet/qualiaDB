use dioxus::prelude::*;

#[component]
pub fn EnvironmentalHumanitiesQapp() -> Element {
    let mut theoretical_lens = use_signal(|| "Ecocriticism".to_string());
    let mut literary_mode = use_signal(|| "Nature Writing".to_string());
    let mut geographic_scale = use_signal(|| "Global".to_string());
    let mut temporal_frame = use_signal(|| "Anthropocene".to_string());
    let mut medium = use_signal(|| "Literature".to_string());
    let mut notes = use_signal(|| String::new());

    let lenses = [
        "Ecocriticism",
        "Multispecies Studies",
        "New Materialism",
        "Political Ecology",
        "Environmental Justice",
        "Solastalgia",
        "Cli-fi",
        "Indigenous Ecological Knowledge",
    ];
    let modes = [
        "Pastoral",
        "Nature Writing",
        "Cli-Fi",
        "Eco-Gothic",
        "Solarpunk",
        "Documentary",
    ];
    let scales = ["Local", "Regional", "National", "Global", "Planetary"];
    let frames = [
        "Holocene",
        "Anthropocene",
        "Capitalocene",
        "Plantationocene",
        "Deep Time",
    ];
    let media = ["Literature", "Film", "Visual Art", "Music", "Architecture"];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Environmental Humanities" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Theoretical Lens" }
                    select {
                        value: "{theoretical_lens}",
                        onchange: move |e| theoretical_lens.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in lenses { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Literary Mode" }
                    select {
                        value: "{literary_mode}",
                        onchange: move |e| literary_mode.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in modes { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Geographic Scale" }
                    select {
                        value: "{geographic_scale}",
                        onchange: move |e| geographic_scale.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in scales { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Temporal Frame" }
                    select {
                        value: "{temporal_frame}",
                        onchange: move |e| temporal_frame.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in frames { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Medium" }
                    select {
                        value: "{medium}",
                        onchange: move |e| medium.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in media { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                style: "flex: 1; display: flex; flex-direction: column;",
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Notes" }
                textarea {
                    value: "{notes}",
                    oninput: move |e| notes.set(e.value()),
                    style: "flex: 1; width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; resize: none; box-sizing: border-box; min-height: 80px;"
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 12px 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent);",
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{theoretical_lens} | {temporal_frame} | {geographic_scale} | {medium}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → ecological sieve | multispecies graph | deep time engine" }
            }
        }
    }
}
