use dioxus::prelude::*;

#[component]
pub fn JewishStudiesQapp() -> Element {
    let mut period = use_signal(|| "Rabbinic".to_string());
    let mut subfield = use_signal(|| "History".to_string());
    let mut text_reference = use_signal(|| String::new());
    let mut geographic_focus = use_signal(|| "Ashkenazi".to_string());
    let mut language = use_signal(|| "Hebrew".to_string());
    let mut hermeneutic = use_signal(|| "Historical-Critical".to_string());
    let mut notes = use_signal(|| String::new());

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box; overflow-y: auto;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Jewish Studies QApp" }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Period" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| period.set(e.value()),
                    option { "Biblical" }
                    option { "Second Temple" }
                    option { selected: true, "Rabbinic" }
                    option { "Medieval" }
                    option { "Early Modern" }
                    option { "Haskalah" }
                    option { "Holocaust" }
                    option { "Israel Studies" }
                    option { "Contemporary Diaspora" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Subfield" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| subfield.set(e.value()),
                    option { selected: true, "History" }
                    option { "Theology" }
                    option { "Literature" }
                    option { "Philosophy" }
                    option { "Language & Linguistics" }
                    option { "Gender" }
                    option { "Law" }
                    option { "Mysticism" }
                    option { "Zionism Studies" }
                    option { "Anti-Semitism Studies" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Text Reference" }
                input {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    placeholder: "e.g. Talmud Bavli, Zohar, Maimonides...",
                    oninput: move |e| text_reference.set(e.value()),
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Geographic Focus" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| geographic_focus.set(e.value()),
                    option { selected: true, "Ashkenazi" }
                    option { "Sephardi" }
                    option { "Mizrahi" }
                    option { "Ethiopian" }
                    option { "Yemenite" }
                    option { "Israeli" }
                    option { "Diaspora" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Language" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| language.set(e.value()),
                    option { selected: true, "Hebrew" }
                    option { "Aramaic" }
                    option { "Yiddish" }
                    option { "Ladino" }
                    option { "Judeo-Arabic" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Hermeneutic" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| hermeneutic.set(e.value()),
                    option { "Midrashic" }
                    option { "Philosophical" }
                    option { selected: true, "Historical-Critical" }
                    option { "Feminist" }
                    option { "Postcolonial" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Notes" }
                textarea {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box; min-height: 60px; resize: vertical;",
                    placeholder: "Additional notes...",
                    oninput: move |e| notes.set(e.value()),
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent); flex: 1;",
                h3 { style: "margin-top: 0; color: var(--qualia-accent); font-size: 0.9rem;", "QualiaDB Output" }
                div { style: "font-size: 0.8rem; color: var(--qualia-text-muted); display: flex; flex-direction: column; gap: 4px;",
                    div { "Period: {period()}" }
                    div { "Subfield: {subfield()}" }
                    div { "Language: {language()}" }
                    div { "Hermeneutic: {hermeneutic()}" }
                }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 8px;", "QualiaDB → knowledge graph | textual sieve | Allen Interval" }
            }
        }
    }
}
