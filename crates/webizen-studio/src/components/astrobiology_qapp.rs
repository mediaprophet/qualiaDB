use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn AstrobiologyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:astrobiology".to_string(),
            title: "Astrobiology Explorer".to_string()
        }
    }
}
