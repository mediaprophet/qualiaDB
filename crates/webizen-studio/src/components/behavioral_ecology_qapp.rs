use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `BehavioralEcologyQapp` component.
pub fn BehavioralEcologyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:behavioralecology".to_string(),
            title: "Behavioral Ecology Explorer".to_string()
        }
    }
}
