use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn SculptureQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:sculpture".to_string(),
            title: "Sculpture Explorer".to_string()
        }
    }
}
