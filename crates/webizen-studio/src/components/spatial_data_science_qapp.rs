use dioxus::prelude::*;

#[component]
pub fn SpatialDataScienceQapp() -> Element {
    let mut analysis_type = use_signal(|| "Spatial Clustering".to_string());
    let mut data_source = use_signal(|| "Satellite".to_string());
    let mut coordinate_system = use_signal(|| "WGS84".to_string());
    let mut resolution = use_signal(|| "1–10m".to_string());
    let mut accuracy_m = use_signal(|| 5.0f64);
    let mut notes = use_signal(|| String::new());

    let analysis_types = [
        "Spatial Clustering",
        "Hot Spot Analysis",
        "Network Analysis",
        "Land Use Classification",
        "Remote Sensing",
        "Geocoding",
        "Geostatistics",
    ];
    let data_sources = [
        "Satellite",
        "LiDAR",
        "Census",
        "GPS Track",
        "Social Media",
        "Sensor Network",
    ];
    let coordinate_systems = ["WGS84", "UTM", "Local Grid", "Web Mercator"];
    let resolutions = ["<1m", "1–10m", "10–100m", "100m–1km", ">1km"];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",

            h2 {
                style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;",
                "Spatial Data Science"
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Analysis Type" }
                select {
                    value: "{analysis_type}",
                    onchange: move |e| analysis_type.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in analysis_types { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Data Source" }
                select {
                    value: "{data_source}",
                    onchange: move |e| data_source.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in data_sources { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Coordinate System" }
                select {
                    value: "{coordinate_system}",
                    onchange: move |e| coordinate_system.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in coordinate_systems { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Resolution" }
                select {
                    value: "{resolution}",
                    onchange: move |e| resolution.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in resolutions { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Accuracy (m): {accuracy_m():.2}" }
                input {
                    r#type: "range", min: "0", max: "100",
                    value: "{accuracy_m()}",
                    oninput: move |e| accuracy_m.set(e.value().parse::<f64>().unwrap_or(5.0)),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{analysis_type} | {data_source} | {coordinate_system} | {resolution} | acc {accuracy_m():.2}m" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → knowledge engine | sieve | anchor" }
            }
        }
    }
}
