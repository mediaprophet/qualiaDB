use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn FolkloreAndMythologyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:folkloreandmythology".to_string(),
            title: "Folklore And Mythology Explorer".to_string()
        }
    }
}
