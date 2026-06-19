use dioxus::prelude::*;

#[component]
pub fn VisualAndCriticalStudiesQapp() -> Element {
    let mut theoretical_lens = use_signal(|| "Visual Culture Studies".to_string());
    let mut medium = use_signal(|| "Photography".to_string());
    let mut iconographic_method = use_signal(|| "Panofsky Iconology".to_string());
    let mut gaze_type = use_signal(|| "Male Gaze".to_string());
    let mut image_context = use_signal(|| String::new());
    let mut notes = use_signal(|| String::new());

    let lenses = [
        "Visual Culture Studies",
        "Semiotics (Barthes)",
        "Phenomenology of Vision",
        "Feminist Gaze Theory",
        "Postcolonial Visuality",
        "Affect Theory",
        "New Materialism",
        "Political Economy of Images",
    ];
    let media = [
        "Photography",
        "Film",
        "Painting",
        "Digital",
        "Installation",
        "Architecture",
        "Advertising",
        "Social Media Image",
        "Surveillance",
    ];
    let methods = [
        "Panofsky Iconology",
        "Barthes Mythologies",
        "Rose Visual Methodology",
        "Rose Social Semiotics",
    ];
    let gazes = [
        "Male Gaze",
        "Colonial Gaze",
        "Panopticon",
        "Counter-Gaze",
        "Algorithmic Vision",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Visual & Critical Studies" }

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
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Medium" }
                    select {
                        value: "{medium}",
                        onchange: move |e| medium.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in media { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Iconographic Method" }
                    select {
                        value: "{iconographic_method}",
                        onchange: move |e| iconographic_method.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in methods { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Gaze Type" }
                    select {
                        value: "{gaze_type}",
                        onchange: move |e| gaze_type.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in gazes { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Image Context" }
                input {
                    r#type: "text",
                    value: "{image_context}",
                    oninput: move |e| image_context.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{theoretical_lens} | {medium} | {iconographic_method} | {gaze_type}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → iconographic sieve | gaze theory engine | visual semiotics graph" }
            }
        }
    }
}
