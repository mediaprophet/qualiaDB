use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `AppalachianStudiesQapp` component.
pub fn AppalachianStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:appalachianstudies".to_string(),
            title: "Appalachian Studies Explorer".to_string()
        }
    }
}
