use dioxus::prelude::*;

#[component]
pub fn SparqlExplorer() -> Element {
    let mut query = use_signal(|| {
        "SELECT ?subject ?predicate ?object\nWHERE {\n  ?subject ?predicate ?object\n}\nLIMIT 10"
            .to_string()
    });
    let mut results = use_signal(|| Vec::<(String, String, String)>::new());
    let mut is_loading = use_signal(|| false);

    let run_query = move |_| {
        is_loading.set(true);
        // Mock query execution
        let mock_data = vec![
            (
                "did:q42:alice".to_string(),
                "foaf:knows".to_string(),
                "did:q42:bob".to_string(),
            ),
            (
                "did:q42:bob".to_string(),
                "foaf:name".to_string(),
                "\"Bob\"".to_string(),
            ),
        ];
        results.set(mock_data);
        is_loading.set(false);
    };

    rsx! {
        div {
            style: "flex: 1; display: flex; flex-direction: column; gap: 1rem; padding: 2rem; background: rgba(30, 30, 40, 0.6); backdrop-filter: blur(12px); border: 1px solid rgba(255, 255, 255, 0.1); border-radius: 16px; color: var(--qualia-text);",
            h2 {
                style: "margin: 0; font-family: 'Inter', sans-serif; font-size: 1.8rem; background: linear-gradient(90deg, #00FF88, #00B8FF); -webkit-background-clip: text; -webkit-text-fill-color: transparent;",
                "SPARQL Explorer"
            }
            p { style: "color: #A0A0B0; margin: 0 0 1rem 0;", "Query the decentralized knowledge graph." }

            textarea {
                style: "width: 100%; height: 200px; padding: 1rem; background: rgba(0, 0, 0, 0.4); border: 1px solid rgba(0, 255, 136, 0.3); border-radius: 8px; color: #E0E0E0; font-family: 'JetBrains Mono', monospace; resize: vertical; transition: all 0.3s ease;",
                value: "{query}",
                oninput: move |e| query.set(e.value().clone()),
            }

            div {
                style: "display: flex; justify-content: flex-end;",
                button {
                    style: "padding: 0.8rem 1.5rem; background: linear-gradient(45deg, #00FF88, #00B8FF); border: none; border-radius: 8px; color: #000; font-weight: bold; cursor: pointer; transition: transform 0.2s, box-shadow 0.2s; box-shadow: 0 4px 12px rgba(0, 255, 136, 0.2);",
                    onclick: run_query,
                    if *is_loading.read() {
                        "Executing..."
                    } else {
                        "Run Query"
                    }
                }
            }

            if !results.read().is_empty() {
                div {
                    style: "margin-top: 2rem; background: rgba(0, 0, 0, 0.2); border-radius: 8px; overflow: hidden; border: 1px solid rgba(255, 255, 255, 0.05);",
                    table {
                        style: "width: 100%; border-collapse: collapse; text-align: left;",
                        thead {
                            style: "background: rgba(255, 255, 255, 0.05);",
                            tr {
                                th { style: "padding: 1rem; border-bottom: 1px solid rgba(255, 255, 255, 0.1); color: #00B8FF;", "Subject" }
                                th { style: "padding: 1rem; border-bottom: 1px solid rgba(255, 255, 255, 0.1); color: #00FF88;", "Predicate" }
                                th { style: "padding: 1rem; border-bottom: 1px solid rgba(255, 255, 255, 0.1); color: #FF00FF;", "Object" }
                            }
                        }
                        tbody {
                            for (s, p, o) in results.read().iter() {
                                tr {
                                    style: "border-bottom: 1px solid rgba(255, 255, 255, 0.05); transition: background 0.2s;",
                                    td { style: "padding: 1rem; font-family: 'JetBrains Mono', monospace; font-size: 0.9em;", "{s}" }
                                    td { style: "padding: 1rem; font-family: 'JetBrains Mono', monospace; font-size: 0.9em;", "{p}" }
                                    td { style: "padding: 1rem; font-family: 'JetBrains Mono', monospace; font-size: 0.9em;", "{o}" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
