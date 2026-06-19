use dioxus::prelude::*;

#[component]
pub fn PaleontologyQapp() -> Element {
    let mut fossil_type = use_signal(|| "Body Fossil".to_string());
    let mut taxonomic_group = use_signal(|| "Dinosauria".to_string());
    let mut geological_period = use_signal(|| "Cretaceous".to_string());
    let mut preservation_quality = use_signal(|| "Good".to_string());
    let mut age_mya = use_signal(|| 66.0f64);
    let mut locality = use_signal(|| String::new());
    let mut notes = use_signal(|| String::new());

    let fossil_types = [
        "Body Fossil",
        "Trace Fossil",
        "Molecular Fossil",
        "Exceptional Preservation",
        "Compression",
        "Cast & Mould",
    ];
    let taxonomic_groups = [
        "Dinosauria",
        "Marine Reptile",
        "Pterosauria",
        "Mammal",
        "Trilobite",
        "Graptolite",
        "Plant",
        "Microfossil",
        "Early Hominid",
    ];
    let geological_periods = [
        "Cambrian",
        "Ordovician",
        "Silurian",
        "Devonian",
        "Carboniferous",
        "Permian",
        "Triassic",
        "Jurassic",
        "Cretaceous",
        "Paleogene",
        "Neogene",
    ];
    let preservation_qualities = ["Exceptional", "Good", "Fragmentary", "Poor"];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Paleontology" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Fossil Type" }
                    select {
                        value: "{fossil_type}",
                        onchange: move |e| fossil_type.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in fossil_types { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Taxonomic Group" }
                    select {
                        value: "{taxonomic_group}",
                        onchange: move |e| taxonomic_group.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in taxonomic_groups { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Geological Period" }
                    select {
                        value: "{geological_period}",
                        onchange: move |e| geological_period.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in geological_periods { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Preservation Quality" }
                    select {
                        value: "{preservation_quality}",
                        onchange: move |e| preservation_quality.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in preservation_qualities { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Age (Mya): {age_mya:.1}" }
                input {
                    r#type: "range",
                    min: "0",
                    max: "600",
                    step: "1",
                    value: "{age_mya()}",
                    oninput: move |e| age_mya.set(e.value().parse().unwrap_or(66.0)),
                    style: "width: 100%; margin-top: 4px;"
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Locality" }
                input {
                    r#type: "text",
                    value: "{locality}",
                    oninput: move |e| locality.set(e.value()),
                    placeholder: "e.g. Hell Creek Formation Montana, Liaoning China",
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{fossil_type} | {taxonomic_group} | {geological_period} | {age_mya:.0}Mya | {preservation_quality}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → palaeontological engine | stratigraphic sieve | taxonomic anchor" }
            }
        }
    }
}
