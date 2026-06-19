use dioxus::prelude::*;

#[component]
pub fn BalkanStudiesQapp() -> Element {
    let mut country_focus = use_signal(|| "Serbia".to_string());
    let mut period = use_signal(|| "Post-Yugoslav".to_string());
    let mut disciplinary_lens = use_signal(|| "History".to_string());
    let mut ethnic_or_religious_dimension = use_signal(|| "Orthodox Christian".to_string());
    let mut conflict_dimension = use_signal(|| "1990s Wars".to_string());
    let mut notes = use_signal(|| String::new());

    let countries = [
        "Serbia",
        "Croatia",
        "Bosnia",
        "Kosovo",
        "North Macedonia",
        "Bulgaria",
        "Albania",
        "Romania",
        "Greece",
        "Slovenia",
        "Montenegro",
    ];
    let periods = [
        "Byzantine",
        "Ottoman",
        "Habsburg",
        "Interwar",
        "Socialist Yugoslavia",
        "Post-Yugoslav",
        "Contemporary EU",
    ];
    let lenses = [
        "History",
        "Political Science",
        "Literature",
        "Anthropology",
        "Religion",
        "Nationalism Studies",
    ];
    let dimensions = [
        "Orthodox Christian",
        "Catholic",
        "Muslim",
        "Secular",
        "Mixed",
    ];
    let conflicts = [
        "Balkanism",
        "1990s Wars",
        "Reconciliation",
        "EU Integration",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Balkan Studies" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Country Focus" }
                    select {
                        value: "{country_focus}",
                        onchange: move |e| country_focus.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in countries { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Period" }
                    select {
                        value: "{period}",
                        onchange: move |e| period.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in periods { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Disciplinary Lens" }
                    select {
                        value: "{disciplinary_lens}",
                        onchange: move |e| disciplinary_lens.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in lenses { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Ethnic/Religious Dimension" }
                    select {
                        value: "{ethnic_or_religious_dimension}",
                        onchange: move |e| ethnic_or_religious_dimension.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in dimensions { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Conflict Dimension" }
                    select {
                        value: "{conflict_dimension}",
                        onchange: move |e| conflict_dimension.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in conflicts { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                style: "flex: 1; display: flex; flex-direction: column;",
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Notes" }
                textarea {
                    value: "{notes}",
                    oninput: move |e| notes.set(e.value()),
                    style: "flex: 1; width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; resize: none; box-sizing: border-box; min-height: 80px;"
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 12px 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent);",
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{country_focus} | {period} | {disciplinary_lens} | {ethnic_or_religious_dimension}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → regional history engine | conflict sieve | identity anchor" }
            }
        }
    }
}
