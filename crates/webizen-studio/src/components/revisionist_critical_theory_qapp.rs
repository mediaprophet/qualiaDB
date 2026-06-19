use dioxus::prelude::*;

#[component]
pub fn RevisionistCriticalTheoryQapp() -> Element {
    let mut revisionist_move = use_signal(|| "Updating Marxism".to_string());
    let mut revised_tradition = use_signal(|| "Frankfurt School".to_string());
    let mut revision_rationale = use_signal(|| "Political Failure".to_string());
    let mut theoretical_fidelity = use_signal(|| 50u32);
    let mut innovation = use_signal(|| 50u32);
    let mut notes = use_signal(|| String::new());

    let moves = [
        "Updating Marxism",
        "Post-Critical Turn",
        "Anti-Theory",
        "Affirmative Turn",
        "Pragmatist Revision",
        "Aesthetic Turn",
    ];
    let traditions = [
        "Frankfurt School",
        "Foucauldian",
        "Postcolonial",
        "Feminist",
        "Queer",
    ];
    let rationales = [
        "Political Failure",
        "Theoretical Inconsistency",
        "Affective Deficit",
        "Cultural Shift",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Revisionist Critical Theory" }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Revisionist Move" }
                select {
                    value: "{revisionist_move}",
                    onchange: move |e| revisionist_move.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in moves { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Revised Tradition" }
                select {
                    value: "{revised_tradition}",
                    onchange: move |e| revised_tradition.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in traditions { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Revision Rationale" }
                select {
                    value: "{revision_rationale}",
                    onchange: move |e| revision_rationale.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in rationales { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Theoretical Fidelity: {theoretical_fidelity}" }
                input {
                    r#type: "range", min: "0", max: "100", value: "{theoretical_fidelity}",
                    oninput: move |e| theoretical_fidelity.set(e.value().parse().unwrap_or(50)),
                    style: "width: 100%; margin-top: 4px;"
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Innovation: {innovation}" }
                input {
                    r#type: "range", min: "0", max: "100", value: "{innovation}",
                    oninput: move |e| innovation.set(e.value().parse().unwrap_or(50)),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{revisionist_move} | {revised_tradition} | Fidelity: {theoretical_fidelity} | Innovation: {innovation}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → revisionist theory engine | revision sieve | innovation anchor" }
            }
        }
    }
}
