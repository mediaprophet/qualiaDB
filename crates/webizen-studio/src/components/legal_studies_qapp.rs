use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `LegalStudiesQapp` component.
pub fn LegalStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:legalstudies".to_string(),
            title: "Legal Studies Explorer".to_string()
        }
    }
}
