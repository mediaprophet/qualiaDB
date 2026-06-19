use dioxus::prelude::*;

#[component]
pub fn MusicPerformanceQapp() -> Element {
    let mut instrument = use_signal(|| "Piano".to_string());
    let mut tempo_bpm = use_signal(|| 120u32);
    let mut dynamic = use_signal(|| "mf".to_string());
    let mut articulation = use_signal(|| "Legato".to_string());
    let mut expression_marking = use_signal(|| String::new());
    let mut rubato = use_signal(|| 0i32);
    let mut edition = use_signal(|| String::new());
    let mut interpretation_notes = use_signal(|| String::new());

    let instruments = [
        "Piano",
        "Violin",
        "Viola",
        "Cello",
        "Double Bass",
        "Flute",
        "Oboe",
        "Clarinet",
        "Bassoon",
        "Trumpet",
        "French Horn",
        "Trombone",
        "Tuba",
        "Harp",
        "Guitar",
        "Harpsichord",
        "Organ",
        "Voice (Soprano)",
        "Voice (Mezzo)",
        "Voice (Tenor)",
        "Voice (Bass)",
    ];
    let dynamics = [
        "ppp", "pp", "p", "mp", "mf", "f", "ff", "fff", "fp", "sfz", "cresc.", "dim.",
    ];
    let articulations = [
        "Legato",
        "Staccato",
        "Staccatissimo",
        "Tenuto",
        "Marcato",
        "Portato",
        "Spiccato (strings)",
        "Col legno (strings)",
        "Sul ponticello",
        "Pizzicato",
        "Flutter tongue (winds)",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Music Performance" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Instrument / Voice Type" }
                    select {
                        value: "{instrument}",
                        onchange: move |e| instrument.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for i in instruments { option { value: "{i}", "{i}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Articulation" }
                    select {
                        value: "{articulation}",
                        onchange: move |e| articulation.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for a in articulations { option { value: "{a}", "{a}" } }
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Tempo: {tempo_bpm} BPM" }
                input {
                    r#type: "range", min: "20", max: "300",
                    value: "{tempo_bpm}",
                    oninput: move |e| tempo_bpm.set(e.value().parse().unwrap_or(120)),
                    style: "width: 100%; margin-top: 4px;"
                }
            }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Dynamic Level" }
                    select {
                        value: "{dynamic}",
                        onchange: move |e| dynamic.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for d in dynamics { option { value: "{d}", "{d}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Rubato / Flexibility: {rubato:+}%" }
                    input {
                        r#type: "range", min: "-30", max: "30",
                        value: "{rubato}",
                        oninput: move |e| rubato.set(e.value().parse().unwrap_or(0)),
                        style: "width: 100%; margin-top: 12px;"
                    }
                }
            }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Expression Marking" }
                    input {
                        r#type: "text", placeholder: "e.g. espressivo, con fuoco, dolce…",
                        value: "{expression_marking}",
                        oninput: move |e| expression_marking.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Edition / Urtext Source" }
                    input {
                        r#type: "text", placeholder: "e.g. Bärenreiter BA 4711, Henle 461…",
                        value: "{edition}",
                        oninput: move |e| edition.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
            }

            div {
                style: "flex: 1; display: flex; flex-direction: column;",
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Interpretation Notes" }
                textarea {
                    value: "{interpretation_notes}",
                    oninput: move |e| interpretation_notes.set(e.value()),
                    style: "flex: 1; width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; resize: none; box-sizing: border-box; min-height: 60px;"
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 12px 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent); display: flex; gap: 16px; flex-wrap: wrap;",
                span { style: "font-size: 0.8rem; color: var(--qualia-text); font-weight: bold;", "{instrument}" }
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{tempo_bpm} BPM" }
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{dynamic}" }
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{articulation}" }
                if rubato() != 0 { span { style: "font-size: 0.8rem; color: var(--qualia-accent);", "Rubato: {rubato:+}%" } }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); width: 100%;", "QualiaDB → Allen Interval rhythm engine | expressive parameter sieve" }
            }
        }
    }
}
