use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `BioinformaticsQapp` component.
pub fn BioinformaticsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:bioinformatics".to_string(),
            title: "Bioinformatics Explorer".to_string()
        }
    }
}
