use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `SoutheastAsianStudiesQapp` component.
pub fn SoutheastAsianStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:southeastasianstudies".to_string(),
            title: "Southeast Asian Studies Explorer".to_string()
        }
    }
}
