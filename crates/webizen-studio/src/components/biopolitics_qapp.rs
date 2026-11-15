use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `BiopoliticsQapp` component.
pub fn BiopoliticsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:biopolitics".to_string(),
            title: "Biopolitics Explorer".to_string()
        }
    }
}
