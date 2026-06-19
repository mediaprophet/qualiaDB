use dioxus::prelude::*;

#[component]
pub fn SportsStudiesQapp() -> Element {
    let mut theoretical_lens = use_signal(|| "Sociology of Sport".to_string());
    let mut sport = use_signal(|| String::new());
    let mut professional_level = use_signal(|| "Professional".to_string());
    let mut governance_body = use_signal(|| String::new());
    let mut commercialisation_level = use_signal(|| "High".to_string());
    let mut doping_context = use_signal(|| "Clean".to_string());
    let mut attendance = use_signal(|| 25000u32);
    let mut notes = use_signal(|| String::new());

    let lenses = [
        "Sociology of Sport",
        "Political Economy of Sport",
        "Sports Psychology",
        "Gender & Sport",
        "Postcolonial Sport",
        "Critical Race & Sport",
        "Media & Sport",
        "Globalisation of Sport",
    ];
    let levels = [
        "Grassroots",
        "Amateur",
        "Semi-Professional",
        "Professional",
        "Olympic",
        "Paralympic",
    ];
    let commercialisation_levels = ["Low", "Medium", "High", "Global Mega-Event"];
    let doping_contexts = ["Clean", "Suspected", "Adjudicated"];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Sports Studies" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
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
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Professional Level" }
                    select {
                        value: "{professional_level}",
                        onchange: move |e| professional_level.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in levels { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Commercialisation Level" }
                    select {
                        value: "{commercialisation_level}",
                        onchange: move |e| commercialisation_level.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in commercialisation_levels { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Doping Context" }
                    select {
                        value: "{doping_context}",
                        onchange: move |e| doping_context.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in doping_contexts { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Sport" }
                    input {
                        r#type: "text",
                        value: "{sport}",
                        oninput: move |e| sport.set(e.value()),
                        placeholder: "e.g. football, cricket, athletics",
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Governance Body" }
                    input {
                        r#type: "text",
                        value: "{governance_body}",
                        oninput: move |e| governance_body.set(e.value()),
                        placeholder: "e.g. FIFA, IOC, WADA",
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Attendance: {attendance}" }
                input {
                    r#type: "range",
                    min: "0",
                    max: "100000",
                    step: "500",
                    value: "{attendance}",
                    oninput: move |e| attendance.set(e.value().parse().unwrap_or(25000)),
                    style: "width: 100%; margin-top: 4px;"
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{sport} | {theoretical_lens} | {professional_level} | {commercialisation_level} | Att:{attendance}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → sports sociology engine | governance sieve | commercialisation anchor" }
            }
        }
    }
}
