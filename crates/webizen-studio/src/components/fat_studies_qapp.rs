use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `FatStudiesQapp` component.
pub fn FatStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:fatstudies".to_string(),
            title: "Fat Studies Explorer".to_string()
        }
    }
}
