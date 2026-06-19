use dioxus::prelude::*;

#[component]
pub fn BiologyQapp() -> Element {
    let mut subdiscipline = use_signal(|| "Molecular Biology".to_string());
    let mut organism_domain = use_signal(|| "Eukarya".to_string());
    let mut method = use_signal(|| "Sequencing".to_string());
    let mut gene_or_protein = use_signal(|| String::new());
    let mut pathway = use_signal(|| String::new());
    let mut sample_type = use_signal(|| String::new());
    let mut notes = use_signal(|| String::new());

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Biology QApp" }

            div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Subdiscipline" }
                    select {
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        onchange: move |e| subdiscipline.set(e.value()),
                        option { "Cell Biology" }
                        option { "Genetics" }
                        option { "Molecular Biology" }
                        option { "Physiology" }
                        option { "Developmental Biology" }
                        option { "Microbiology" }
                        option { "Immunology" }
                        option { "Ecology" }
                        option { "Evolutionary Biology" }
                        option { "Structural Biology" }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Organism Domain" }
                    select {
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        onchange: move |e| organism_domain.set(e.value()),
                        option { "Bacteria" }
                        option { "Archaea" }
                        option { "Eukarya" }
                        option { "Virus" }
                        option { "Viroid" }
                        option { "Prion" }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Experimental Method" }
                    select {
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        onchange: move |e| method.set(e.value()),
                        option { "PCR" }
                        option { "Western Blot" }
                        option { "CRISPR" }
                        option { "Microscopy" }
                        option { "Flow Cytometry" }
                        option { "Sequencing" }
                        option { "Electrophysiology" }
                        option { "Mass Spectrometry" }
                        option { "ChIP-seq" }
                        option { "RNA-seq" }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Gene or Protein" }
                    input {
                        r#type: "text",
                        value: "{gene_or_protein}",
                        placeholder: "e.g. TP53, BRCA1, Actin...",
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        oninput: move |e| gene_or_protein.set(e.value()),
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Biological Pathway" }
                    input {
                        r#type: "text",
                        value: "{pathway}",
                        placeholder: "e.g. MAPK, mTOR, Apoptosis...",
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        oninput: move |e| pathway.set(e.value()),
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Sample Type" }
                    input {
                        r#type: "text",
                        value: "{sample_type}",
                        placeholder: "e.g. HEK293, primary neurons, blood...",
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        oninput: move |e| sample_type.set(e.value()),
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Research Notes" }
                textarea {
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; min-height: 60px; box-sizing: border-box; resize: vertical;",
                    placeholder: "Enter hypotheses, protocols, expected outcomes...",
                    oninput: move |e| notes.set(e.value()),
                    "{notes}"
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent); flex: 1;",
                h3 { style: "margin-top: 0; color: var(--qualia-accent); font-size: 0.9rem;", "Output / Analysis" }
                div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 8px; font-size: 0.8rem;",
                    div { style: "color: var(--qualia-text-muted);", "Subdiscipline:" }
                    div { style: "color: var(--qualia-text);", "{subdiscipline}" }
                    div { style: "color: var(--qualia-text-muted);", "Domain:" }
                    div { style: "color: var(--qualia-text);", "{organism_domain}" }
                    div { style: "color: var(--qualia-text-muted);", "Method:" }
                    div { style: "color: var(--qualia-text);", "{method}" }
                    div { style: "color: var(--qualia-text-muted);", "Gene/Protein:" }
                    div { style: "color: var(--qualia-text);", "{gene_or_protein}" }
                    div { style: "color: var(--qualia-text-muted);", "Pathway:" }
                    div { style: "color: var(--qualia-text);", "{pathway}" }
                }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 12px; border-top: 1px solid var(--qualia-border); padding-top: 8px;",
                    "QualiaDB → bioinformatics_lab engine | graph theory pathway analysis | neuro-symbolic sieve"
                }
            }
        }
    }
}
