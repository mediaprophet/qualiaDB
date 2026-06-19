use dioxus::prelude::*;

#[component]
pub fn EvolutionaryBiologyQapp() -> Element {
    let mut mechanism = use_signal(|| "Natural Selection".to_string());
    let mut speciation_mode = use_signal(|| "Allopatric".to_string());
    let mut phylogenetic_method = use_signal(|| "Maximum Likelihood".to_string());
    let mut time_mya = use_signal(|| 10.0f64);
    let mut selection_coefficient_s = use_signal(|| 0.01f64);
    let mut population_size_ne = use_signal(|| 10000u32);
    let mut clade_notes = use_signal(|| String::new());

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Evolutionary Biology QApp" }

            div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Evolutionary Mechanism" }
                    select {
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        onchange: move |e| mechanism.set(e.value()),
                        option { "Natural Selection" }
                        option { "Genetic Drift" }
                        option { "Gene Flow" }
                        option { "Mutation" }
                        option { "Sexual Selection" }
                        option { "Kin Selection" }
                        option { "Group Selection" }
                        option { "Neutral Evolution" }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Speciation Mode" }
                    select {
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        onchange: move |e| speciation_mode.set(e.value()),
                        option { "Allopatric" }
                        option { "Sympatric" }
                        option { "Parapatric" }
                        option { "Peripatric" }
                        option { "Anagenesis" }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Phylogenetic Method" }
                    select {
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        onchange: move |e| phylogenetic_method.set(e.value()),
                        option { "Maximum Likelihood" }
                        option { "Bayesian Inference" }
                        option { "Parsimony" }
                        option { "Distance-Based (NJ)" }
                        option { "UPGMA" }
                        option { "Supertree" }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Divergence Time (Mya): {time_mya:.1}" }
                    input {
                        r#type: "range",
                        min: "0.001",
                        max: "3500",
                        step: "0.1",
                        value: "{time_mya}",
                        style: "width: 100%; margin-top: 8px;",
                        oninput: move |e| time_mya.set(e.value().parse().unwrap_or(10.0)),
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Selection Coefficient s: {selection_coefficient_s:.4}" }
                    input {
                        r#type: "range",
                        min: "0.0",
                        max: "1.0",
                        step: "0.0001",
                        value: "{selection_coefficient_s}",
                        style: "width: 100%; margin-top: 8px;",
                        oninput: move |e| selection_coefficient_s.set(e.value().parse().unwrap_or(0.01)),
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Effective Population Size (Ne)" }
                    input {
                        r#type: "number",
                        value: "{population_size_ne}",
                        min: "1",
                        step: "100",
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        oninput: move |e| population_size_ne.set(e.value().parse().unwrap_or(10000)),
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Clade Notes" }
                textarea {
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; min-height: 60px; box-sizing: border-box; resize: vertical;",
                    placeholder: "Synapomorphies, outgroup, fossil calibration points, genomic markers...",
                    oninput: move |e| clade_notes.set(e.value()),
                    "{clade_notes}"
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent); flex: 1;",
                h3 { style: "margin-top: 0; color: var(--qualia-accent); font-size: 0.9rem;", "Output / Analysis" }
                div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 8px; font-size: 0.8rem;",
                    div { style: "color: var(--qualia-text-muted);", "Mechanism:" }
                    div { style: "color: var(--qualia-text);", "{mechanism}" }
                    div { style: "color: var(--qualia-text-muted);", "Speciation:" }
                    div { style: "color: var(--qualia-text);", "{speciation_mode}" }
                    div { style: "color: var(--qualia-text-muted);", "Method:" }
                    div { style: "color: var(--qualia-text);", "{phylogenetic_method}" }
                    div { style: "color: var(--qualia-text-muted);", "Divergence:" }
                    div { style: "color: var(--qualia-text);", "{time_mya:.1} Mya" }
                    div { style: "color: var(--qualia-text-muted);", "s (selection):" }
                    div { style: "color: var(--qualia-text);", "{selection_coefficient_s:.4}" }
                    div { style: "color: var(--qualia-text-muted);", "Ne:" }
                    div { style: "color: var(--qualia-text);", "{population_size_ne}" }
                }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 12px; border-top: 1px solid var(--qualia-border); padding-top: 8px;",
                    "QualiaDB → graph phylogenetic tree | Bayesian epistemic engine | ODE population genetics"
                }
            }
        }
    }
}
