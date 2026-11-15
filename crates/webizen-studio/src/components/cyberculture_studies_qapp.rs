use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `CybercultureStudiesQapp` component.
pub fn CybercultureStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:cyberculturestudies".to_string(),
            title: "Cyberculture Studies Explorer".to_string()
        }
    }
}
