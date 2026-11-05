use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `TraumaStudiesQapp` component.
pub fn TraumaStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:traumastudies".to_string(),
            title: "Trauma Studies Explorer".to_string()
        }
    }
}
