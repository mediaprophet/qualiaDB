#![allow(non_snake_case)]
use dioxus::prelude::*;
use crate::components::shoelace::*;

#[component]
pub fn PersonalOntologyBuilder() -> Element {
    rsx! {
        div { class: "personal-ontology-pane flex flex-col h-full",
            SlCard {
                div { slot: "header",
                    "Personal Ontology Axiom Builder"
                }
                div { class: "flex-col flex gap-4",
                    p { class: "text-sm text-gray-400",
                        "Define your semantic reality. Map natural language boundaries to strict RDF/SHACL rules."
                    }
                    div {
                        h4 { "Concept Name" }
                        SlInput { placeholder: "e.g., CloseFriend, Colleague" }
                    }
                    div {
                        h4 { "Natural Language Constraint (WordNet Context)" }
                        SlTextarea { placeholder: "A person I interact with outside of work context at least twice a month." }
                    }
                    div {
                        h4 { "SHACL Derivation Preview" }
                        div { class: "bg-black/50 p-2 rounded border border-[#b026ff]/30 font-mono text-xs text-[#b026ff]",
                            "q42:CloseFriend rdfs:subClassOf foaf:Person ;\n  sh:property [ ... ] ."
                        }
                    }
                }
                div { slot: "footer", class: "flex gap-2",
                    SlButton { variant: "primary", "Compile Axiom" }
                }
            }
        }
    }
}
