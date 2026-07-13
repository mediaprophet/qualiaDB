use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn PoetryAndPoeticsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:poetryandpoetics".to_string(),
            title: "Poetry And Poetics Explorer".to_string()
        }
    }
}
