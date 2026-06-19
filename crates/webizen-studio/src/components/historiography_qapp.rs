use dioxus::prelude::*;

#[component]
pub fn HistoriographyQapp() -> Element {
    let mut historiographic_school = use_signal(|| "Annales".to_string());
    let mut methodology = use_signal(|| "Primary Source".to_string());
    let mut temporal_focus = use_signal(|| "Modern".to_string());
    let mut geographic_scope = use_signal(|| "National".to_string());
    let mut theoretical_influence = use_signal(|| 50u32);
    let mut notes = use_signal(|| String::new());

    let historiographic_schools = [
        "Rankean",
        "Annales",
        "Marxist",
        "Social History",
        "Cultural Turn",
        "Postcolonial",
        "Digital History",
        "Microhistory",
    ];
    let methodologies = [
        "Primary Source",
        "Oral History",
        "Quantitative",
        "Comparative",
        "Global",
        "Transnational",
    ];
    let temporal_foci = [
        "Ancient",
        "Medieval",
        "Early Modern",
        "Modern",
        "Contemporary",
    ];
    let geographic_scopes = ["Local", "National", "Regional", "Global", "Transnational"];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Historiography" }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Historiographic School" }
                select {
                    value: "{historiographic_school}", onchange: move |e| historiographic_school.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in historiographic_schools { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Methodology" }
                select {
                    value: "{methodology}", onchange: move |e| methodology.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in methodologies { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Temporal Focus" }
                select {
                    value: "{temporal_focus}", onchange: move |e| temporal_focus.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in temporal_foci { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Geographic Scope" }
                select {
                    value: "{geographic_scope}", onchange: move |e| geographic_scope.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in geographic_scopes { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Theoretical Influence: {theoretical_influence}" }
                input { r#type: "range", min: "0", max: "100", value: "{theoretical_influence}",
                    oninput: move |e| theoretical_influence.set(e.value().parse().unwrap_or(50)),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{historiographic_school} | {methodology} | {temporal_focus} | {geographic_scope} | influence: {theoretical_influence}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → historiography engine | discourse sieve | anchor" }
            }
        }
    }
}
