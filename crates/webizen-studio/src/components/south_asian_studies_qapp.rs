use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `SouthAsianStudiesQapp` component.
pub fn SouthAsianStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:southasianstudies".to_string(),
            title: "South Asian Studies Explorer".to_string()
        }
    }
}
