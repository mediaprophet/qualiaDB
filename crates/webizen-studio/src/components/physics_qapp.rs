use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn PhysicsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:physics".to_string(),
            title: "Physics Explorer".to_string()
        }
    }
}
