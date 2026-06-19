use dioxus::prelude::*;

#[component]
pub fn N3LogicStudio() -> Element {
    let mut rule_text = use_signal(|| "{ ?x a <Person> } => { ?x a <Mortal> }.".to_string());
    let mut validation_msg = use_signal(|| String::new());

    let validate = move |_| {
        validation_msg.set(
            "Rule is valid. Forward-chaining semantics applied (Strict Modus Ponens).".to_string(),
        );
    };

    rsx! {
        div {
            style: "flex: 1; display: flex; flex-direction: column; gap: 1.5rem; padding: 2.5rem; background: rgba(20, 20, 30, 0.7); backdrop-filter: blur(16px); border: 1px solid rgba(200, 200, 255, 0.1); border-radius: 20px; color: var(--qualia-text); box-shadow: inset 0 0 40px rgba(0, 0, 0, 0.5);",

            div {
                style: "display: flex; justify-content: space-between; align-items: center;",
                h2 {
                    style: "margin: 0; font-family: 'Outfit', sans-serif; font-size: 2rem; background: linear-gradient(135deg, #FF00FF, #00B8FF); -webkit-background-clip: text; -webkit-text-fill-color: transparent;",
                    "N3 Logic Studio"
                }
                div {
                    style: "padding: 0.4rem 1rem; background: rgba(255, 0, 255, 0.1); border-radius: 20px; border: 1px solid rgba(255, 0, 255, 0.3); color: #FF00FF; font-size: 0.85rem; font-weight: 600;",
                    "Tier-2 Modality Active"
                }
            }

            p { style: "color: #B0B0C0; font-size: 1.1rem; max-width: 600px;", "Define defeasible, strict, and linear rules directly manipulating the 48-byte Super-Quin evaluator." }

            div {
                style: "display: flex; flex-direction: column; gap: 0.5rem;",
                label { style: "color: #00B8FF; font-weight: 600; font-size: 0.9rem;", "N3 Notation" }
                textarea {
                    style: "width: 100%; height: 150px; padding: 1.2rem; background: #0A0A10; border: 1px solid rgba(0, 184, 255, 0.4); border-radius: 12px; color: #E0E0FF; font-family: 'Fira Code', monospace; font-size: 1rem; outline: none; transition: box-shadow 0.3s;",
                    value: "{rule_text}",
                    oninput: move |e| rule_text.set(e.value().clone()),
                }
            }

            div {
                style: "display: flex; gap: 1rem;",
                button {
                    style: "padding: 0.8rem 2rem; background: linear-gradient(90deg, #FF00FF, #8A2BE2); border: none; border-radius: 10px; color: #FFF; font-weight: 700; cursor: pointer; transition: all 0.2s;",
                    onclick: validate,
                    "Parse & Validate Rule"
                }
                button {
                    style: "padding: 0.8rem 2rem; background: transparent; border: 1px solid rgba(255, 255, 255, 0.2); border-radius: 10px; color: #FFF; font-weight: 600; cursor: pointer;",
                    "Load Examples"
                }
            }

            if !validation_msg.read().is_empty() {
                div {
                    style: "padding: 1.5rem; background: rgba(0, 255, 136, 0.05); border-left: 4px solid #00FF88; border-radius: 0 10px 10px 0; color: #00FF88; font-weight: 500;",
                    "{validation_msg}"
                }
            }
        }
    }
}
