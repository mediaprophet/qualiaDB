use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `WhitenessStudiesQapp` component.
pub fn WhitenessStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:whitenessstudies".to_string(),
            title: "Whiteness Studies Explorer".to_string()
        }
    }
}
