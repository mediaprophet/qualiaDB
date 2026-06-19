use dioxus::prelude::*;

#[component]
pub fn CinematographyQapp() -> Element {
    let mut camera_format = use_signal(|| "Digital Sensor 4K".to_string());
    let mut lens_type = use_signal(|| "Prime".to_string());
    let mut stop_f = use_signal(|| 2.8f64);
    let mut shutter_angle = use_signal(|| 180u32);
    let mut colour_science = use_signal(|| "Rec.709".to_string());
    let mut lighting_setup = use_signal(|| "Three-Point".to_string());
    let mut notes = use_signal(|| String::new());

    let camera_formats = [
        "35mm Film",
        "16mm Film",
        "8mm",
        "Digital Sensor 4K",
        "Anamorphic",
        "IMAX",
        "VR 360",
    ];
    let lens_types = [
        "Prime",
        "Zoom",
        "Anamorphic",
        "Fisheye",
        "Macro",
        "Tilt-Shift",
    ];
    let colour_sciences = ["Flat Log", "Rec.709", "ACES", "Film Emulation", "HDR"];
    let lighting_setups = [
        "Three-Point",
        "High Key",
        "Low Key",
        "Chiaroscuro",
        "Available Light",
        "Practical Only",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Cinematography" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Camera Format" }
                    select {
                        value: "{camera_format}",
                        onchange: move |e| camera_format.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in camera_formats { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Lens Type" }
                    select {
                        value: "{lens_type}",
                        onchange: move |e| lens_type.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in lens_types { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Colour Science" }
                    select {
                        value: "{colour_science}",
                        onchange: move |e| colour_science.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in colour_sciences { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Lighting Setup" }
                    select {
                        value: "{lighting_setup}",
                        onchange: move |e| lighting_setup.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in lighting_setups { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "f-stop: f/{stop_f:.1}" }
                input {
                    r#type: "range",
                    min: "10",
                    max: "220",
                    step: "1",
                    value: "{stop_f() * 10.0}",
                    oninput: move |e| stop_f.set(e.value().parse::<f64>().unwrap_or(28.0) / 10.0),
                    style: "width: 100%; margin-top: 4px;"
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Shutter Angle: {shutter_angle}°" }
                input {
                    r#type: "range",
                    min: "45",
                    max: "360",
                    value: "{shutter_angle}",
                    oninput: move |e| shutter_angle.set(e.value().parse().unwrap_or(180)),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{camera_format} | {lens_type} | f/{stop_f:.1} | {shutter_angle}° | {colour_science} | {lighting_setup}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → cinematography engine | exposure sieve | colour science anchor" }
            }
        }
    }
}
