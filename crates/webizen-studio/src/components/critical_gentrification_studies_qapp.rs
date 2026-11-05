use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `CriticalGentrificationStudiesQapp` component.
pub fn CriticalGentrificationStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:criticalgentrificationstudies".to_string(),
            title: "Critical Gentrification Studies Explorer".to_string()
        }
    }
}
