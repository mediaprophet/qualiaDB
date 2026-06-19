use dioxus::prelude::*;

#[component]
pub fn BiblicalStudiesQapp() -> Element {
    let mut testament = use_signal(|| "New Testament".to_string());
    let mut hermeneutical_method = use_signal(|| "Historical-Critical".to_string());
    let mut textual_tradition = use_signal(|| "Masoretic Text".to_string());
    let mut passage_reference = use_signal(|| String::new());
    let mut literary_form = use_signal(|| "Gospel".to_string());
    let mut language = use_signal(|| "Koine Greek".to_string());
    let mut notes = use_signal(|| String::new());

    let testaments = [
        "Old Testament",
        "Hebrew Bible",
        "New Testament",
        "Deuterocanonical",
        "Dead Sea Scrolls",
        "Extra-Canonical",
    ];
    let methods = [
        "Historical-Critical",
        "Canonical Criticism",
        "Rhetorical",
        "Narrative",
        "Feminist",
        "Liberation",
        "Postcolonial",
        "Canonical Approach",
    ];
    let traditions = [
        "Masoretic Text",
        "Septuagint",
        "Vulgate",
        "Dead Sea Scrolls",
        "Peshitta",
        "Samaritan Pentateuch",
    ];
    let literary_forms = [
        "Law",
        "Prophecy",
        "Wisdom",
        "Psalm",
        "Gospel",
        "Epistle",
        "Apocalyptic",
    ];
    let languages = ["Biblical Hebrew", "Aramaic", "Koine Greek", "Latin"];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Biblical Studies" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Testament" }
                    select {
                        value: "{testament}",
                        onchange: move |e| testament.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in testaments { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Hermeneutical Method" }
                    select {
                        value: "{hermeneutical_method}",
                        onchange: move |e| hermeneutical_method.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in methods { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Textual Tradition" }
                    select {
                        value: "{textual_tradition}",
                        onchange: move |e| textual_tradition.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in traditions { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Literary Form" }
                    select {
                        value: "{literary_form}",
                        onchange: move |e| literary_form.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in literary_forms { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Language" }
                    select {
                        value: "{language}",
                        onchange: move |e| language.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in languages { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Passage Reference" }
                    input {
                        r#type: "text",
                        value: "{passage_reference}",
                        oninput: move |e| passage_reference.set(e.value()),
                        placeholder: "e.g. Genesis 1:1, John 3:16, Isaiah 53",
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{testament} | {hermeneutical_method} | {textual_tradition} | {language} | {passage_reference}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → biblical corpus engine | text-critical sieve | hermeneutic anchor" }
            }
        }
    }
}
