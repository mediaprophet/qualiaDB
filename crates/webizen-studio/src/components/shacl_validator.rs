use dioxus::prelude::*;

#[component]
pub fn ShaclValidator() -> Element {
    let mut progress = use_signal(|| 0);
    let mut is_validating = use_signal(|| false);

    let validate = move |_| {
        is_validating.set(true);
        progress.set(100);
    };

    rsx! {
        div {
            style: "flex: 1; padding: 2.5rem; background: #161B22; border-radius: 16px; border: 1px solid #30363D; color: #C9D1D9;",

            h2 {
                style: "font-family: 'Inter', sans-serif; font-size: 2rem; color: #58A6FF; margin-bottom: 0.5rem;",
                "SHACL Validator"
            }
            p { style: "color: #8B949E; margin-bottom: 2rem;", "Validate your knowledge graphs against SHACL shape constraints." }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 2rem;",

                div {
                    style: "display: flex; flex-direction: column; gap: 1rem;",
                    h3 { style: "margin: 0; color: #E6EDF3;", "Shape Definition" }
                    textarea {
                        style: "width: 100%; height: 200px; padding: 1rem; background: #0D1117; border: 1px solid #30363D; border-radius: 8px; color: #C9D1D9; font-family: 'JetBrains Mono', monospace; font-size: 0.9rem;",
                        value: "ex:PersonShape\n  a sh:NodeShape ;\n  sh:targetClass ex:Person ;\n  sh:property [\n    sh:path ex:ssn ;\n    sh:maxCount 1 ;\n    sh:datatype xsd:string ;\n  ] .",
                    }
                    button {
                        style: "padding: 0.8rem; background: #238636; border: 1px solid rgba(240,246,252,0.1); border-radius: 6px; color: #FFF; font-weight: bold; cursor: pointer; transition: background 0.2s;",
                        onclick: validate,
                        "Run Validation"
                    }
                }

                div {
                    style: "background: #0D1117; border: 1px solid #30363D; border-radius: 8px; padding: 1.5rem;",
                    h3 { style: "margin-top: 0; color: #E6EDF3;", "Validation Report" }

                    if *is_validating.read() {
                        // HONESTY: this panel is not wired to a real validator.
                        // It previously rendered a hardcoded green "Conforms:
                        // True" for ANY input — a fabricated verdict. The shape
                        // parser + shape registry that would evaluate the
                        // definition on the left do not exist yet, so we show an
                        // explicit not-implemented state rather than a pass we
                        // never computed.
                        div {
                            style: "padding: 1rem; background: rgba(210, 153, 34, 0.1); border-left: 4px solid #D29922; border-radius: 4px; color: #D29922;",
                            h4 { style: "margin: 0 0 0.5rem 0;", "Validation not available" }
                            p { style: "margin: 0; font-size: 0.9rem;", "This panel does not yet run real SHACL validation — the shape parser and registry that would evaluate the definition on the left aren't implemented. It will not report a conformance verdict it hasn't computed." }
                        }
                    } else {
                        div {
                            style: "display: flex; align-items: center; justify-content: center; height: 150px; color: #8B949E; font-style: italic;",
                            "Click 'Run Validation' to see results."
                        }
                    }
                }
            }
        }
    }
}
