use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn SocialPsychologyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:socialpsychology".to_string(),
            title: "Social Psychology Explorer".to_string()
        }
    }
}
