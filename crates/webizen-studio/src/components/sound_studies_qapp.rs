use dioxus::prelude::*;

#[component]
pub fn SoundStudiesQapp() -> Element {
    let mut theoretical_approach = use_signal(|| "Soundscape Studies".to_string());
    let mut sound_type = use_signal(|| "Music".to_string());
    let mut frequency_hz = use_signal(|| 440.0f64);
    let mut decibel_spl = use_signal(|| 70.0f64);
    let mut temporal_mode = use_signal(|| "Recorded".to_string());
    let mut cultural_context = use_signal(|| String::new());
    let mut analysis_notes = use_signal(|| String::new());

    let approaches = [
        "Acoustic Ecology",
        "Soundscape Studies",
        "Acousmatic",
        "Phonography",
        "Noise Studies",
        "Voice Studies",
        "Sound Art",
        "Bioacoustics",
        "Auditory Culture",
    ];
    let sound_types = [
        "Music",
        "Speech",
        "Noise",
        "Ambient",
        "Urban Soundscape",
        "Natural Soundscape",
        "Electronic",
        "Silence",
    ];
    let temporal_modes = [
        "Real-Time",
        "Recorded",
        "Archival",
        "Synthetic",
        "Spatialized",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Sound Studies" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Theoretical Approach" }
                    select {
                        value: "{theoretical_approach}",
                        onchange: move |e| theoretical_approach.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in approaches { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Sound Type" }
                    select {
                        value: "{sound_type}",
                        onchange: move |e| sound_type.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in sound_types { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Temporal Mode" }
                    select {
                        value: "{temporal_mode}",
                        onchange: move |e| temporal_mode.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in temporal_modes { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Frequency Hz: {frequency_hz:.1}" }
                    input {
                        r#type: "range",
                        min: "20",
                        max: "20000",
                        step: "1",
                        value: "{frequency_hz}",
                        oninput: move |e| frequency_hz.set(e.value().parse().unwrap_or(440.0)),
                        style: "width: 100%; margin-top: 4px;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "dB SPL: {decibel_spl:.1}" }
                    input {
                        r#type: "range",
                        min: "0",
                        max: "130",
                        step: "0.5",
                        value: "{decibel_spl}",
                        oninput: move |e| decibel_spl.set(e.value().parse().unwrap_or(70.0)),
                        style: "width: 100%; margin-top: 4px;"
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Cultural Context" }
                input {
                    r#type: "text",
                    value: "{cultural_context}",
                    oninput: move |e| cultural_context.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                }
            }

            div {
                style: "flex: 1; display: flex; flex-direction: column;",
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Analysis Notes" }
                textarea {
                    value: "{analysis_notes}",
                    oninput: move |e| analysis_notes.set(e.value()),
                    style: "flex: 1; width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; resize: none; box-sizing: border-box; min-height: 60px;"
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 12px 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent);",
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{theoretical_approach} | {sound_type} | {frequency_hz:.1}Hz | {decibel_spl:.1}dB SPL" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → Allen Interval engine | spectral analysis sieve | phenomenological graph" }
            }
        }
    }
}
