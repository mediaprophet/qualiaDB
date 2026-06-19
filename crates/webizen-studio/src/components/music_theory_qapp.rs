use dioxus::prelude::*;

#[component]
pub fn MusicTheoryQapp() -> Element {
    let mut root_note = use_signal(|| "C".to_string());
    let mut scale = use_signal(|| "Major".to_string());
    let mut chord_type = use_signal(|| "Triad".to_string());
    let mut inversion = use_signal(|| "Root Position".to_string());
    let mut progression = use_signal(|| "I–IV–V–I".to_string());
    let mut counterpoint_rule = use_signal(|| "First Species".to_string());
    let mut voice_leading_notes = use_signal(|| String::new());

    let notes = [
        "C", "C#/Db", "D", "D#/Eb", "E", "F", "F#/Gb", "G", "G#/Ab", "A", "A#/Bb", "B",
    ];
    let scales = [
        "Major",
        "Natural Minor",
        "Harmonic Minor",
        "Melodic Minor",
        "Dorian",
        "Phrygian",
        "Lydian",
        "Mixolydian",
        "Locrian",
        "Whole Tone",
        "Octatonic (Diminished)",
        "Pentatonic Major",
        "Pentatonic Minor",
        "Blues",
    ];
    let chords = [
        "Triad",
        "Seventh Chord",
        "Ninth Chord",
        "Eleventh Chord",
        "Thirteenth Chord",
        "Suspended",
        "Added 6th",
        "Augmented",
        "Diminished",
        "Half-Diminished",
    ];
    let inversions = [
        "Root Position",
        "First Inversion",
        "Second Inversion",
        "Third Inversion",
    ];
    let progressions = [
        "I–IV–V–I",
        "I–V–vi–IV",
        "ii–V–I (Jazz)",
        "I–VI–IV–V",
        "I–IV–I–V",
        "iii–vi–ii–V",
        "I–bVII–IV–I",
        "Custom…",
    ];
    let cp_rules = [
        "First Species (1:1)",
        "Second Species (2:1)",
        "Third Species (4:1)",
        "Fourth Species (Suspension)",
        "Fifth Species (Florid)",
        "Free Counterpoint",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Music Theory" }

            div {
                style: "display: grid; grid-template-columns: repeat(4, 1fr); gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Root Note" }
                    select {
                        value: "{root_note}",
                        onchange: move |e| root_note.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for n in notes { option { value: "{n}", "{n}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Scale / Mode" }
                    select {
                        value: "{scale}",
                        onchange: move |e| scale.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for s in scales { option { value: "{s}", "{s}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Chord Type" }
                    select {
                        value: "{chord_type}",
                        onchange: move |e| chord_type.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for c in chords { option { value: "{c}", "{c}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Inversion" }
                    select {
                        value: "{inversion}",
                        onchange: move |e| inversion.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for i in inversions { option { value: "{i}", "{i}" } }
                    }
                }
            }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Chord Progression" }
                    select {
                        value: "{progression}",
                        onchange: move |e| progression.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for p in progressions { option { value: "{p}", "{p}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Counterpoint Species" }
                    select {
                        value: "{counterpoint_rule}",
                        onchange: move |e| counterpoint_rule.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for r in cp_rules { option { value: "{r}", "{r}" } }
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Voice Leading Notes" }
                textarea {
                    value: "{voice_leading_notes}",
                    oninput: move |e| voice_leading_notes.set(e.value()),
                    rows: "2",
                    placeholder: "Describe voice crossing, parallel fifths/octaves, resolution of tendency tones…",
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; resize: vertical; box-sizing: border-box;"
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent); flex: 1;",
                h3 { style: "margin-top: 0; color: var(--qualia-accent); font-size: 0.9rem;", "Harmonic Analysis" }
                div {
                    style: "font-size: 0.9rem; color: var(--qualia-text); margin-bottom: 8px;",
                    "{root_note} {scale} — {chord_type} ({inversion})"
                }
                div {
                    style: "background: var(--qualia-bg); padding: 10px; border-radius: 6px; font-size: 0.85rem;",
                    "Progression: {progression}"
                }
                div {
                    style: "margin-top: 8px; font-size: 0.8rem; color: var(--qualia-text-muted);",
                    "Counterpoint: {counterpoint_rule}"
                }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 8px;", "QualiaDB → harmonic graph engine | constraint solver | Allen Interval rhythm analysis" }
            }
        }
    }
}
