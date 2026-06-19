use dioxus::prelude::*;

#[component]
pub fn IslamicStudiesQapp() -> Element {
    let mut tradition = use_signal(|| "Sunni Hanafi".to_string());
    let mut discipline = use_signal(|| "Quran Studies".to_string());
    let mut primary_text = use_signal(|| String::new());
    let mut historical_period = use_signal(|| "Abbasid".to_string());
    let mut methodology = use_signal(|| "Textual".to_string());
    let mut notes = use_signal(|| String::new());

    let traditions = [
        "Sunni Hanafi",
        "Sunni Maliki",
        "Sunni Shafi'i",
        "Sunni Hanbali",
        "Shia Twelver",
        "Shia Ismaili",
        "Ibadi",
        "Sufi Orders",
        "Ahmadiyya",
        "Secular Muslim Thought",
    ];
    let disciplines = [
        "Quran Studies",
        "Hadith",
        "Fiqh",
        "Kalam",
        "Sufism",
        "Islamic History",
        "Islamic Philosophy",
        "Islamic Political Thought",
        "Gender & Islam",
    ];
    let periods = [
        "Prophetic",
        "Rashidun",
        "Umayyad",
        "Abbasid",
        "Medieval",
        "Ottoman",
        "Colonial",
        "Contemporary",
    ];
    let methodologies = [
        "Textual",
        "Historical-Critical",
        "Anthropological",
        "Postcolonial",
        "Feminist",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Islamic Studies" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Tradition" }
                    select {
                        value: "{tradition}",
                        onchange: move |e| tradition.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in traditions { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Discipline" }
                    select {
                        value: "{discipline}",
                        onchange: move |e| discipline.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in disciplines { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Historical Period" }
                    select {
                        value: "{historical_period}",
                        onchange: move |e| historical_period.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in periods { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Methodology" }
                    select {
                        value: "{methodology}",
                        onchange: move |e| methodology.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in methodologies { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Primary Text" }
                input {
                    r#type: "text",
                    value: "{primary_text}",
                    oninput: move |e| primary_text.set(e.value()),
                    placeholder: "e.g. Quran Sura 2, Bukhari Hadith, Al-Ghazali Ihya",
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{tradition} | {discipline} | {historical_period} | {methodology}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → Islamic corpus engine | fiqh sieve | hadith anchor" }
            }
        }
    }
}
