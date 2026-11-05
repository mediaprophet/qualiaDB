use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Dioxus `CaribbeanStudiesQapp` component.
pub fn CaribbeanStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:caribbeanstudies".to_string(),
            title: "Caribbean Studies Explorer".to_string()
        }
    }
}
