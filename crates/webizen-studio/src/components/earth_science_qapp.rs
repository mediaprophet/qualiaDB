use dioxus::prelude::*;

#[component]
pub fn EarthScienceQapp() -> Element {
    let mut subdiscipline = use_signal(|| "Geology".to_string());
    let mut timescale = use_signal(|| "Holocene".to_string());
    let mut rock_type = use_signal(|| "Igneous".to_string());
    let mut tectonic_setting = use_signal(|| "Subduction Zone".to_string());
    let mut feature_notes = use_signal(|| String::new());

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Earth Science QApp" }

            div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Subdiscipline" }
                    select {
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        onchange: move |e| subdiscipline.set(e.value()),
                        option { "Geology" }
                        option { "Geomorphology" }
                        option { "Hydrology" }
                        option { "Meteorology" }
                        option { "Oceanography" }
                        option { "Glaciology" }
                        option { "Soil Science" }
                        option { "Volcanology" }
                        option { "Seismology" }
                        option { "Geochemistry" }
                        option { "Paleoclimatology" }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Geological Timescale" }
                    select {
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        onchange: move |e| timescale.set(e.value()),
                        option { "Holocene" }
                        option { "Pleistocene" }
                        option { "Neogene" }
                        option { "Paleogene" }
                        option { "Cretaceous" }
                        option { "Jurassic" }
                        option { "Triassic" }
                        option { "Permian" }
                        option { "Carboniferous" }
                        option { "Devonian" }
                        option { "Silurian" }
                        option { "Ordovician" }
                        option { "Cambrian" }
                        option { "Proterozoic" }
                        option { "Archean" }
                        option { "Hadean" }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Rock Type" }
                    select {
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        onchange: move |e| rock_type.set(e.value()),
                        option { "Igneous" }
                        option { "Sedimentary" }
                        option { "Metamorphic" }
                        option { "Volcanic" }
                        option { "Plutonic" }
                        option { "Clastic" }
                        option { "Carbonate" }
                        option { "Evaporite" }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Tectonic Setting" }
                    select {
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        onchange: move |e| tectonic_setting.set(e.value()),
                        option { "Subduction Zone" }
                        option { "Mid-Ocean Ridge" }
                        option { "Collision Zone" }
                        option { "Intraplate" }
                        option { "Rift Zone" }
                        option { "Transform Fault" }
                        option { "Passive Margin" }
                        option { "Hotspot" }
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Feature Notes" }
                textarea {
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; min-height: 80px; box-sizing: border-box; resize: vertical;",
                    placeholder: "Describe geological features, sample locations, stratigraphic context...",
                    oninput: move |e| feature_notes.set(e.value()),
                    "{feature_notes}"
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent); flex: 1;",
                h3 { style: "margin-top: 0; color: var(--qualia-accent); font-size: 0.9rem;", "Output / Analysis" }
                div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 8px; font-size: 0.8rem;",
                    div { style: "color: var(--qualia-text-muted);", "Subdiscipline:" }
                    div { style: "color: var(--qualia-text);", "{subdiscipline}" }
                    div { style: "color: var(--qualia-text-muted);", "Timescale:" }
                    div { style: "color: var(--qualia-text);", "{timescale}" }
                    div { style: "color: var(--qualia-text-muted);", "Rock Type:" }
                    div { style: "color: var(--qualia-text);", "{rock_type}" }
                    div { style: "color: var(--qualia-text-muted);", "Tectonic Setting:" }
                    div { style: "color: var(--qualia-text);", "{tectonic_setting}" }
                }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 12px; border-top: 1px solid var(--qualia-border); padding-top: 8px;",
                    "QualiaDB → Allen Interval temporal engine | geochemistry sieve | stratigraphic graph"
                }
            }
        }
    }
}
