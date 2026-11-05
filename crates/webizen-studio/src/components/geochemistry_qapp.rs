use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `GeochemistryQapp` component.
pub fn GeochemistryQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:geochemistry".to_string(),
            title: "Geochemistry Explorer".to_string()
        }
    }
}
