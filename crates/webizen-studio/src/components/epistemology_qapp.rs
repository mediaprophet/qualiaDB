use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn EpistemologyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:epistemology".to_string(),
            title: "Epistemology Explorer".to_string()
        }
    }
}
