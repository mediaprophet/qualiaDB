use dioxus::prelude::*;

#[component]
pub fn PublicHistoryQapp() -> Element {
    let mut venue_type = use_signal(|| "Museum".to_string());
    let mut audience = use_signal(|| "General Public".to_string());
    let mut interpretation_approach = use_signal(|| "Educational".to_string());
    let mut access_model = use_signal(|| "Free".to_string());
    let mut engagement_depth = use_signal(|| 60u32);
    let mut notes = use_signal(|| String::new());

    let venues = [
        "Museum",
        "Historic Site",
        "Archive",
        "Oral History Project",
        "Heritage Trail",
        "Digital Platform",
        "Film/Media",
    ];
    let audiences = [
        "General Public",
        "Schools",
        "Scholars",
        "Policymakers",
        "Communities of Descent",
    ];
    let approaches = [
        "Curatorial",
        "Participatory",
        "Critical",
        "Commemorative",
        "Educational",
    ];
    let access_models = ["Free", "Ticketed", "Online", "Mixed"];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",

            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Public History" }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Venue Type" }
                select {
                    value: "{venue_type}",
                    onchange: move |e| venue_type.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in venues { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Audience" }
                select {
                    value: "{audience}",
                    onchange: move |e| audience.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in audiences { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Interpretation Approach" }
                select {
                    value: "{interpretation_approach}",
                    onchange: move |e| interpretation_approach.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in approaches { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Access Model" }
                select {
                    value: "{access_model}",
                    onchange: move |e| access_model.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in access_models { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Engagement Depth: {engagement_depth}%" }
                input {
                    r#type: "range", min: "0", max: "100",
                    value: "{engagement_depth}",
                    oninput: move |e| engagement_depth.set(e.value().parse().unwrap_or(60)),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{venue_type} | {audience} | {interpretation_approach} | {access_model} | Engagement: {engagement_depth}%" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → knowledge engine | sieve | anchor" }
            }
        }
    }
}
