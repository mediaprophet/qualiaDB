use dioxus::prelude::*;

#[component]
pub fn MemoryStudiesQapp() -> Element {
    let mut memory_type = use_signal(|| "Collective".to_string());
    let mut site = use_signal(|| "Museum".to_string());
    let mut temporality = use_signal(|| "Living Memory".to_string());
    let mut theoretical_frame = use_signal(|| "Halbwachs".to_string());
    let mut fidelity = use_signal(|| 60u32);
    let mut controversy = use_signal(|| 40u32);
    let mut notes = use_signal(|| String::new());

    let memory_types = [
        "Individual",
        "Collective",
        "Cultural",
        "Postmemory",
        "Counter-Memory",
        "Prosthetic",
    ];
    let sites = [
        "Museum",
        "Monument",
        "Archive",
        "Ritual",
        "Media",
        "Literature",
        "Urban Space",
    ];
    let temporalities = ["Living Memory", "Post-Memory", "Deep Past", "Future Memory"];
    let frames = ["Halbwachs", "Nora", "Assmann", "Hirsch", "Butler"];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",

            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Memory Studies" }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Memory Type" }
                select {
                    value: "{memory_type}",
                    onchange: move |e| memory_type.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in memory_types { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Site" }
                select {
                    value: "{site}",
                    onchange: move |e| site.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in sites { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Temporality" }
                select {
                    value: "{temporality}",
                    onchange: move |e| temporality.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in temporalities { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Theoretical Frame" }
                select {
                    value: "{theoretical_frame}",
                    onchange: move |e| theoretical_frame.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in frames { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Fidelity: {fidelity}%" }
                input {
                    r#type: "range", min: "0", max: "100",
                    value: "{fidelity}",
                    oninput: move |e| fidelity.set(e.value().parse().unwrap_or(60)),
                    style: "width: 100%; margin-top: 4px;"
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Controversy: {controversy}%" }
                input {
                    r#type: "range", min: "0", max: "100",
                    value: "{controversy}",
                    oninput: move |e| controversy.set(e.value().parse().unwrap_or(40)),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{memory_type} | {site} | {temporality} | {theoretical_frame} | Fidelity: {fidelity}% | Controversy: {controversy}%" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → knowledge engine | sieve | anchor" }
            }
        }
    }
}
