use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `FanStudiesQapp` component.
pub fn FanStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:fanstudies".to_string(),
            title: "Fan Studies Explorer".to_string()
        }
    }
}
