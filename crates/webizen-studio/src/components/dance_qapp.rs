use dioxus::prelude::*;

#[component]
pub fn DanceQapp() -> Element {
    let mut style = use_signal(|| "Contemporary".to_string());
    let mut tempo_bpm = use_signal(|| 120u32);
    let mut level = use_signal(|| "Middle".to_string());
    let mut direction = use_signal(|| "Forward".to_string());
    let mut duration_beats = use_signal(|| 8u32);
    let mut spatial_path = use_signal(|| "Linear".to_string());
    let mut dynamic_quality = use_signal(|| "Sustained".to_string());
    let mut notation_note = use_signal(|| String::new());

    let styles = [
        "Ballet",
        "Contemporary",
        "Jazz",
        "Hip-Hop",
        "Flamenco",
        "Kathak",
        "Butoh",
        "Folk / Traditional",
        "Ballroom",
        "Tap",
    ];
    let levels = ["Low (Floor)", "Middle (Standing)", "High (Elevated)"];
    let directions = [
        "Forward",
        "Backward",
        "Sideways Left",
        "Sideways Right",
        "Diagonal Forward-Left",
        "Diagonal Forward-Right",
        "Rotate CW",
        "Rotate CCW",
    ];
    let paths = [
        "Linear", "Circular", "Zigzag", "Spiral", "Figure-8", "Random",
    ];
    let dynamics = [
        "Sustained",
        "Sudden",
        "Strong",
        "Light",
        "Direct",
        "Indirect",
        "Bound",
        "Free",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Dance — Movement Notation Studio" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Dance Style" }
                    select {
                        value: "{style}",
                        onchange: move |e| style.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for s in styles { option { value: "{s}", "{s}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Tempo (BPM): {tempo_bpm}" }
                    input {
                        r#type: "range", min: "40", max: "240",
                        value: "{tempo_bpm}",
                        oninput: move |e| tempo_bpm.set(e.value().parse().unwrap_or(120)),
                        style: "width: 100%; margin-top: 10px;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Duration (beats): {duration_beats}" }
                    input {
                        r#type: "number", min: "1", max: "256",
                        value: "{duration_beats}",
                        oninput: move |e| duration_beats.set(e.value().parse().unwrap_or(8)),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
            }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Level (Laban)" }
                    select {
                        value: "{level}",
                        onchange: move |e| level.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for l in levels { option { value: "{l}", "{l}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Direction" }
                    select {
                        value: "{direction}",
                        onchange: move |e| direction.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for d in directions { option { value: "{d}", "{d}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Dynamic Quality" }
                    select {
                        value: "{dynamic_quality}",
                        onchange: move |e| dynamic_quality.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for d in dynamics { option { value: "{d}", "{d}" } }
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Spatial Path" }
                select {
                    value: "{spatial_path}",
                    onchange: move |e| spatial_path.set(e.value()),
                    style: "width: 40%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                    for p in paths { option { value: "{p}", "{p}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Choreographic Notes / Labanotation Annotations" }
                textarea {
                    value: "{notation_note}",
                    oninput: move |e| notation_note.set(e.value()),
                    rows: "3",
                    placeholder: "Describe phrase, motif, or spatial relationship…",
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; resize: vertical; box-sizing: border-box;"
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent);",
                h3 { style: "margin-top: 0; color: var(--qualia-accent); font-size: 0.9rem;", "Allen Interval — Phrase Timing" }
                div {
                    style: "display: flex; gap: 12px; flex-wrap: wrap; font-size: 0.85rem; color: var(--qualia-text-muted);",
                    span { "Style: {style}" }
                    span { "Tempo: {tempo_bpm} BPM" }
                    span { "Duration: {duration_beats} beats" }
                    span { "Level: {level}" }
                    span { "Dir: {direction}" }
                    span { "Dynamic: {dynamic_quality}" }
                    span { "Path: {spatial_path}" }
                }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 8px;", "QualiaDB → Allen Interval Algebra | movement ontology sieve" }
            }
        }
    }
}
