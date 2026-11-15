use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `MetaphysicsQapp` component.
pub fn MetaphysicsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:metaphysics".to_string(),
            title: "Metaphysics Explorer".to_string()
        }
    }
}
