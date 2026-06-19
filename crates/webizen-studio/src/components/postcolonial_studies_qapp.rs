use dioxus::prelude::*;

#[component]
pub fn PostcolonialStudiesQapp() -> Element {
    let mut theoretical_tradition = use_signal(|| "Said Orientalism".to_string());
    let mut colonial_empire = use_signal(|| "British".to_string());
    let mut disciplinary_lens = use_signal(|| "Literature".to_string());
    let mut time_frame = use_signal(|| "Neo-Colonial".to_string());
    let mut notes = use_signal(|| String::new());

    let traditions = [
        "Said Orientalism",
        "Spivak Subaltern",
        "Bhabha Hybridity",
        "Fanon",
        "Négritude",
        "Decolonial Turn",
        "Settler Colonialism",
        "World-Systems",
    ];
    let empires = [
        "British",
        "French",
        "Portuguese",
        "Spanish",
        "Dutch",
        "Belgian",
        "German",
        "Ottoman",
        "Japanese",
        "American",
        "Soviet",
    ];
    let lenses = [
        "Literature",
        "History",
        "Anthropology",
        "Political Economy",
        "Legal",
        "Film",
        "Architecture",
    ];
    let frames = [
        "High Colonialism",
        "Late Colonial",
        "Independence",
        "Neo-Colonial",
        "Contemporary",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Postcolonial Studies" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Theoretical Tradition" }
                    select {
                        value: "{theoretical_tradition}",
                        onchange: move |e| theoretical_tradition.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in traditions { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Colonial Empire" }
                    select {
                        value: "{colonial_empire}",
                        onchange: move |e| colonial_empire.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in empires { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Disciplinary Lens" }
                    select {
                        value: "{disciplinary_lens}",
                        onchange: move |e| disciplinary_lens.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in lenses { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Time Frame" }
                    select {
                        value: "{time_frame}",
                        onchange: move |e| time_frame.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in frames { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                style: "flex: 1; display: flex; flex-direction: column;",
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Notes" }
                textarea {
                    value: "{notes}",
                    oninput: move |e| notes.set(e.value()),
                    style: "flex: 1; width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; resize: none; box-sizing: border-box; min-height: 100px;"
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 12px 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent);",
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{theoretical_tradition} | {colonial_empire} | {disciplinary_lens} | {time_frame}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → subaltern sieve | hybridity graph | decolonial theory engine" }
            }
        }
    }
}
