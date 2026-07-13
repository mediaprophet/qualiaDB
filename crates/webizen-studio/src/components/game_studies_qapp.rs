use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn GameStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:gamestudies".to_string(),
            title: "Game Studies Explorer".to_string()
        }
    }
}
