use dioxus::prelude::*;

#[component]
pub fn PostCriticalPedagogyQapp() -> Element {
    let mut pedagogical_approach = use_signal(|| "Affirming".to_string());
    let mut educational_context = use_signal(|| "Higher Education".to_string());
    let mut critical_heritage = use_signal(|| "Freire".to_string());
    let mut post_critical_move = use_signal(|| "Affirmation over Critique".to_string());
    let mut student_wellbeing = use_signal(|| 50u32);
    let mut critical_engagement = use_signal(|| 50u32);
    let mut notes = use_signal(|| String::new());

    let approaches = [
        "Affirming",
        "Reparative",
        "Loving Critique",
        "Slow Pedagogy",
        "Mindful Education",
        "Affective Turn",
    ];
    let contexts = [
        "Higher Education",
        "K-12",
        "Informal",
        "Online",
        "Community",
    ];
    let heritages = ["Freire", "hooks", "Giroux", "Apple"];
    let moves = [
        "Affirmation over Critique",
        "Reparative Reading",
        "Generative Attention",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Post-Critical Pedagogy" }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Pedagogical Approach" }
                select {
                    value: "{pedagogical_approach}",
                    onchange: move |e| pedagogical_approach.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in approaches { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Educational Context" }
                select {
                    value: "{educational_context}",
                    onchange: move |e| educational_context.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in contexts { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Critical Heritage" }
                select {
                    value: "{critical_heritage}",
                    onchange: move |e| critical_heritage.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in heritages { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Post-Critical Move" }
                select {
                    value: "{post_critical_move}",
                    onchange: move |e| post_critical_move.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in moves { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Student Wellbeing: {student_wellbeing}" }
                input {
                    r#type: "range", min: "0", max: "100", value: "{student_wellbeing}",
                    oninput: move |e| student_wellbeing.set(e.value().parse().unwrap_or(50)),
                    style: "width: 100%; margin-top: 4px;"
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Critical Engagement: {critical_engagement}" }
                input {
                    r#type: "range", min: "0", max: "100", value: "{critical_engagement}",
                    oninput: move |e| critical_engagement.set(e.value().parse().unwrap_or(50)),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{pedagogical_approach} | {educational_context} | Wellbeing: {student_wellbeing} | Engagement: {critical_engagement}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → post-critical pedagogy engine | affirmation sieve | engagement anchor" }
            }
        }
    }
}
