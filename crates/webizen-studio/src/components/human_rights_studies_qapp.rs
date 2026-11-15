use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `HumanRightsStudiesQapp` component.
pub fn HumanRightsStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:humanrightsstudies".to_string(),
            title: "Human Rights Studies Explorer".to_string()
        }
    }
}
