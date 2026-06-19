use dioxus::prelude::*;

#[component]
pub fn GenderAndSexualityStudiesQapp() -> Element {
    let mut theoretical_framework = use_signal(|| "Feminist Theory".to_string());
    let mut gender_category = use_signal(|| "Woman / Feminine".to_string());
    let mut sexuality_category = use_signal(|| "Heterosexuality".to_string());
    let mut intersectional_axes = use_signal(|| "Gender + Race".to_string());
    let mut method = use_signal(|| "Discourse Analysis".to_string());
    let mut policy_context = use_signal(|| String::new());
    let mut analysis_notes = use_signal(|| String::new());

    let frameworks = [
        "Feminist Theory (Second Wave)",
        "Feminist Theory (Third Wave)",
        "Queer Theory (Butler, Sedgwick)",
        "Trans Studies (Stryker)",
        "Intersectionality (Crenshaw)",
        "Masculinity Studies (Connell)",
        "Postcolonial Feminism",
        "Black Feminism / Womanism",
        "Lesbian Feminism",
        "Socialist Feminism",
        "Phenomenological (De Beauvoir)",
        "Affect Theory",
    ];
    let gender_cats = [
        "Woman / Feminine",
        "Man / Masculine",
        "Non-Binary / Genderqueer",
        "Transgender",
        "Gender Fluid",
        "Agender",
        "Two-Spirit (Indigenous)",
        "Hijra / Third Gender",
        "Intersex",
        "All / Non-Specified",
    ];
    let sexuality_cats = [
        "Heterosexuality",
        "Homosexuality (Gay)",
        "Homosexuality (Lesbian)",
        "Bisexuality",
        "Pansexuality",
        "Asexuality",
        "Queer (Umbrella)",
        "Aromantic",
        "Demisexual",
        "All / Non-Specified",
    ];
    let intersections = [
        "Gender + Race",
        "Gender + Class",
        "Gender + Sexuality",
        "Gender + Disability",
        "Gender + Religion",
        "Gender + Nation",
        "Gender + Age",
        "Multiple / Multidimensional",
    ];
    let methods = [
        "Discourse Analysis",
        "Ethnography",
        "Survey",
        "Narrative / Autoethnography",
        "Policy Analysis",
        "Historical Analysis",
        "Textual / Literary Analysis",
        "Participatory Action Research",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Gender & Sexuality Studies" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Theoretical Framework" }
                    select {
                        value: "{theoretical_framework}",
                        onchange: move |e| theoretical_framework.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for x in frameworks { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Research Method" }
                    select {
                        value: "{method}",
                        onchange: move |e| method.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for x in methods { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Gender Category Focus" }
                    select {
                        value: "{gender_category}",
                        onchange: move |e| gender_category.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for x in gender_cats { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Sexuality Category Focus" }
                    select {
                        value: "{sexuality_category}",
                        onchange: move |e| sexuality_category.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for x in sexuality_cats { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Intersectional Axes" }
                    select {
                        value: "{intersectional_axes}",
                        onchange: move |e| intersectional_axes.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for x in intersections { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Policy / Legal Context" }
                    input {
                        r#type: "text", placeholder: "e.g. Title IX, CEDAW, Marriage Equality Act…",
                        value: "{policy_context}",
                        oninput: move |e| policy_context.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
            }

            div {
                style: "flex: 1; display: flex; flex-direction: column;",
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Analysis Notes" }
                textarea {
                    value: "{analysis_notes}",
                    oninput: move |e| analysis_notes.set(e.value()),
                    style: "flex: 1; width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; resize: none; box-sizing: border-box; min-height: 80px;"
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 12px 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent); display: flex; gap: 16px; flex-wrap: wrap;",
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{theoretical_framework}" }
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{intersectional_axes}" }
                span { style: "font-size: 0.8rem; color: var(--qualia-accent);", "{method}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); width: 100%;", "QualiaDB → discourse sieve | epistemic logic | intersectionality graph" }
            }
        }
    }
}
