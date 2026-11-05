use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `BalkanStudiesQapp` component.
pub fn BalkanStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:balkanstudies".to_string(),
            title: "Balkan Studies Explorer".to_string()
        }
    }
}
