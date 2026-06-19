use dioxus::prelude::*;

#[component]
pub fn AtmosphericScienceQapp() -> Element {
    let mut phenomenon = use_signal(|| "Tropical Cyclone".to_string());
    let mut atmospheric_layer = use_signal(|| "Troposphere".to_string());
    let mut temperature_k = use_signal(|| 288.0f64);
    let mut pressure_hpa = use_signal(|| 1013.25f64);
    let mut humidity_pct = use_signal(|| 60.0f64);
    let mut wind_speed_ms = use_signal(|| 10.0f64);
    let mut co2_ppm = use_signal(|| 420.0f64);
    let mut notes = use_signal(|| String::new());

    let phenomena = [
        "Tropical Cyclone",
        "Jet Stream",
        "ENSO",
        "Monsoon",
        "Arctic Oscillation",
        "Convective Storm",
        "Fog",
        "Atmospheric River",
        "Stratospheric Sudden Warming",
        "Pollution Episode",
    ];
    let layers = [
        "Troposphere",
        "Stratosphere",
        "Mesosphere",
        "Thermosphere",
        "Exosphere",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Atmospheric Science" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Phenomenon" }
                    select {
                        value: "{phenomenon}",
                        onchange: move |e| phenomenon.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in phenomena { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Atmospheric Layer" }
                    select {
                        value: "{atmospheric_layer}",
                        onchange: move |e| atmospheric_layer.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in layers { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Temperature (K): {temperature_k:.1}" }
                    input {
                        r#type: "range",
                        min: "150",
                        max: "330",
                        step: "1",
                        value: "{temperature_k()}",
                        oninput: move |e| temperature_k.set(e.value().parse().unwrap_or(288.0)),
                        style: "width: 100%; margin-top: 4px;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Pressure (hPa): {pressure_hpa:.1}" }
                    input {
                        r#type: "range",
                        min: "800",
                        max: "1060",
                        step: "1",
                        value: "{pressure_hpa()}",
                        oninput: move |e| pressure_hpa.set(e.value().parse().unwrap_or(1013.0)),
                        style: "width: 100%; margin-top: 4px;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Humidity (%): {humidity_pct:.0}" }
                    input {
                        r#type: "range",
                        min: "0",
                        max: "100",
                        step: "1",
                        value: "{humidity_pct()}",
                        oninput: move |e| humidity_pct.set(e.value().parse().unwrap_or(60.0)),
                        style: "width: 100%; margin-top: 4px;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "CO₂ (ppm): {co2_ppm:.1}" }
                    input {
                        r#type: "range",
                        min: "280",
                        max: "600",
                        step: "1",
                        value: "{co2_ppm()}",
                        oninput: move |e| co2_ppm.set(e.value().parse().unwrap_or(420.0)),
                        style: "width: 100%; margin-top: 4px;"
                    }
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{phenomenon} | {atmospheric_layer} | {temperature_k:.0}K | {pressure_hpa:.0}hPa | {humidity_pct:.0}% RH | CO₂:{co2_ppm:.0}ppm" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → atmospheric model engine | meteorological sieve | climate anchor" }
            }
        }
    }
}
