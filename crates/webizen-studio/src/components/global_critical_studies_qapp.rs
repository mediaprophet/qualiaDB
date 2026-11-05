use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Dioxus `GlobalCriticalStudiesQapp` component.
pub fn GlobalCriticalStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:globalcriticalstudies".to_string(),
            title: "Global Critical Studies Explorer".to_string()
        }
    }
}
