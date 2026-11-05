use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Dioxus `DisasterStudiesQapp` component.
pub fn DisasterStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:disasterstudies".to_string(),
            title: "Disaster Studies Explorer".to_string()
        }
    }
}
