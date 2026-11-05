use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `LeisureStudiesQapp` component.
pub fn LeisureStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:leisurestudies".to_string(),
            title: "Leisure Studies Explorer".to_string()
        }
    }
}
