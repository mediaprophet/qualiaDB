use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn RuralStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:ruralstudies".to_string(),
            title: "Rural Studies Explorer".to_string()
        }
    }
}
