use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `ChemicalPhysicsQapp` component.
pub fn ChemicalPhysicsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:chemicalphysics".to_string(),
            title: "Chemical Physics Explorer".to_string()
        }
    }
}
