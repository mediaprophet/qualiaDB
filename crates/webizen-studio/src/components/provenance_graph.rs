use dioxus::prelude::*;

use crate::components::qapp_engine::invoke_json;

#[component]
pub fn ProvenanceGraph() -> Element {
    let mut nodes = use_signal(Vec::<(String, String)>::new);
    let mut edges = use_signal(Vec::<(String, String, String)>::new);

    use_effect(move || {
        spawn(async move {
            let query = "SELECT ?s ?p ?o WHERE { ?s ?p ?o . FILTER(?p = <urn:prov:wasGeneratedBy> || ?p = <urn:prov:wasAssociatedWith>) }";
            let args = serde_json::json!({"query": query});
            if let Ok(res) = invoke_json("execute_sparql_query", args).await {
                if let Ok(triples) = serde_json::from_value::<Vec<(String, String, String)>>(res) {
                    let mut n = Vec::new();
                    for t in triples.iter() {
                        if !n.contains(&(t.0.clone(), "Entity".to_string())) {
                            n.push((t.0.clone(), "Entity".to_string()));
                        }
                        if !n.contains(&(t.2.clone(), "Target".to_string())) {
                            n.push((t.2.clone(), "Target".to_string()));
                        }
                    }
                    nodes.set(n);
                    edges.set(triples);
                }
            }
        });
    });

    rsx! {
        div { style: "height: 100vh; display: flex; flex-direction: column; background: #18181b; color: #fafafa; font-family: sans-serif;",
            div { style: "padding: 16px; border-bottom: 1px solid #27272a; display: flex; justify-content: space-between; align-items: center;",
                h2 { style: "margin: 0; font-size: 18px; color: #a1a1aa;", "W3C PROV-O Graph Explorer" }
                div { style: "display: flex; gap: 8px;",
                    button { style: "background: #27272a; border: none; color: white; padding: 6px 12px; border-radius: 4px;", "Zoom In" }
                    button { style: "background: #27272a; border: none; color: white; padding: 6px 12px; border-radius: 4px;", "Zoom Out" }
                }
            }
            div { style: "flex: 1; position: relative; overflow: hidden; background: radial-gradient(circle, #27272a 1px, transparent 1px); background-size: 20px 20px;",
                for (i, (id, typ)) in nodes.read().iter().enumerate() {
                    div {
                        key: "{id}",
                        style: "position: absolute; top: {20 + i * 15}%; left: {30 + i * 15}%; background: #0284c7; padding: 10px; border-radius: 8px; box-shadow: 0 4px 6px rgba(0,0,0,0.5); z-index: 10;",
                        div { style: "font-size: 12px; opacity: 0.8;", "{typ}" }
                        div { style: "font-weight: bold;", "{id}" }
                    }
                }
                if nodes.read().is_empty() {
                    div { style: "position: absolute; top: 50%; left: 50%; transform: translate(-50%, -50%); color: #71717a;", "No provenance data found." }
                }
            }
        }
    }
}
