use dioxus::prelude::*;

#[component]
pub fn BiophysicsQapp() -> Element {
    let mut subfield = use_signal(|| "Single Molecule".to_string());
    let mut technique = use_signal(|| "Optical Tweezers".to_string());
    let mut force_pn = use_signal(|| 10.0f64);
    let mut diffusion_coefficient = use_signal(|| 1.0f64);
    let mut membrane_potential_mv = use_signal(|| -70.0f64);
    let mut kt_ratio = use_signal(|| 1.0f64);
    let mut notes = use_signal(|| String::new());

    let subfields = [
        "Membrane Biophysics",
        "Single Molecule",
        "Motor Proteins",
        "Neuronal Electrophysiology",
        "Structural Biophysics",
        "Computational",
        "Systems Biophysics",
    ];
    let techniques = [
        "AFM",
        "Optical Tweezers",
        "Patch Clamp",
        "Cryo-EM",
        "FRET",
        "MD Simulation",
        "NMR",
        "X-ray Crystallography",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Biophysics" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Subfield" }
                    select {
                        value: "{subfield}",
                        onchange: move |e| subfield.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in subfields { option { value: "{x}", "{x}" } }
                    }
                }
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
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Force (pN)" }
                    input {
                        r#type: "number",
                        step: "0.1",
                        value: "{force_pn}",
                        oninput: move |e| force_pn.set(e.value().parse().unwrap_or(10.0)),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Diffusion Coefficient (µm²/s)" }
                    input {
                        r#type: "number",
                        step: "0.01",
                        value: "{diffusion_coefficient}",
                        oninput: move |e| diffusion_coefficient.set(e.value().parse().unwrap_or(1.0)),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Membrane Potential (mV)" }
                    input {
                        r#type: "number",
                        step: "1",
                        value: "{membrane_potential_mv}",
                        oninput: move |e| membrane_potential_mv.set(e.value().parse().unwrap_or(-70.0)),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "kT Ratio (thermal energy)" }
                    input {
                        r#type: "number",
                        step: "0.1",
                        value: "{kt_ratio}",
                        oninput: move |e| kt_ratio.set(e.value().parse().unwrap_or(1.0)),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{subfield} | {technique} | F={force_pn:.1}pN | Vm={membrane_potential_mv:.0}mV | kT={kt_ratio:.1}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → molecular dynamics engine | force spectroscopy sieve | membrane physics graph" }
            }
        }
    }
}
