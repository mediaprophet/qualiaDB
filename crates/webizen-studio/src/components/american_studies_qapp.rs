use dioxus::prelude::*;

#[component]
pub fn AmericanStudiesQapp() -> Element {
    let mut period = use_signal(|| "Civil Rights".to_string());
    let mut lens = use_signal(|| "Cultural Studies".to_string());
    let mut region = use_signal(|| "New England".to_string());
    let mut primary_source_type = use_signal(|| "Literature".to_string());
    let mut research_question = use_signal(|| String::new());

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box; overflow-y: auto;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "American Studies QApp" }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Historical Period" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| period.set(e.value()),
                    option { "Colonial" }
                    option { "Antebellum" }
                    option { "Gilded Age" }
                    option { "Progressive Era" }
                    option { "New Deal" }
                    option { "Post-WWII" }
                    option { value: "Civil Rights", selected: true, "Civil Rights" }
                    option { "Reagan Era" }
                    option { "Contemporary" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Analytical Lens" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| lens.set(e.value()),
                    option { selected: true, "Cultural Studies" }
                    option { "Political History" }
                    option { "Literary" }
                    option { "Indigenous Perspectives" }
                    option { "Immigrant Studies" }
                    option { "African American" }
                    option { "Gender" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Region" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| region.set(e.value()),
                    option { selected: true, "New England" }
                    option { "South" }
                    option { "Midwest" }
                    option { "West" }
                    option { "Southwest" }
                    option { "Pacific" }
                    option { "Alaska" }
                    option { "Hawaii" }
                    option { "Transnational" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Primary Source Type" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| primary_source_type.set(e.value()),
                    option { selected: true, "Literature" }
                    option { "Film" }
                    option { "Policy" }
                    option { "Oral History" }
                    option { "Music" }
                    option { "Visual Art" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Research Question" }
                textarea {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box; min-height: 80px; resize: vertical;",
                    placeholder: "Enter your research question...",
                    oninput: move |e| research_question.set(e.value()),
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent); flex: 1;",
                h3 { style: "margin-top: 0; color: var(--qualia-accent); font-size: 0.9rem;", "QualiaDB Output" }
                div { style: "font-size: 0.8rem; color: var(--qualia-text-muted); display: flex; flex-direction: column; gap: 4px;",
                    div { "Period: {period()}" }
                    div { "Lens: {lens()}" }
                    div { "Region: {region()}" }
                    div { "Source: {primary_source_type()}" }
                }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 8px;", "QualiaDB → knowledge graph | Allen Interval | neuro-symbolic sieve" }
            }
        }
    }
}
