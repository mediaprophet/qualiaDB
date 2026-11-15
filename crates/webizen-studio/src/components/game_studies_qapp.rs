use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `GameStudiesQapp` component.
pub fn GameStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:gamestudies".to_string(),
            title: "Game Studies Explorer".to_string()
        }
    }
}
