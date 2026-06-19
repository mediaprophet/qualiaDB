use dioxus::prelude::*;

#[component]
pub fn ScienceTechnologyAndSocietyStsQapp() -> Element {
    let mut sts_theme = use_signal(|| "Actor-Network Theory".to_string());
    let mut technology_domain = use_signal(|| "AI".to_string());
    let mut methodological_approach = use_signal(|| "Laboratory Studies".to_string());
    let mut timeframe = use_signal(|| String::new());
    let mut case_study = use_signal(|| String::new());
    let mut notes = use_signal(|| String::new());

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box; overflow-y: auto;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Science, Technology & Society (STS) QApp" }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "STS Theme" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| sts_theme.set(e.value()),
                    option { "Social Construction of Technology" }
                    option { selected: true, "Actor-Network Theory" }
                    option { "Sociotechnical Imaginary" }
                    option { "Technological Determinism" }
                    option { "Science Wars" }
                    option { "Feminist STS" }
                    option { "Postcolonial STS" }
                    option { "Risk Society" }
                    option { "Algorithmic Governance" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Technology Domain" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| technology_domain.set(e.value()),
                    option { selected: true, "AI" }
                    option { "Biotechnology" }
                    option { "Nuclear" }
                    option { "Internet" }
                    option { "Agriculture" }
                    option { "Pharmaceuticals" }
                    option { "Space" }
                    option { "Energy" }
                    option { "Surveillance" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Methodological Approach" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| methodological_approach.set(e.value()),
                    option { selected: true, "Laboratory Studies" }
                    option { "Historical" }
                    option { "Discourse Analysis" }
                    option { "Ethnography" }
                    option { "Policy Analysis" }
                    option { "Comparative" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Timeframe" }
                input {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    placeholder: "e.g. 1970s–present, post-2000...",
                    oninput: move |e| timeframe.set(e.value()),
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Case Study" }
                input {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    placeholder: "e.g. CRISPR regulation, social media algorithms...",
                    oninput: move |e| case_study.set(e.value()),
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Notes" }
                textarea {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box; min-height: 60px; resize: vertical;",
                    placeholder: "Additional notes...",
                    oninput: move |e| notes.set(e.value()),
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent); flex: 1;",
                h3 { style: "margin-top: 0; color: var(--qualia-accent); font-size: 0.9rem;", "QualiaDB Output" }
                div { style: "font-size: 0.8rem; color: var(--qualia-text-muted); display: flex; flex-direction: column; gap: 4px;",
                    div { "Theme: {sts_theme()}" }
                    div { "Domain: {technology_domain()}" }
                    div { "Method: {methodological_approach()}" }
                    div { "Case: {case_study()}" }
                }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 8px;", "QualiaDB → graph theory | epistemic logic | Allen Interval" }
            }
        }
    }
}
