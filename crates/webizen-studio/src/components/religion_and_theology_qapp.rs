use dioxus::prelude::*;

#[component]
pub fn ReligionAndTheologyQapp() -> Element {
    let mut tradition = use_signal(|| "Christianity".to_string());
    let mut sub_tradition = use_signal(|| String::new());
    let mut text_reference = use_signal(|| String::new());
    let mut hermeneutic = use_signal(|| "Historical-Critical".to_string());
    let mut theological_theme = use_signal(|| String::new());
    let mut comparative_mode = use_signal(|| false);
    let mut second_tradition = use_signal(|| "Islam".to_string());

    let traditions = [
        "Christianity",
        "Islam",
        "Judaism",
        "Buddhism",
        "Hinduism",
        "Sikhism",
        "Taoism",
        "Confucianism",
        "Zoroastrianism",
        "Indigenous / Animist",
        "New Religious Movements",
        "Secular / Non-Religious",
    ];
    let hermeneutics = [
        "Historical-Critical",
        "Allegorical",
        "Typological",
        "Anagogical",
        "Tropological (Moral)",
        "Feminist Theology",
        "Liberation Theology",
        "Postcolonial Theology",
        "Apophatic (Negative Theology)",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Religion & Theology" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Religious Tradition" }
                    select {
                        value: "{tradition}",
                        onchange: move |e| tradition.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for t in traditions { option { value: "{t}", "{t}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Sub-tradition / Denomination" }
                    input {
                        r#type: "text", placeholder: "e.g. Sunni Hanafi, Theravāda, Reformed…",
                        value: "{sub_tradition}",
                        oninput: move |e| sub_tradition.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Sacred Text Reference" }
                input {
                    r#type: "text", placeholder: "e.g. Qur'an 2:255, John 1:1, Dhammapada 1–20, Bhagavad Gita 2.47…",
                    value: "{text_reference}",
                    oninput: move |e| text_reference.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                }
            }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Hermeneutical Approach" }
                    select {
                        value: "{hermeneutic}",
                        onchange: move |e| hermeneutic.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for h in hermeneutics { option { value: "{h}", "{h}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Theological Theme / Locus" }
                    input {
                        r#type: "text", placeholder: "e.g. soteriology, theodicy, eschatology, dharma…",
                        value: "{theological_theme}",
                        oninput: move |e| theological_theme.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
            }

            div {
                style: "display: flex; align-items: center; gap: 12px;",
                input {
                    r#type: "checkbox",
                    checked: "{comparative_mode}",
                    onchange: move |e| comparative_mode.set(e.checked()),
                    id: "comp-mode"
                }
                label { r#for: "comp-mode", style: "font-size: 0.85rem; color: var(--qualia-text-muted); cursor: pointer;", "Comparative Theology Mode" }
                if comparative_mode() {
                    select {
                        value: "{second_tradition}",
                        onchange: move |e| second_tradition.set(e.value()),
                        style: "flex: 1; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px;",
                        for t in traditions { option { value: "{t}", "{t}" } }
                    }
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent); flex: 1;",
                h3 { style: "margin-top: 0; color: var(--qualia-accent); font-size: 0.9rem;", "Theological Analysis" }
                div { style: "font-size: 0.85rem; color: var(--qualia-text);", "{tradition}" if !sub_tradition().is_empty() { " / {sub_tradition}" } }
                div { style: "font-size: 0.85rem; color: var(--qualia-text-muted); margin-top: 4px;", "Hermeneutic: {hermeneutic}" }
                if !text_reference().is_empty() {
                    div { style: "font-size: 0.85rem; color: var(--qualia-text-muted); margin-top: 4px;", "Text: {text_reference}" }
                }
                if comparative_mode() {
                    div { style: "font-size: 0.85rem; color: var(--qualia-accent); margin-top: 8px;", "Comparing: {tradition} ↔ {second_tradition}" }
                }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 8px;", "QualiaDB → epistemic logic | knowledge graph | theological ontology sieve" }
            }
        }
    }
}
