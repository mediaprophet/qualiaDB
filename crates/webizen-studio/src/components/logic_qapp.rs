use dioxus::prelude::*;

#[component]
pub fn LogicQapp() -> Element {
    let mut logic_type = use_signal(|| "First-Order Predicate".to_string());
    let mut proof_system = use_signal(|| "Natural Deduction".to_string());
    let mut formula = use_signal(|| String::new());
    let mut inference_rule = use_signal(|| "Modus Ponens".to_string());
    let mut validity_status = use_signal(|| "Unknown".to_string());
    let mut notes = use_signal(|| String::new());

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Logic QApp" }

            div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Logic Type" }
                    select {
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        onchange: move |e| logic_type.set(e.value()),
                        option { "Classical Propositional" }
                        option { "First-Order Predicate" }
                        option { "Modal (S4/S5)" }
                        option { "Temporal (LTL/CTL)" }
                        option { "Deontic" }
                        option { "Epistemic" }
                        option { "Intuitionistic" }
                        option { "Paraconsistent" }
                        option { "Fuzzy" }
                        option { "Default / Non-monotonic" }
                        option { "Description Logic" }
                        option { "Higher-Order Logic" }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Proof System" }
                    select {
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        onchange: move |e| proof_system.set(e.value()),
                        option { "Natural Deduction" }
                        option { "Sequent Calculus" }
                        option { "Hilbert System" }
                        option { "Resolution" }
                        option { "Tableau" }
                        option { "Type Theory" }
                        option { "DPLL / CDCL" }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Inference Rule" }
                    select {
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        onchange: move |e| inference_rule.set(e.value()),
                        option { "Modus Ponens" }
                        option { "Modus Tollens" }
                        option { "Universal Instantiation" }
                        option { "Existential Generalisation" }
                        option { "De Morgan" }
                        option { "Cut" }
                        option { "Conjunction Introduction" }
                        option { "Disjunction Elimination" }
                        option { "Reductio ad Absurdum" }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Validity Status" }
                    select {
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        onchange: move |e| validity_status.set(e.value()),
                        option { "Valid" }
                        option { "Invalid" }
                        option { "Unknown" }
                        option { "Undecidable" }
                        option { "Satisfiable" }
                        option { "Unsatisfiable" }
                        option { "Tautology" }
                        option { "Contingent" }
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Formula / Sequent" }
                input {
                    r#type: "text",
                    value: "{formula}",
                    placeholder: "e.g. ∀x(P(x) → Q(x)), □(A → B) → (□A → □B)...",
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box; font-size: 0.9rem;",
                    oninput: move |e| formula.set(e.value()),
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Notes" }
                textarea {
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; min-height: 60px; box-sizing: border-box; resize: vertical;",
                    placeholder: "Proof steps, semantic tableaux, countermodels, decidability arguments...",
                    oninput: move |e| notes.set(e.value()),
                    "{notes}"
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent); flex: 1;",
                h3 { style: "margin-top: 0; color: var(--qualia-accent); font-size: 0.9rem;", "Output / Analysis" }
                div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 8px; font-size: 0.8rem;",
                    div { style: "color: var(--qualia-text-muted);", "Logic Type:" }
                    div { style: "color: var(--qualia-text);", "{logic_type}" }
                    div { style: "color: var(--qualia-text-muted);", "Proof System:" }
                    div { style: "color: var(--qualia-text);", "{proof_system}" }
                    div { style: "color: var(--qualia-text-muted);", "Inference Rule:" }
                    div { style: "color: var(--qualia-text);", "{inference_rule}" }
                    div { style: "color: var(--qualia-text-muted);", "Status:" }
                    div { style: "color: var(--qualia-text);", "{validity_status}" }
                }
                if !formula().is_empty() {
                    div {
                        style: "margin-top: 8px; padding: 8px; background: var(--qualia-bg); border-radius: 4px; border: 1px solid var(--qualia-border);",
                        div { style: "font-size: 0.75rem; color: var(--qualia-text-muted);", "Formula:" }
                        div { style: "font-size: 0.85rem; color: var(--qualia-accent); margin-top: 4px; word-break: break-all;", "{formula}" }
                    }
                }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 12px; border-top: 1px solid var(--qualia-border); padding-top: 8px;",
                    "QualiaDB → epistemic logic engine | deontic logic | resolution theorem prover"
                }
            }
        }
    }
}
