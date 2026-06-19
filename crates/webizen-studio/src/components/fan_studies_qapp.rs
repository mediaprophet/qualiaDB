use dioxus::prelude::*;

#[component]
pub fn FanStudiesQapp() -> Element {
    let mut theoretical_lens = use_signal(|| "Participatory Culture (Jenkins)".to_string());
    let mut fandom_type = use_signal(|| "Fantasy".to_string());
    let mut platform = use_signal(|| "AO3".to_string());
    let mut fan_practice = use_signal(|| "Fanfiction".to_string());
    let mut community_size = use_signal(|| 10000u32);
    let mut engagement_level = use_signal(|| "Core".to_string());
    let mut notes = use_signal(|| String::new());

    let lenses = [
        "Participatory Culture (Jenkins)",
        "Textual Poaching",
        "Affect Theory",
        "Fan Labour",
        "Queer Fan Studies",
        "Fandom as Subculture",
        "Digital Ethnography",
    ];
    let fandom_types = [
        "Sci-Fi",
        "Fantasy",
        "Anime",
        "K-Pop",
        "Sports",
        "Gaming",
        "True Crime",
        "Celebrity",
    ];
    let platforms = [
        "Fanfiction.net",
        "AO3",
        "Tumblr",
        "Twitter",
        "Reddit",
        "Discord",
        "TikTok",
        "YouTube",
    ];
    let practices = [
        "Fanfiction",
        "Fanart",
        "Vidding",
        "Cosplay",
        "Fan Wiki",
        "Meta-Commentary",
        "Shipping",
    ];
    let engagement_levels = ["Lurker", "Casual", "Core", "BNF"];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Fan Studies" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
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
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Fandom Type" }
                    select {
                        value: "{fandom_type}",
                        onchange: move |e| fandom_type.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in fandom_types { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Platform" }
                    select {
                        value: "{platform}",
                        onchange: move |e| platform.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in platforms { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Fan Practice" }
                    select {
                        value: "{fan_practice}",
                        onchange: move |e| fan_practice.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in practices { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Engagement Level" }
                    select {
                        value: "{engagement_level}",
                        onchange: move |e| engagement_level.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in engagement_levels { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Community Size: {community_size}" }
                input {
                    r#type: "range",
                    min: "100",
                    max: "1000000",
                    step: "100",
                    value: "{community_size}",
                    oninput: move |e| community_size.set(e.value().parse().unwrap_or(10000)),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{fandom_type} | {platform} | {fan_practice} | {engagement_level} | n={community_size}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → fan culture engine | participatory media sieve | community anchor" }
            }
        }
    }
}
