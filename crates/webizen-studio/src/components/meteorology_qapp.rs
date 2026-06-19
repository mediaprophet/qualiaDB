use dioxus::prelude::*;

#[component]
pub fn MeteorologyQapp() -> Element {
    let mut weather_system = use_signal(|| "Cyclone".to_string());
    let mut forecast_method = use_signal(|| "Numerical Weather Prediction".to_string());
    let mut temperature_c = use_signal(|| 20.0f64);
    let mut pressure_hpa = use_signal(|| 1013.25f64);
    let mut wind_speed_kph = use_signal(|| 30.0f64);
    let mut precipitation_mm = use_signal(|| 10.0f64);
    let visibility_km = use_signal(|| 10.0f64);
    let mut notes = use_signal(|| String::new());

    let weather_systems = [
        "Anticyclone",
        "Cyclone",
        "Cold Front",
        "Warm Front",
        "Occluded Front",
        "Stationary Front",
        "Squall Line",
        "MCS",
        "Supercell",
        "Thunderstorm",
    ];
    let forecast_methods = [
        "Numerical Weather Prediction",
        "Statistical",
        "Ensemble",
        "Analogue",
        "AI",
        "Nowcasting",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Meteorology" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Weather System" }
                    select {
                        value: "{weather_system}",
                        onchange: move |e| weather_system.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in weather_systems { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Forecast Method" }
                    select {
                        value: "{forecast_method}",
                        onchange: move |e| forecast_method.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in forecast_methods { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Temperature (°C): {temperature_c:.1}" }
                    input {
                        r#type: "range",
                        min: "-40",
                        max: "50",
                        step: "1",
                        value: "{temperature_c()}",
                        oninput: move |e| temperature_c.set(e.value().parse().unwrap_or(20.0)),
                        style: "width: 100%; margin-top: 4px;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Pressure (hPa): {pressure_hpa:.0}" }
                    input {
                        r#type: "range",
                        min: "950",
                        max: "1050",
                        step: "1",
                        value: "{pressure_hpa()}",
                        oninput: move |e| pressure_hpa.set(e.value().parse().unwrap_or(1013.0)),
                        style: "width: 100%; margin-top: 4px;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Wind Speed (kph): {wind_speed_kph:.0}" }
                    input {
                        r#type: "range",
                        min: "0",
                        max: "300",
                        step: "5",
                        value: "{wind_speed_kph()}",
                        oninput: move |e| wind_speed_kph.set(e.value().parse().unwrap_or(30.0)),
                        style: "width: 100%; margin-top: 4px;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Precipitation (mm): {precipitation_mm:.0}" }
                    input {
                        r#type: "range",
                        min: "0",
                        max: "500",
                        step: "5",
                        value: "{precipitation_mm()}",
                        oninput: move |e| precipitation_mm.set(e.value().parse().unwrap_or(10.0)),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{weather_system} | {forecast_method} | {temperature_c:.0}°C | {pressure_hpa:.0}hPa | {wind_speed_kph:.0}kph | Vis:{visibility_km:.0}km" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → NWP engine | synoptic sieve | mesoscale anchor" }
            }
        }
    }
}
