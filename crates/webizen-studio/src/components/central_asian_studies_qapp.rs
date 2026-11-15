use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `CentralAsianStudiesQapp` component.
pub fn CentralAsianStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:centralasianstudies".to_string(),
            title: "Central Asian Studies Explorer".to_string()
        }
    }
}
