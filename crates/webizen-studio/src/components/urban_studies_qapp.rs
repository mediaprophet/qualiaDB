use dioxus::prelude::*;

#[component]
pub fn UrbanStudiesQapp() -> Element {
    let mut urban_theory = use_signal(|| "Political Economy".to_string());
    let mut city_scale = use_signal(|| "Metropolis".to_string());
    let mut issue_area = use_signal(|| "Housing".to_string());
    let mut density_persons_per_km2 = use_signal(|| 5000u32);
    let mut urban_form = use_signal(|| "Compact".to_string());
    let mut region = use_signal(|| String::new());
    let mut notes = use_signal(|| String::new());

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box; overflow-y: auto;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Urban Studies QApp" }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Urban Theory" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| urban_theory.set(e.value()),
                    option { "Chicago School" }
                    option { selected: true, "Political Economy" }
                    option { "Postmodern Urbanism" }
                    option { "Right to the City" }
                    option { "Smart City" }
                    option { "Planetary Urbanisation" }
                    option { "Feminist Urban Theory" }
                    option { "Decolonial" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "City Scale" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| city_scale.set(e.value()),
                    option { "Megacity" }
                    option { selected: true, "Metropolis" }
                    option { "Medium City" }
                    option { "Small City" }
                    option { "Town" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Issue Area" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| issue_area.set(e.value()),
                    option { selected: true, "Housing" }
                    option { "Transport" }
                    option { "Gentrification" }
                    option { "Segregation" }
                    option { "Informal Settlements" }
                    option { "Green Space" }
                    option { "Water" }
                    option { "Digital Infrastructure" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Density (persons/km²): {density_persons_per_km2()}" }
                input {
                    r#type: "number",
                    min: "0",
                    max: "100000",
                    step: "100",
                    value: "{density_persons_per_km2()}",
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    oninput: move |e| density_persons_per_km2.set(e.value().parse().unwrap_or(5000)),
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Urban Form" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| urban_form.set(e.value()),
                    option { selected: true, "Compact" }
                    option { "Sprawl" }
                    option { "Polycentric" }
                    option { "Linear" }
                    option { "Radial" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Region" }
                input {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    placeholder: "e.g. Sub-Saharan Africa, East Asia, Western Europe...",
                    oninput: move |e| region.set(e.value()),
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Notes" }
                textarea {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box; min-height: 60px; resize: vertical;",
                    placeholder: "Additional notes...",
                    oninput: move |e| notes.set(e.value()),
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent); flex: 1;",
                h3 { style: "margin-top: 0; color: var(--qualia-accent); font-size: 0.9rem;", "QualiaDB Output" }
                div { style: "font-size: 0.8rem; color: var(--qualia-text-muted); display: flex; flex-direction: column; gap: 4px;",
                    div { "Theory: {urban_theory()}" }
                    div { "Scale: {city_scale()}" }
                    div { "Issue: {issue_area()}" }
                    div { "Density: {density_persons_per_km2()} p/km²" }
                    div { "Form: {urban_form()}" }
                }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 8px;", "QualiaDB → geospatial | graph theory | Allen Interval" }
            }
        }
    }
}
