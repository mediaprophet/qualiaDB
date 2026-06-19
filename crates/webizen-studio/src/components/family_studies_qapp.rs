use dioxus::prelude::*;

#[component]
pub fn FamilyStudiesQapp() -> Element {
    let mut family_form = use_signal(|| "Nuclear".to_string());
    let mut theoretical_lens = use_signal(|| "Systems Theory".to_string());
    let mut issue_area = use_signal(|| "Parenting".to_string());
    let mut methodology = use_signal(|| "Survey".to_string());
    let mut notes = use_signal(|| String::new());

    let family_forms = [
        "Nuclear",
        "Extended",
        "Single Parent",
        "Blended",
        "Same-Sex",
        "Childless",
        "Multigenerational",
        "Cohabiting",
        "Chosen Family",
    ];
    let lenses = [
        "Systems Theory",
        "Structural-Functional",
        "Feminist",
        "Life Course",
        "Ecological Systems (Bronfenbrenner)",
        "Intersectional",
    ];
    let issues = [
        "Parenting",
        "Divorce",
        "Elder Care",
        "Domestic Violence",
        "Child Development",
        "Work-Family Balance",
        "Adoption",
        "Fertility",
        "Grief",
    ];
    let methods = [
        "Survey",
        "Ethnography",
        "Clinical",
        "Mixed Methods",
        "Longitudinal",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Family Studies" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Family Form" }
                    select {
                        value: "{family_form}",
                        onchange: move |e| family_form.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in family_forms { option { value: "{x}", "{x}" } }
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
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Issue Area" }
                    select {
                        value: "{issue_area}",
                        onchange: move |e| issue_area.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in issues { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Methodology" }
                    select {
                        value: "{methodology}",
                        onchange: move |e| methodology.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in methods { option { value: "{x}", "{x}" } }
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{family_form} | {theoretical_lens} | {issue_area} | {methodology}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → family systems engine | social sieve | life course anchor" }
            }
        }
    }
}
