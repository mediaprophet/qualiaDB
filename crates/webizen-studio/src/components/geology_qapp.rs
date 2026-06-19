use dioxus::prelude::*;

#[component]
pub fn GeologyQapp() -> Element {
    let mut geological_period = use_signal(|| "Cretaceous".to_string());
    let mut rock_class = use_signal(|| "Igneous Intrusive".to_string());
    let mut mineral_hardness_mohs = use_signal(|| 5u32);
    let mut formation_process = use_signal(|| "Magmatic crystallisation".to_string());
    let mut dip_angle = use_signal(|| 30u32);
    let mut strike = use_signal(|| "N45E".to_string());
    let mut field_notes = use_signal(|| String::new());

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Geology QApp" }

            div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Geological Period" }
                    select {
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        onchange: move |e| geological_period.set(e.value()),
                        option { "Hadean" }
                        option { "Archean" }
                        option { "Proterozoic" }
                        option { "Cambrian" }
                        option { "Ordovician" }
                        option { "Silurian" }
                        option { "Devonian" }
                        option { "Carboniferous" }
                        option { "Permian" }
                        option { "Triassic" }
                        option { "Jurassic" }
                        option { "Cretaceous" }
                        option { "Paleogene" }
                        option { "Neogene" }
                        option { "Quaternary" }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Rock Class" }
                    select {
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        onchange: move |e| rock_class.set(e.value()),
                        option { "Igneous Intrusive" }
                        option { "Igneous Extrusive" }
                        option { "Clastic Sedimentary" }
                        option { "Chemical Sedimentary" }
                        option { "Organic Sedimentary" }
                        option { "Regional Metamorphic" }
                        option { "Contact Metamorphic" }
                        option { "Hydrothermal" }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Mineral Hardness (Mohs): {mineral_hardness_mohs}" }
                    input {
                        r#type: "range",
                        min: "1",
                        max: "10",
                        step: "1",
                        value: "{mineral_hardness_mohs}",
                        style: "width: 100%; margin-top: 8px;",
                        oninput: move |e| mineral_hardness_mohs.set(e.value().parse().unwrap_or(5)),
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Dip Angle (°): {dip_angle}" }
                    input {
                        r#type: "range",
                        min: "0",
                        max: "90",
                        step: "1",
                        value: "{dip_angle}",
                        style: "width: 100%; margin-top: 8px;",
                        oninput: move |e| dip_angle.set(e.value().parse().unwrap_or(30)),
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Formation Process" }
                    input {
                        r#type: "text",
                        value: "{formation_process}",
                        placeholder: "e.g. Magmatic crystallisation, Compaction...",
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        oninput: move |e| formation_process.set(e.value()),
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Strike" }
                    input {
                        r#type: "text",
                        value: "{strike}",
                        placeholder: "e.g. N45°E, 045°...",
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        oninput: move |e| strike.set(e.value()),
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Field Notes" }
                textarea {
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; min-height: 60px; box-sizing: border-box; resize: vertical;",
                    placeholder: "Sample coordinates, mineral composition, alteration, structural context...",
                    oninput: move |e| field_notes.set(e.value()),
                    "{field_notes}"
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent); flex: 1;",
                h3 { style: "margin-top: 0; color: var(--qualia-accent); font-size: 0.9rem;", "Output / Analysis" }
                div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 8px; font-size: 0.8rem;",
                    div { style: "color: var(--qualia-text-muted);", "Period:" }
                    div { style: "color: var(--qualia-text);", "{geological_period}" }
                    div { style: "color: var(--qualia-text-muted);", "Rock Class:" }
                    div { style: "color: var(--qualia-text);", "{rock_class}" }
                    div { style: "color: var(--qualia-text-muted);", "Hardness (Mohs):" }
                    div { style: "color: var(--qualia-text);", "{mineral_hardness_mohs}" }
                    div { style: "color: var(--qualia-text-muted);", "Dip / Strike:" }
                    div { style: "color: var(--qualia-text);", "{dip_angle}° / {strike}" }
                    div { style: "color: var(--qualia-text-muted);", "Formation:" }
                    div { style: "color: var(--qualia-text);", "{formation_process}" }
                }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 12px; border-top: 1px solid var(--qualia-border); padding-top: 8px;",
                    "QualiaDB → Allen Interval temporal | stratigraphic graph | geochemistry sieve"
                }
            }
        }
    }
}
