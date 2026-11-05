use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Dioxus `CommunicationStudiesQapp` component.
pub fn CommunicationStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:communicationstudies".to_string(),
            title: "Communication Studies Explorer".to_string()
        }
    }
}
