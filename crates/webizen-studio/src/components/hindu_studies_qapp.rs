use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Dioxus `HinduStudiesQapp` component.
pub fn HinduStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:hindustudies".to_string(),
            title: "Hindu Studies Explorer".to_string()
        }
    }
}
