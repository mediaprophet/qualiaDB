use dioxus::prelude::*;

#[component]
pub fn ChemistryQapp() -> Element {
    let mut subdiscipline = use_signal(|| "Organic".to_string());
    let mut reaction_type = use_signal(|| "Substitution SN2".to_string());
    let mut temperature_k = use_signal(|| 300.0f64);
    let mut pressure_atm = use_signal(|| 1.0f64);
    let mut concentration_m = use_signal(|| 1.0f64);
    let mut ph = use_signal(|| 7.0f64);
    let mut solvent = use_signal(|| "Water".to_string());
    let mut reagent_notes = use_signal(|| String::new());

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Chemistry QApp" }

            div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Subdiscipline" }
                    select {
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        onchange: move |e| subdiscipline.set(e.value()),
                        option { "Organic" }
                        option { "Inorganic" }
                        option { "Physical" }
                        option { "Analytical" }
                        option { "Biochemistry" }
                        option { "Materials" }
                        option { "Computational" }
                        option { "Green Chemistry" }
                        option { "Polymer" }
                        option { "Nuclear" }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Reaction Type" }
                    select {
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        onchange: move |e| reaction_type.set(e.value()),
                        option { "Substitution SN1" }
                        option { "Substitution SN2" }
                        option { "Addition" }
                        option { "Elimination" }
                        option { "Oxidation-Reduction" }
                        option { "Acid-Base" }
                        option { "Coordination" }
                        option { "Pericyclic" }
                        option { "Radical" }
                        option { "Coupling" }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Temperature (K): {temperature_k:.0}" }
                    input {
                        r#type: "range",
                        min: "0",
                        max: "2000",
                        step: "1",
                        value: "{temperature_k}",
                        style: "width: 100%; margin-top: 8px;",
                        oninput: move |e| temperature_k.set(e.value().parse().unwrap_or(300.0)),
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Pressure (atm): {pressure_atm:.2}" }
                    input {
                        r#type: "range",
                        min: "0.01",
                        max: "100",
                        step: "0.01",
                        value: "{pressure_atm}",
                        style: "width: 100%; margin-top: 8px;",
                        oninput: move |e| pressure_atm.set(e.value().parse().unwrap_or(1.0)),
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Concentration (M)" }
                    input {
                        r#type: "number",
                        value: "{concentration_m}",
                        step: "0.001",
                        min: "0",
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        oninput: move |e| concentration_m.set(e.value().parse().unwrap_or(1.0)),
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "pH: {ph:.2}" }
                    input {
                        r#type: "range",
                        min: "0",
                        max: "14",
                        step: "0.01",
                        value: "{ph}",
                        style: "width: 100%; margin-top: 8px;",
                        oninput: move |e| ph.set(e.value().parse().unwrap_or(7.0)),
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Solvent" }
                    select {
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        onchange: move |e| solvent.set(e.value()),
                        option { "Water" }
                        option { "Ethanol" }
                        option { "DMSO" }
                        option { "DCM" }
                        option { "THF" }
                        option { "Acetonitrile" }
                        option { "Hexane" }
                        option { "DMF" }
                        option { "Acetone" }
                        option { "Toluene" }
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Reagent Notes" }
                textarea {
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; min-height: 60px; box-sizing: border-box; resize: vertical;",
                    placeholder: "Enter reagents, catalysts, expected products, yield targets...",
                    oninput: move |e| reagent_notes.set(e.value()),
                    "{reagent_notes}"
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent); flex: 1;",
                h3 { style: "margin-top: 0; color: var(--qualia-accent); font-size: 0.9rem;", "Output / Analysis" }
                div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 8px; font-size: 0.8rem;",
                    div { style: "color: var(--qualia-text-muted);", "Subdiscipline:" }
                    div { style: "color: var(--qualia-text);", "{subdiscipline}" }
                    div { style: "color: var(--qualia-text-muted);", "Reaction:" }
                    div { style: "color: var(--qualia-text);", "{reaction_type}" }
                    div { style: "color: var(--qualia-text-muted);", "Temperature:" }
                    div { style: "color: var(--qualia-text);", "{temperature_k:.0} K" }
                    div { style: "color: var(--qualia-text-muted);", "Pressure:" }
                    div { style: "color: var(--qualia-text);", "{pressure_atm:.2} atm" }
                    div { style: "color: var(--qualia-text-muted);", "pH:" }
                    div { style: "color: var(--qualia-text);", "{ph:.2}" }
                    div { style: "color: var(--qualia-text-muted);", "Solvent:" }
                    div { style: "color: var(--qualia-text);", "{solvent}" }
                }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 12px; border-top: 1px solid var(--qualia-border); padding-top: 8px;",
                    "QualiaDB → chemistry_modeler engine | ODE thermodynamic solver | reaction pathway sieve"
                }
            }
        }
    }
}
