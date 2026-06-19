use dioxus::prelude::*;

#[component]
pub fn FoodStudiesQapp() -> Element {
    let mut subfield = use_signal(|| "Food History".to_string());
    let mut theoretical_lens = use_signal(|| "Cultural Studies".to_string());
    let mut scale = use_signal(|| "Regional".to_string());
    let mut food_system_type = use_signal(|| "Traditional".to_string());
    let mut cuisine_or_tradition = use_signal(|| String::new());
    let mut dietary_pattern = use_signal(|| "Omnivore".to_string());
    let mut notes = use_signal(|| String::new());

    let subfields = [
        "Food History",
        "Food Anthropology",
        "Food Policy",
        "Food Justice",
        "Culinary Arts",
        "Food Science",
        "Gastronomy",
        "Agricultural Ethics",
    ];
    let lenses = [
        "Political Economy",
        "Cultural Studies",
        "Gender",
        "Postcolonial",
        "Environmental",
        "Phenomenological",
    ];
    let scales = [
        "Global Supply Chain",
        "National",
        "Regional",
        "Local",
        "Household",
        "Individual",
    ];
    let system_types = [
        "Industrial",
        "Agroecological",
        "Traditional",
        "Urban",
        "Post-Scarcity",
    ];
    let patterns = [
        "Omnivore",
        "Vegetarian",
        "Vegan",
        "Kosher",
        "Halal",
        "Macrobiotic",
        "Locavore",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Food Studies" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Subfield" }
                    select {
                        value: "{subfield}",
                        onchange: move |e| subfield.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in subfields { option { value: "{x}", "{x}" } }
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
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Scale" }
                    select {
                        value: "{scale}",
                        onchange: move |e| scale.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in scales { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Food System Type" }
                    select {
                        value: "{food_system_type}",
                        onchange: move |e| food_system_type.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in system_types { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Dietary Pattern" }
                    select {
                        value: "{dietary_pattern}",
                        onchange: move |e| dietary_pattern.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in patterns { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Cuisine / Tradition" }
                input {
                    r#type: "text",
                    value: "{cuisine_or_tradition}",
                    oninput: move |e| cuisine_or_tradition.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{subfield} | {food_system_type} | {scale} | {dietary_pattern}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → supply chain graph | food justice sieve | gastronomy engine" }
            }
        }
    }
}
