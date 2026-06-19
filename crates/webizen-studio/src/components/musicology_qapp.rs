use dioxus::prelude::*;

#[component]
pub fn MusicologyQapp() -> Element {
    let mut subfield = use_signal(|| "Historical Musicology".to_string());
    let mut theoretical_approach = use_signal(|| "Hermeneutics".to_string());
    let mut repertoire = use_signal(|| "Baroque".to_string());
    let mut analytical_method = use_signal(|| "Style Analysis".to_string());
    let mut composer_or_work = use_signal(|| String::new());
    let mut notes = use_signal(|| String::new());

    let subfields = [
        "Historical Musicology",
        "Systematic Musicology",
        "Cognitive Musicology",
        "Critical Musicology",
        "Computational Musicology",
    ];
    let approaches = [
        "Formalism",
        "Hermeneutics",
        "New Musicology",
        "Feminist",
        "Postcolonial",
        "Empirical",
    ];
    let repertoires = [
        "Medieval",
        "Renaissance",
        "Baroque",
        "Classical",
        "Romantic",
        "20th C. Art Music",
        "Jazz",
        "Popular",
        "World Music",
        "Electronic",
    ];
    let methods = [
        "Schenkerian",
        "Set Theory",
        "Neo-Riemannian",
        "Topic Theory",
        "Style Analysis",
        "Corpus",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Musicology" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Subfield" }
                    select {
                        value: "{subfield}",
                        onchange: move |e| subfield.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in subfields { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Theoretical Approach" }
                    select {
                        value: "{theoretical_approach}",
                        onchange: move |e| theoretical_approach.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in approaches { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Repertoire" }
                    select {
                        value: "{repertoire}",
                        onchange: move |e| repertoire.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in repertoires { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Analytical Method" }
                    select {
                        value: "{analytical_method}",
                        onchange: move |e| analytical_method.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in methods { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Composer or Work" }
                input {
                    r#type: "text",
                    value: "{composer_or_work}",
                    oninput: move |e| composer_or_work.set(e.value()),
                    placeholder: "e.g. Bach WTC, Beethoven Op.131, Coltrane A Love Supreme",
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{subfield} | {theoretical_approach} | {repertoire} | {analytical_method}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → music corpus engine | harmonic analysis sieve | score anchor" }
            }
        }
    }
}
