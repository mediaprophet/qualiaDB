use dioxus::prelude::*;

#[component]
pub fn PhilosophyQapp() -> Element {
    let mut argument_type = use_signal(|| "Deductive".to_string());
    let mut school = use_signal(|| "Analytic".to_string());
    let mut premise_1 = use_signal(|| String::new());
    let mut premise_2 = use_signal(|| String::new());
    let mut conclusion = use_signal(|| String::new());
    let mut modal_op = use_signal(|| "Necessarily".to_string());
    let mut certainty = use_signal(|| 80u32);

    let arg_types = [
        "Deductive",
        "Inductive",
        "Abductive",
        "Transcendental",
        "Reductio ad Absurdum",
        "Dilemma",
        "Sorites",
    ];
    let schools = [
        "Analytic",
        "Continental",
        "Pragmatism",
        "Phenomenology",
        "Existentialism",
        "Stoicism",
        "Epicureanism",
        "Neo-Platonism",
        "Kantian",
        "Hegelian",
        "Marxist",
        "Post-Structuralist",
    ];
    let modals = [
        "Necessarily",
        "Possibly",
        "Contingently",
        "Impossibly",
        "Probably",
        "Obligatorily (Deontic)",
        "Permissibly (Deontic)",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Philosophy — Argument Analyser" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Argument Type" }
                    select {
                        value: "{argument_type}",
                        onchange: move |e| argument_type.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for t in arg_types { option { value: "{t}", "{t}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "School of Thought" }
                    select {
                        value: "{school}",
                        onchange: move |e| school.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for s in schools { option { value: "{s}", "{s}" } }
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Premise 1 (P₁)" }
                input {
                    r#type: "text", placeholder: "All men are mortal.",
                    value: "{premise_1}",
                    oninput: move |e| premise_1.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Premise 2 (P₂)" }
                input {
                    r#type: "text", placeholder: "Socrates is a man.",
                    value: "{premise_2}",
                    oninput: move |e| premise_2.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Conclusion (C)" }
                input {
                    r#type: "text", placeholder: "Therefore, Socrates is mortal.",
                    value: "{conclusion}",
                    oninput: move |e| conclusion.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                }
            }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Modal Operator" }
                    select {
                        value: "{modal_op}",
                        onchange: move |e| modal_op.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for m in modals { option { value: "{m}", "{m}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Epistemic Certainty: {certainty}%" }
                    input {
                        r#type: "range", min: "0", max: "100",
                        value: "{certainty}",
                        oninput: move |e| certainty.set(e.value().parse().unwrap_or(80)),
                        style: "width: 100%; margin-top: 12px;"
                    }
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent); flex: 1;",
                h3 { style: "margin-top: 0; color: var(--qualia-accent); font-size: 0.9rem;", "Logical Form" }
                div {
                    style: "font-family: monospace; font-size: 0.9rem; background: var(--qualia-bg); padding: 12px; border-radius: 6px; white-space: pre-wrap; color: var(--qualia-text);",
                    if !premise_1().is_empty() { "P₁: {premise_1}\n" }
                    if !premise_2().is_empty() { "P₂: {premise_2}\n" }
                    if !conclusion().is_empty() { "∴  C: {conclusion}" }
                    if premise_1().is_empty() && premise_2().is_empty() && conclusion().is_empty() { "(enter premises above)" }
                }
                div {
                    style: "margin-top: 8px; font-size: 0.8rem; color: var(--qualia-text-muted);",
                    "{argument_type} | {school} | {modal_op} [{certainty}%]"
                }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 8px;", "QualiaDB → epistemic logic engine | deontic_logic_editor | modal solver" }
            }
        }
    }
}
