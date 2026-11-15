use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `CriticalTheoryQapp` component.
pub fn CriticalTheoryQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:criticaltheory".to_string(),
            title: "Critical Theory Explorer".to_string()
        }
    }
}
