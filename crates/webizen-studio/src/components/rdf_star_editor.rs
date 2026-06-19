use dioxus::prelude::*;

#[component]
pub fn RdfStarEditor() -> Element {
    let mut expanded = use_signal(|| false);

    rsx! {
        div {
            style: "flex: 1; padding: 2rem; background: rgba(15, 20, 25, 0.8); backdrop-filter: blur(20px); border-radius: 16px; border: 1px solid rgba(255, 255, 255, 0.05); color: #E8E8E8;",

            h2 {
                style: "font-family: 'Inter', sans-serif; font-size: 2.2rem; margin-bottom: 0.5rem; background: linear-gradient(to right, #F5A623, #F83600); -webkit-background-clip: text; -webkit-text-fill-color: transparent;",
                "RDF-Star Editor"
            }
            p { style: "color: #999; margin-bottom: 2rem;", "Edit and visualize nested statements (statements about statements)." }

            div {
                style: "background: rgba(0,0,0,0.3); border-radius: 12px; padding: 1.5rem; border: 1px solid rgba(245, 166, 35, 0.2);",

                div {
                    style: "display: flex; align-items: center; gap: 1rem; padding-bottom: 1rem; border-bottom: 1px solid rgba(255,255,255,0.1); margin-bottom: 1rem;",
                    div { style: "padding: 0.5rem 1rem; background: rgba(245, 166, 35, 0.1); border-radius: 6px; color: #F5A623; font-family: monospace;", "did:q42:alice" }
                    span { style: "color: #888;", "claims" }
                    div {
                        style: "cursor: pointer; padding: 0.5rem 1rem; background: rgba(255,255,255,0.05); border-radius: 6px; border: 1px solid rgba(255,255,255,0.1); transition: all 0.2s;",
                        onclick: move |_| { let v = *expanded.peek(); expanded.set(!v); },
                        if *expanded.read() { "▼ [Nested Statement]" } else { "▶ [Nested Statement]" }
                    }
                }

                if *expanded.read() {
                    div {
                        style: "margin-left: 2rem; padding: 1.5rem; background: rgba(248, 54, 0, 0.05); border-left: 2px solid #F83600; border-radius: 0 8px 8px 0; animation: fadeIn 0.3s ease;",
                        div {
                            style: "display: flex; gap: 1rem; align-items: center;",
                            input { style: "flex: 1; padding: 0.8rem; background: #000; border: 1px solid #333; border-radius: 6px; color: #FFF;", value: "did:q42:bob" }
                            input { style: "flex: 1; padding: 0.8rem; background: #000; border: 1px solid #333; border-radius: 6px; color: #FFF;", value: "foaf:age" }
                            input { style: "flex: 1; padding: 0.8rem; background: #000; border: 1px solid #333; border-radius: 6px; color: #FFF;", value: "42" }
                        }
                    }
                }

                div {
                    style: "margin-top: 2rem; display: flex; gap: 1rem;",
                    button {
                        style: "padding: 0.8rem 1.5rem; background: #F5A623; border: none; border-radius: 6px; color: #000; font-weight: bold; cursor: pointer;",
                        "Commit to Graph"
                    }
                }
            }
        }
    }
}
