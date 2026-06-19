use dioxus::prelude::*;

#[component]
pub fn ZkProofStudio() -> Element {
    let mut is_generating = use_signal(|| false);

    let generate_proof = move |_| {
        is_generating.set(true);
    };

    rsx! {
        div {
            style: "flex: 1; padding: 2rem; background: radial-gradient(circle at top right, rgba(0, 255, 100, 0.1), transparent 400px), #111; border-radius: 16px; color: #FFF;",

            h2 {
                style: "font-size: 2.2rem; color: #00FF88; margin-bottom: 1rem;",
                "Zero-Knowledge Proof Studio"
            }
            p { style: "color: #999; margin-bottom: 2rem;", "Generate and verify succinct non-interactive arguments of knowledge (SNARKs/STARKs) for your claims without revealing underlying data." }

            div {
                style: "display: grid; grid-template-columns: 2fr 1fr; gap: 2rem;",

                div {
                    style: "background: rgba(255,255,255,0.02); padding: 1.5rem; border-radius: 12px; border: 1px solid rgba(255,255,255,0.05);",
                    h3 { style: "margin-top: 0; color: #EEE;", "Proof Circuit: Age Verification" }

                    div {
                        style: "margin: 1.5rem 0; font-family: 'Fira Code', monospace; color: #A0A0B0; background: #000; padding: 1rem; border-radius: 8px;",
                        "// Proves: age >= 18\n// Public Input: current_year\n// Private Input: birth_year\n\nassert(current_year - birth_year >= 18);"
                    }

                    button {
                        style: "padding: 0.8rem 1.5rem; background: #00FF88; border: none; border-radius: 8px; color: #000; font-weight: bold; cursor: pointer; transition: transform 0.1s;",
                        onclick: generate_proof,
                        if *is_generating.read() { "Generating SNARK..." } else { "Generate Proof" }
                    }
                }

                div {
                    style: "background: rgba(0,255,136,0.05); padding: 1.5rem; border-radius: 12px; border: 1px dashed rgba(0,255,136,0.3); display: flex; flex-direction: column; align-items: center; justify-content: center; text-align: center;",
                    if *is_generating.read() {
                        div {
                            div { style: "width: 40px; height: 40px; border: 4px solid rgba(0,255,136,0.3); border-top-color: #00FF88; border-radius: 50%; animation: spin 1s linear infinite; margin: 0 auto 1rem auto;" }
                            div { style: "color: #00FF88;", "Synthesizing circuit...\nComputing witness..." }
                        }
                    } else {
                        div {
                            div { style: "font-size: 3rem; margin-bottom: 1rem;", "📄" }
                            div { style: "color: #888;", "No proof generated yet. Run a circuit to generate a verifiable proof artifact." }
                        }
                    }
                }
            }
        }
    }
}
