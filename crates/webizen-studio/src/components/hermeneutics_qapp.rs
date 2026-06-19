use dioxus::prelude::*;

#[component]
pub fn HermeneuticsQapp() -> Element {
    let mut hermeneutic_tradition = use_signal(|| "Gadamer".to_string());
    let mut interpretive_mode = use_signal(|| "Grammatical".to_string());
    let mut text_type = use_signal(|| "Literary".to_string());
    let mut horizon = use_signal(|| "Fusion".to_string());
    let mut understanding_depth = use_signal(|| 50u32);
    let mut notes = use_signal(|| String::new());

    let hermeneutic_traditions = [
        "Schleiermacher",
        "Dilthey",
        "Heidegger",
        "Gadamer",
        "Ricoeur",
        "Betti",
        "Derrida",
    ];
    let interpretive_modes = [
        "Grammatical",
        "Psychological",
        "Historical",
        "Phenomenological",
        "Deconstructive",
        "Dialogical",
    ];
    let text_types = [
        "Sacred",
        "Literary",
        "Legal",
        "Historical",
        "Social Scientific",
        "Everyday",
    ];
    let horizons = [
        "Author's Intent",
        "Text's World",
        "Reader's Horizon",
        "Fusion",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Hermeneutics" }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Hermeneutic Tradition" }
                select {
                    value: "{hermeneutic_tradition}", onchange: move |e| hermeneutic_tradition.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in hermeneutic_traditions { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Interpretive Mode" }
                select {
                    value: "{interpretive_mode}", onchange: move |e| interpretive_mode.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in interpretive_modes { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Text Type" }
                select {
                    value: "{text_type}", onchange: move |e| text_type.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in text_types { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Horizon" }
                select {
                    value: "{horizon}", onchange: move |e| horizon.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in horizons { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Understanding Depth: {understanding_depth}" }
                input { r#type: "range", min: "0", max: "100", value: "{understanding_depth}",
                    oninput: move |e| understanding_depth.set(e.value().parse().unwrap_or(50)),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{hermeneutic_tradition} | {interpretive_mode} | {text_type} | {horizon} | depth: {understanding_depth}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → hermeneutics engine | discourse sieve | anchor" }
            }
        }
    }
}
