use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `BodyStudiesQapp` component.
pub fn BodyStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:bodystudies".to_string(),
            title: "Body Studies Explorer".to_string()
        }
    }
}
