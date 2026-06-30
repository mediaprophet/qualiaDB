use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn SocialPsychologyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:socialpsychology".to_string(),
            title: "Social Psychology Explorer".to_string()
        }
    }
}
