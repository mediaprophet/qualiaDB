use dioxus::prelude::*;

#[component]
pub fn PrintmakingQapp() -> Element {
    let mut technique = use_signal(|| "Etching".to_string());
    let mut ink_type = use_signal(|| "Oil-Based".to_string());
    let mut edition_size = use_signal(|| 25u32);
    let mut colour_separation = use_signal(|| 1u32);
    let mut matrix_material = use_signal(|| "Copper".to_string());
    let mut registration_method = use_signal(|| "Pin Registration".to_string());
    let mut notes = use_signal(|| String::new());

    let techniques = [
        "Etching",
        "Aquatint",
        "Drypoint",
        "Mezzotint",
        "Woodcut",
        "Linocut",
        "Lithography",
        "Screen Print",
        "Monotype",
        "Collagraph",
        "Digital Print",
    ];
    let ink_types = [
        "Oil-Based",
        "Water-Based",
        "UV",
        "Metallic",
        "Transparent",
        "Chine-Collé",
    ];
    let matrices = [
        "Copper",
        "Zinc",
        "Aluminium",
        "Wood",
        "Linoleum",
        "Limestone",
        "Photo-Polymer",
        "Screen",
    ];
    let reg_methods = ["Pin Registration", "Visual", "Digital"];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Printmaking" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Technique" }
                    select {
                        value: "{technique}",
                        onchange: move |e| technique.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in techniques { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Ink Type" }
                    select {
                        value: "{ink_type}",
                        onchange: move |e| ink_type.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in ink_types { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Matrix Material" }
                    select {
                        value: "{matrix_material}",
                        onchange: move |e| matrix_material.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in matrices { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Registration Method" }
                    select {
                        value: "{registration_method}",
                        onchange: move |e| registration_method.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in reg_methods { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Edition Size: {edition_size}" }
                input {
                    r#type: "range",
                    min: "1",
                    max: "500",
                    value: "{edition_size}",
                    oninput: move |e| edition_size.set(e.value().parse().unwrap_or(25)),
                    style: "width: 100%; margin-top: 4px;"
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Colour Separations: {colour_separation}" }
                input {
                    r#type: "range",
                    min: "1",
                    max: "8",
                    value: "{colour_separation}",
                    oninput: move |e| colour_separation.set(e.value().parse().unwrap_or(1)),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{technique} | {matrix_material} | {ink_type} | Ed.{edition_size} | {colour_separation}-colour" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → printmaking engine | intaglio/relief sieve | edition anchor" }
            }
        }
    }
}
