use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `NeuroscienceQapp` component.
pub fn NeuroscienceQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:neuroscience".to_string(),
            title: "Neuroscience Explorer".to_string()
        }
    }
}
