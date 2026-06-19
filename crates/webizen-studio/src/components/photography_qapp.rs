use dioxus::prelude::*;

#[component]
pub fn PhotographyQapp() -> Element {
    let mut genre = use_signal(|| "Documentary".to_string());
    let mut process = use_signal(|| "Digital".to_string());
    let mut exposure_ev = use_signal(|| 0i32);
    let mut focal_length_mm = use_signal(|| 50u32);
    let mut depth_of_field = use_signal(|| "Medium".to_string());
    let mut post_processing = use_signal(|| "Minimal".to_string());
    let mut notes = use_signal(|| String::new());

    let genres = [
        "Documentary",
        "Fine Art",
        "Portrait",
        "Landscape",
        "Street",
        "Photojournalism",
        "Aerial",
        "Macro",
        "Astrophotography",
        "Fashion",
        "Architecture",
    ];
    let processes = [
        "Digital",
        "Film 35mm",
        "Medium Format",
        "Large Format",
        "Daguerreotype",
        "Cyanotype",
        "Wet Plate Collodion",
        "Pinhole",
        "Darkroom Print",
    ];
    let dof_options = ["Shallow", "Medium", "Deep"];
    let post_options = [
        "Minimal",
        "Colour Grading",
        "Black & White Conversion",
        "Heavy Edit",
        "None",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Photography" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Genre" }
                    select {
                        value: "{genre}",
                        onchange: move |e| genre.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in genres { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Process" }
                    select {
                        value: "{process}",
                        onchange: move |e| process.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in processes { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Depth of Field" }
                    select {
                        value: "{depth_of_field}",
                        onchange: move |e| depth_of_field.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in dof_options { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Post Processing" }
                    select {
                        value: "{post_processing}",
                        onchange: move |e| post_processing.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in post_options { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Exposure EV: {exposure_ev}" }
                input {
                    r#type: "range",
                    min: "-3",
                    max: "3",
                    value: "{exposure_ev}",
                    oninput: move |e| exposure_ev.set(e.value().parse().unwrap_or(0)),
                    style: "width: 100%; margin-top: 4px;"
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Focal Length (mm): {focal_length_mm}" }
                input {
                    r#type: "range",
                    min: "8",
                    max: "600",
                    value: "{focal_length_mm}",
                    oninput: move |e| focal_length_mm.set(e.value().parse().unwrap_or(50)),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{genre} | {process} | {focal_length_mm}mm | EV{exposure_ev} | {depth_of_field} DoF" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → photographic process engine | exposure sieve | aesthetic anchor" }
            }
        }
    }
}
