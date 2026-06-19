use dioxus::prelude::*;

#[component]
pub fn EcoFeminismQapp() -> Element {
    let mut strand = use_signal(|| "Classical/Liberal".to_string());
    let mut theorist = use_signal(|| "Warren".to_string());
    let mut domination_logic = use_signal(|| "Nature/Culture".to_string());
    let mut praxis = use_signal(|| "Direct Action".to_string());
    let mut intersectionality_index = use_signal(|| 50u32);
    let mut notes = use_signal(|| String::new());

    let strands = [
        "Classical/Liberal",
        "Radical/Cultural",
        "Socialist",
        "Social",
        "Spiritual",
        "Indigenous",
        "Intersectional",
    ];
    let theorists = [
        "Warren", "Plumwood", "Griffin", "Mies", "Shiva", "Gaard", "Haraway",
    ];
    let domination_logics = [
        "Nature/Culture",
        "Women/Men",
        "Colonised/Coloniser",
        "Body/Mind",
    ];
    let praxes = [
        "Direct Action",
        "Legal Advocacy",
        "Alternative Economy",
        "Cultural Production",
        "Research",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Eco-Feminism" }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Strand" }
                select {
                    value: "{strand}", onchange: move |e| strand.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in strands { option { value: "{x}", "{x}" } }
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
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Domination Logic" }
                select {
                    value: "{domination_logic}", onchange: move |e| domination_logic.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in domination_logics { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Praxis" }
                select {
                    value: "{praxis}", onchange: move |e| praxis.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in praxes { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Intersectionality Index: {intersectionality_index}" }
                input { r#type: "range", min: "0", max: "100", value: "{intersectionality_index}",
                    oninput: move |e| intersectionality_index.set(e.value().parse().unwrap_or(50)),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{strand} | {theorist} | {domination_logic} | {praxis} | intersect: {intersectionality_index}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → eco-feminism engine | discourse sieve | anchor" }
            }
        }
    }
}
