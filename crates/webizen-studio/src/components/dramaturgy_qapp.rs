use dioxus::prelude::*;

#[component]
pub fn DramaturgyQapp() -> Element {
    let mut production_type = use_signal(|| "New Play Development".to_string());
    let mut dramaturgical_role = use_signal(|| "Conceptual".to_string());
    let mut theoretical_lens = use_signal(|| "Brechtian".to_string());
    let mut script_analysis_method = use_signal(|| "Action".to_string());
    let mut research_area = use_signal(|| String::new());
    let mut production_notes = use_signal(|| String::new());

    let production_types = [
        "New Play Development",
        "Classical Revival",
        "Devised",
        "Musical",
        "Opera",
        "Physical Theatre",
        "Site-Specific",
    ];
    let dramaturgical_roles = [
        "Conceptual",
        "Research",
        "Script Editing",
        "Audience Advocacy",
        "Production Dramaturgy",
        "Literary Manager",
    ];
    let lenses = [
        "Brechtian",
        "Stanislavski",
        "Artaudian",
        "Postdramatic",
        "New Dramaturgies",
        "Feminist",
    ];
    let analysis_methods = [
        "Action",
        "Super-Objective",
        "Given Circumstances",
        "Beats",
        "Spine",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Dramaturgy" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Production Type" }
                    select {
                        value: "{production_type}",
                        onchange: move |e| production_type.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in production_types { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Dramaturgical Role" }
                    select {
                        value: "{dramaturgical_role}",
                        onchange: move |e| dramaturgical_role.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in dramaturgical_roles { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Theoretical Lens" }
                    select {
                        value: "{theoretical_lens}",
                        onchange: move |e| theoretical_lens.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in lenses { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Script Analysis Method" }
                    select {
                        value: "{script_analysis_method}",
                        onchange: move |e| script_analysis_method.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in analysis_methods { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Research Area" }
                input {
                    r#type: "text",
                    value: "{research_area}",
                    oninput: move |e| research_area.set(e.value()),
                    placeholder: "e.g. Greek tragedy, Weimar cabaret, devised theatre",
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                }
            }

            div {
                style: "flex: 1; display: flex; flex-direction: column;",
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Production Notes" }
                textarea {
                    value: "{production_notes}",
                    oninput: move |e| production_notes.set(e.value()),
                    style: "flex: 1; width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; resize: none; box-sizing: border-box; min-height: 80px;"
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 12px 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent);",
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{production_type} | {dramaturgical_role} | {theoretical_lens} | {script_analysis_method}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → dramaturgy engine | script analysis sieve | theatre history anchor" }
            }
        }
    }
}
