use dioxus::prelude::*;

#[component]
pub fn NeuroscienceQapp() -> Element {
    let mut level = use_signal(|| "Systems".to_string());
    let mut technique = use_signal(|| "fMRI".to_string());
    let mut brain_region = use_signal(|| "Prefrontal Cortex".to_string());
    let mut neurotransmitter = use_signal(|| "Glutamate".to_string());
    let mut firing_rate_hz = use_signal(|| 40.0f64);
    let mut membrane_potential_mv = use_signal(|| -70.0f64);
    let mut research_notes = use_signal(|| String::new());

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Neuroscience QApp" }

            div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Level of Analysis" }
                    select {
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        onchange: move |e| level.set(e.value()),
                        option { "Molecular" }
                        option { "Cellular" }
                        option { "Circuit" }
                        option { "Systems" }
                        option { "Cognitive" }
                        option { "Computational" }
                        option { "Clinical" }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Technique" }
                    select {
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        onchange: move |e| technique.set(e.value()),
                        option { "fMRI" }
                        option { "EEG" }
                        option { "Patch Clamp" }
                        option { "Two-Photon Imaging" }
                        option { "Optogenetics" }
                        option { "CRISPR" }
                        option { "Diffusion Tensor Imaging" }
                        option { "MEG" }
                        option { "PET" }
                        option { "Single-unit recording" }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Brain Region" }
                    select {
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        onchange: move |e| brain_region.set(e.value()),
                        option { "Prefrontal Cortex" }
                        option { "Hippocampus" }
                        option { "Amygdala" }
                        option { "Cerebellum" }
                        option { "Basal Ganglia" }
                        option { "Brainstem" }
                        option { "Thalamus" }
                        option { "Hypothalamus" }
                        option { "Motor Cortex" }
                        option { "Visual Cortex" }
                        option { "Corpus Callosum" }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Neurotransmitter" }
                    select {
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        onchange: move |e| neurotransmitter.set(e.value()),
                        option { "Glutamate" }
                        option { "GABA" }
                        option { "Dopamine" }
                        option { "Serotonin" }
                        option { "Acetylcholine" }
                        option { "Norepinephrine" }
                        option { "Endocannabinoids" }
                        option { "Glycine" }
                        option { "Oxytocin" }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Firing Rate (Hz): {firing_rate_hz:.1}" }
                    input {
                        r#type: "range",
                        min: "0.0",
                        max: "500.0",
                        step: "0.5",
                        value: "{firing_rate_hz}",
                        style: "width: 100%; margin-top: 8px;",
                        oninput: move |e| firing_rate_hz.set(e.value().parse().unwrap_or(40.0)),
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Membrane Potential (mV)" }
                    input {
                        r#type: "number",
                        value: "{membrane_potential_mv}",
                        step: "0.1",
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        oninput: move |e| membrane_potential_mv.set(e.value().parse().unwrap_or(-70.0)),
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Research Notes" }
                textarea {
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; min-height: 60px; box-sizing: border-box; resize: vertical;",
                    placeholder: "Experimental design, subject details, stimulation protocols, findings...",
                    oninput: move |e| research_notes.set(e.value()),
                    "{research_notes}"
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent); flex: 1;",
                h3 { style: "margin-top: 0; color: var(--qualia-accent); font-size: 0.9rem;", "Output / Analysis" }
                div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 8px; font-size: 0.8rem;",
                    div { style: "color: var(--qualia-text-muted);", "Level:" }
                    div { style: "color: var(--qualia-text);", "{level}" }
                    div { style: "color: var(--qualia-text-muted);", "Technique:" }
                    div { style: "color: var(--qualia-text);", "{technique}" }
                    div { style: "color: var(--qualia-text-muted);", "Brain Region:" }
                    div { style: "color: var(--qualia-text);", "{brain_region}" }
                    div { style: "color: var(--qualia-text-muted);", "Neurotransmitter:" }
                    div { style: "color: var(--qualia-text);", "{neurotransmitter}" }
                    div { style: "color: var(--qualia-text-muted);", "Firing Rate:" }
                    div { style: "color: var(--qualia-text);", "{firing_rate_hz:.1} Hz" }
                    div { style: "color: var(--qualia-text-muted);", "V_m:" }
                    div { style: "color: var(--qualia-text);", "{membrane_potential_mv:.1} mV" }
                }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 12px; border-top: 1px solid var(--qualia-border); padding-top: 8px;",
                    "QualiaDB → ODE Hodgkin-Huxley solver | graph connectome | neuro-symbolic sieve"
                }
            }
        }
    }
}
