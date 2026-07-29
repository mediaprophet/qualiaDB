use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn PeaceAndConflictStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:peaceandconflictstudies".to_string(),
            title: "Peace And Conflict Studies Explorer".to_string()
        }
    }
}
