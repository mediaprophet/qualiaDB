use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn MetaphysicsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:metaphysics".to_string(),
            title: "Metaphysics Explorer".to_string()
        }
    }
}
