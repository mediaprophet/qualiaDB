use dioxus::prelude::*;

#[component]
pub fn SiteSpecificityTheoryQapp() -> Element {
    let mut site_type = use_signal(|| "Physical".to_string());
    let mut art_modality = use_signal(|| "Installation".to_string());
    let mut theoretical_lens = use_signal(|| "Miwon Kwon".to_string());
    let mut ephemerality = use_signal(|| "Temporary".to_string());
    let mut community_engagement = use_signal(|| 50u32);
    let mut notes = use_signal(|| String::new());

    let site_types = [
        "Physical",
        "Institutional",
        "Discursive",
        "Virtual",
        "Body as Site",
        "Community",
    ];
    let modalities = [
        "Installation",
        "Performance",
        "Land Art",
        "Public Art",
        "Relational Aesthetics",
        "Intervention",
    ];
    let lenses = [
        "Miwon Kwon",
        "Kaye",
        "Pearson",
        "Site-Specific Theatre",
        "Urban Art",
    ];
    let ephemeralities = [
        "Permanent",
        "Temporary",
        "Duration-Based",
        "Documentation Only",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Site Specificity Theory" }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Site Type" }
                select {
                    value: "{site_type}",
                    onchange: move |e| site_type.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in site_types { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Art Modality" }
                select {
                    value: "{art_modality}",
                    onchange: move |e| art_modality.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in modalities { option { value: "{x}", "{x}" } }
                }
            }
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
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Ephemerality" }
                select {
                    value: "{ephemerality}",
                    onchange: move |e| ephemerality.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in ephemeralities { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Community Engagement: {community_engagement}" }
                input {
                    r#type: "range", min: "0", max: "100", value: "{community_engagement}",
                    oninput: move |e| community_engagement.set(e.value().parse().unwrap_or(50)),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{site_type} | {art_modality} | {ephemerality} | Engagement: {community_engagement}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → site specificity engine | place sieve | engagement anchor" }
            }
        }
    }
}
