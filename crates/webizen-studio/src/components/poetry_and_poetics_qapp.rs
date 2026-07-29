use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn PoetryAndPoeticsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:poetryandpoetics".to_string(),
            title: "Poetry And Poetics Explorer".to_string()
        }
    }
}
