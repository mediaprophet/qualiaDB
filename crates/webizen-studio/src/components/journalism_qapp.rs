use dioxus::prelude::*;

#[component]
pub fn JournalismQapp() -> Element {
    let mut journalism_type = use_signal(|| "Investigative".to_string());
    let mut medium = use_signal(|| "Online".to_string());
    let mut news_value = use_signal(|| "Impact".to_string());
    let mut ethical_principle = use_signal(|| "Truth".to_string());
    let mut source_type = use_signal(|| "Document".to_string());
    let mut verification_level = use_signal(|| 70u32);
    let mut story_pitch = use_signal(|| String::new());

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box; overflow-y: auto;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Journalism QApp" }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Journalism Type" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| journalism_type.set(e.value()),
                    option { selected: true, "Investigative" }
                    option { "Data Journalism" }
                    option { "Broadcast" }
                    option { "Print" }
                    option { "Digital" }
                    option { "Science" }
                    option { "Environmental" }
                    option { "War Correspondence" }
                    option { "Solutions Journalism" }
                    option { "Advocacy" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Medium" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| medium.set(e.value()),
                    option { "Newspaper" }
                    option { "Television" }
                    option { "Radio" }
                    option { selected: true, "Online" }
                    option { "Social Media" }
                    option { "Podcast" }
                    option { "Newsletter" }
                    option { "Documentary" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "News Value" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| news_value.set(e.value()),
                    option { "Timeliness" }
                    option { "Proximity" }
                    option { "Prominence" }
                    option { "Conflict" }
                    option { "Human Interest" }
                    option { "Unusual" }
                    option { selected: true, "Impact" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Ethical Principle" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| ethical_principle.set(e.value()),
                    option { selected: true, "Truth" }
                    option { "Independence" }
                    option { "Minimise Harm" }
                    option { "Accountability" }
                    option { "Proportionality" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Source Type" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| source_type.set(e.value()),
                    option { "Official" }
                    option { "Whistleblower" }
                    option { "Expert" }
                    option { "Eyewitness" }
                    option { selected: true, "Document" }
                    option { "Data" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Verification Level (0–100): {verification_level()}%" }
                input {
                    r#type: "range",
                    min: "0",
                    max: "100",
                    step: "1",
                    value: "{verification_level()}",
                    style: "width: 100%; box-sizing: border-box; accent-color: if verification_level() >= 70 { \"var(--qualia-accent)\" } else { \"var(--qualia-accent)\" };",
                    oninput: move |e| verification_level.set(e.value().parse().unwrap_or(70)),
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Story Pitch" }
                textarea {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box; min-height: 80px; resize: vertical;",
                    placeholder: "Describe your story pitch...",
                    oninput: move |e| story_pitch.set(e.value()),
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent); flex: 1;",
                h3 { style: "margin-top: 0; color: var(--qualia-accent); font-size: 0.9rem;", "QualiaDB Output" }
                div { style: "font-size: 0.8rem; color: var(--qualia-text-muted); display: flex; flex-direction: column; gap: 4px;",
                    div { "Type: {journalism_type()}" }
                    div { "Medium: {medium()}" }
                    div { "Ethics: {ethical_principle()}" }
                    div { style: "color: if verification_level() >= 70 { \"var(--qualia-accent)\" } else { \"var(--qualia-accent)\" };", "Verification: {verification_level()}%" }
                }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 8px;", "QualiaDB → epistemic logic | provenance graph | discourse sieve" }
            }
        }
    }
}
