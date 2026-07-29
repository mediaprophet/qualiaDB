use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn ChemicalPhysicsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:chemicalphysics".to_string(),
            title: "Chemical Physics Explorer".to_string()
        }
    }
}
