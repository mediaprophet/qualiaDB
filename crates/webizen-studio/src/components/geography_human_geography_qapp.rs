use dioxus::prelude::*;

#[component]
pub fn GeographyHumanGeographyQapp() -> Element {
    let mut subfield = use_signal(|| "Urban Geography".to_string());
    let mut scale = use_signal(|| "City / Metropolitan".to_string());
    let mut latitude = use_signal(|| 0.0f64);
    let mut longitude = use_signal(|| 0.0f64);
    let mut spatial_method = use_signal(|| "GIS / Spatial Analysis".to_string());
    let mut theoretical_lens = use_signal(|| "Political Economy of Space".to_string());
    let mut place_name = use_signal(|| String::new());
    let mut research_notes = use_signal(|| String::new());

    let subfields = [
        "Urban Geography",
        "Rural Geography",
        "Economic Geography",
        "Political Geography",
        "Cultural Geography",
        "Historical Geography",
        "Population / Demographic Geography",
        "Environmental Geography",
        "Transport Geography",
        "Health Geography",
        "Tourism Geography",
        "Feminist / Gender Geography",
        "Postcolonial Geography",
    ];
    let scales = [
        "Global",
        "Continental / Regional",
        "National",
        "City / Metropolitan",
        "Neighbourhood",
        "Local / Street-Level",
        "Body / Micro-Scale",
    ];
    let methods = [
        "GIS / Spatial Analysis",
        "Remote Sensing",
        "Ethnography / Fieldwork",
        "Survey",
        "Census Analysis",
        "Participatory Mapping",
        "Historical Cartography",
        "Network Analysis",
        "Agent-Based Modelling",
    ];
    let lenses = [
        "Political Economy of Space (Harvey)",
        "Social Construction of Space (Lefebvre)",
        "Feminist Geography",
        "Postcolonial Geography",
        "Non-Representational Theory",
        "Critical Physical Geography",
        "Mobilities Paradigm",
        "Assemblage Theory",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Human Geography" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Subfield" }
                    select {
                        value: "{subfield}",
                        onchange: move |e| subfield.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for x in subfields { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Spatial Scale" }
                    select {
                        value: "{scale}",
                        onchange: move |e| scale.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for x in scales { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Theoretical Lens" }
                    select {
                        value: "{theoretical_lens}",
                        onchange: move |e| theoretical_lens.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for x in lenses { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Spatial Method" }
                    select {
                        value: "{spatial_method}",
                        onchange: move |e| spatial_method.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for x in methods { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Latitude" }
                    input {
                        r#type: "number", step: "0.0001", min: "-90.0", max: "90.0",
                        value: "{latitude}",
                        oninput: move |e| latitude.set(e.value().parse().unwrap_or(0.0)),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Longitude" }
                    input {
                        r#type: "number", step: "0.0001", min: "-180.0", max: "180.0",
                        value: "{longitude}",
                        oninput: move |e| longitude.set(e.value().parse().unwrap_or(0.0)),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Place Name / Study Area" }
                input {
                    r#type: "text", placeholder: "e.g. Detroit rustbelt, Dharavi Mumbai, Rhine-Ruhr…",
                    value: "{place_name}",
                    oninput: move |e| place_name.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                }
            }

            div {
                style: "flex: 1; display: flex; flex-direction: column;",
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Research Notes" }
                textarea {
                    value: "{research_notes}",
                    oninput: move |e| research_notes.set(e.value()),
                    style: "flex: 1; width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; resize: none; box-sizing: border-box; min-height: 60px;"
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 12px 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent); display: flex; gap: 16px; flex-wrap: wrap;",
                span { style: "font-size: 0.8rem; color: var(--qualia-accent); font-weight: bold;", "{latitude:.4}°, {longitude:.4}°" }
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{subfield}" }
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Scale: {scale}" }
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{spatial_method}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); width: 100%;", "QualiaDB → geospatial engine | Allen Interval | graph theory spatial networks" }
            }
        }
    }
}
