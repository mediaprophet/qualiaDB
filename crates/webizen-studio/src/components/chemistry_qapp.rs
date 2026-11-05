use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `ChemistryQapp` component.
pub fn ChemistryQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:chemistry".to_string(),
            title: "Chemistry Explorer".to_string()
        }
    }
}
