use dioxus::prelude::*;

#[component]
pub fn EthicsQapp() -> Element {
    let mut framework = use_signal(|| "Deontology".to_string());
    let mut deontic_op = use_signal(|| "Obligation".to_string());
    let mut dilemma = use_signal(|| String::new());
    let mut stakeholder_a = use_signal(|| String::new());
    let mut stakeholder_b = use_signal(|| String::new());
    let mut epistemic_certainty = use_signal(|| 70u32);
    let mut moral_weight_a = use_signal(|| 50u32);

    let frameworks = [
        "Deontology (Kant)",
        "Consequentialism (Mill)",
        "Virtue Ethics (Aristotle)",
        "Care Ethics (Noddings)",
        "Contractarianism (Rawls)",
        "Natural Law",
        "Pragmatic Ethics",
        "Discourse Ethics (Habermas)",
    ];
    let deontic_ops = [
        "Obligation (O)",
        "Permission (P)",
        "Prohibition (F)",
        "Waiver",
        "Immunity",
        "Claim-right",
        "Power",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Ethics — Deontic Logic Simulator" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Ethical Framework" }
                    select {
                        value: "{framework}",
                        onchange: move |e| framework.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for f in frameworks { option { value: "{f}", "{f}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Deontic Operator" }
                    select {
                        value: "{deontic_op}",
                        onchange: move |e| deontic_op.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for op in deontic_ops { option { value: "{op}", "{op}" } }
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Moral Dilemma / Proposition" }
                textarea {
                    value: "{dilemma}",
                    oninput: move |e| dilemma.set(e.value()),
                    rows: "4",
                    placeholder: "State the ethical situation or proposition to analyse…",
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; resize: vertical; box-sizing: border-box;"
                }
            }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Stakeholder A" }
                    input {
                        r#type: "text", placeholder: "e.g. Individual patient, citizen…",
                        value: "{stakeholder_a}",
                        oninput: move |e| stakeholder_a.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Stakeholder B" }
                    input {
                        r#type: "text", placeholder: "e.g. Society, institution, future generation…",
                        value: "{stakeholder_b}",
                        oninput: move |e| stakeholder_b.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Moral Weight — A vs B: {moral_weight_a}% / {100u32.saturating_sub(moral_weight_a())}%" }
                input {
                    r#type: "range", min: "0", max: "100",
                    value: "{moral_weight_a}",
                    oninput: move |e| moral_weight_a.set(e.value().parse().unwrap_or(50)),
                    style: "width: 100%; margin-top: 4px;"
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Epistemic Certainty of Premises: {epistemic_certainty}%" }
                input {
                    r#type: "range", min: "0", max: "100",
                    value: "{epistemic_certainty}",
                    oninput: move |e| epistemic_certainty.set(e.value().parse().unwrap_or(70)),
                    style: "width: 100%; margin-top: 4px;"
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent); flex: 1;",
                h3 { style: "margin-top: 0; color: var(--qualia-accent); font-size: 0.9rem;", "Deontic Logic Output" }
                div { style: "font-family: monospace; font-size: 0.9rem; color: var(--qualia-text);",
                    "{deontic_op}({framework}): [{epistemic_certainty}% confidence]"
                }
                div { style: "font-size: 0.8rem; color: var(--qualia-text-muted); margin-top: 8px;",
                    "Stakeholder A ({stakeholder_a}): {moral_weight_a}% weight"
                }
                div { style: "font-size: 0.8rem; color: var(--qualia-text-muted);",
                    "Stakeholder B ({stakeholder_b}): {100u32.saturating_sub(moral_weight_a())}% weight"
                }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 8px;", "QualiaDB → deontic_logic_editor | epistemic certainty engine" }
            }
        }
    }
}
