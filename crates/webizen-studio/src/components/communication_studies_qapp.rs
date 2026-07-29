use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn CommunicationStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:communicationstudies".to_string(),
            title: "Communication Studies Explorer".to_string()
        }
    }
}
