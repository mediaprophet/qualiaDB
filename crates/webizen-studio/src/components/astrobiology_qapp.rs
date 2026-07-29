use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn AstrobiologyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:astrobiology".to_string(),
            title: "Astrobiology Explorer".to_string()
        }
    }
}
