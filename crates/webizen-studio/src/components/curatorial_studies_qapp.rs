use dioxus::prelude::*;

#[component]
pub fn CuratorialStudiesQapp() -> Element {
    let mut exhibition_type = use_signal(|| "Thematic".to_string());
    let mut curatorial_approach = use_signal(|| "Research-Led".to_string());
    let mut medium = use_signal(|| "Visual Art".to_string());
    let mut object_count = use_signal(|| 40u32);
    let mut venue_type = use_signal(|| "Institution".to_string());
    let mut critical_framework = use_signal(|| "Postmodern".to_string());
    let mut catalog_notes = use_signal(|| String::new());

    let exhibition_types = [
        "Solo Artist",
        "Group",
        "Thematic",
        "Survey",
        "Retrospective",
        "Site-Specific",
        "Virtual",
        "Community-Curated",
    ];
    let approaches = [
        "White Cube",
        "Relational Aesthetics",
        "Institutional Critique",
        "Decolonial",
        "Participatory",
        "Research-Led",
        "Archival",
    ];
    let media = [
        "Visual Art",
        "Performance",
        "Digital",
        "Sound",
        "New Media",
        "Mixed",
        "Interdisciplinary",
    ];
    let venues = [
        "Institution",
        "Gallery",
        "Public Space",
        "Online",
        "Pop-Up",
        "Museum",
    ];
    let frameworks = [
        "Postmodern",
        "Feminist",
        "Postcolonial",
        "Queer",
        "Disability",
        "Ecocritical",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Curatorial Studies" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Exhibition Type" }
                    select {
                        value: "{exhibition_type}",
                        onchange: move |e| exhibition_type.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in exhibition_types { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Curatorial Approach" }
                    select {
                        value: "{curatorial_approach}",
                        onchange: move |e| curatorial_approach.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in approaches { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Medium" }
                    select {
                        value: "{medium}",
                        onchange: move |e| medium.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in media { option { value: "{x}", "{x}" } }
                    }
                }
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
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Critical Framework" }
                    select {
                        value: "{critical_framework}",
                        onchange: move |e| critical_framework.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in frameworks { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Object Count" }
                    input {
                        r#type: "number",
                        value: "{object_count}",
                        oninput: move |e| object_count.set(e.value().parse().unwrap_or(40)),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
            }

            div {
                style: "flex: 1; display: flex; flex-direction: column;",
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Catalog Notes" }
                textarea {
                    value: "{catalog_notes}",
                    oninput: move |e| catalog_notes.set(e.value()),
                    style: "flex: 1; width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; resize: none; box-sizing: border-box; min-height: 80px;"
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 12px 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent);",
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{exhibition_type} | {curatorial_approach} | {venue_type} | Objects: {object_count}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → exhibition graph | critical theory sieve | collection management engine" }
            }
        }
    }
}
